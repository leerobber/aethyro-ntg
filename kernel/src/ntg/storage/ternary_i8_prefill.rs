//! Large-batch (prefill) path for [`PackedTernaryMatrix`].
//!
//! The decode kernels walk one weight row at a time and unpack it again for every group of four
//! batch rows, which is the right shape for a batch of one but leaves about half of the VNNI
//! throughput on the table once the batch is large. This path keeps a second copy of the weights in
//! a *panel* layout and runs a register-tiled micro-kernel over it:
//!
//! * **Panels.** 16 consecutive output rows form a panel. Each 64-byte block of a panel holds 16
//!   consecutive `k` for those 16 rows: byte `4*r + s` carries, in its four 2-bit fields `i`, the
//!   codes (`weight + 1`) of row `r` at `k = 16*g + 4*i + s`. Shifting and masking a block therefore
//!   gives four ready-to-use `u8` vectors, one per `k`-quad, with row `r` in 32-bit lane `r`.
//! * **Micro-kernel.** One accumulator vector per batch row (up to 24) holds that row's 16 outputs.
//!   For each `k`-quad, `vpdpbusd` multiplies the panel vector by the batch row's four activations
//!   (a memory-broadcast operand) into the accumulator. There is no horizontal reduction, and the
//!   unpack cost is shared by every batch row in the tile.
//! * **Activation tiles.** Activations are re-laid out once per call as `[k-quad][batch row]` so
//!   the broadcasts of a tile read one contiguous stream.
//!
//! Costs: the panel copy is another 2 bits per weight (the row layout stays for decode), built on
//! demand by [`PackedTernaryMatrix::prepare_prefill`]. AVX-512 VNNI only; other kernels, and
//! batches below [`PREFILL_MIN_BATCH`], use the row layout. Results are exact `i32`, identical to
//! the row-layout kernels.

use super::spin_pool::SpinPool;
use super::ternary_i8_gemm::{Int8Activations, PackedTernaryMatrix, TernaryI8Kernel, GROUP};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Smallest batch routed to the panel kernel when panels exist.
pub const PREFILL_MIN_BATCH: usize = 4;
/// Output rows per panel.
const PANEL: usize = 16;
/// Largest batch tile (accumulator registers).
const MAX_NR: usize = 24;

/// A group of consecutive batch rows processed together.
struct Tile {
    b0: usize,
    nr: usize,
    /// Offset (in `u32`s) of this tile's activations in the shared buffer.
    off: usize,
}

impl PackedTernaryMatrix {
    /// Build the panel copy used by the large-batch path. Idempotent. Costs another
    /// `rows x cols / 4` bytes (rows rounded up to 16). No effect off x86-64.
    pub fn prepare_prefill(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            if self.panels.is_some() {
                return;
            }
            let blocks = self.k_padded / PANEL;
            let n_panels = self.rows.div_ceil(PANEL);
            // Code 1 (= weight 0) everywhere: padded rows and columns contribute nothing.
            let mut panels = vec![0b0101_0101u8; n_panels * blocks * 64];
            let mut codes = vec![1u8; self.k_padded];
            for row in 0..self.rows {
                let bytes = &self.data[row * self.bytes_per_row..(row + 1) * self.bytes_per_row];
                for g in 0..self.k_padded / GROUP {
                    for j in 0..64 {
                        let b = bytes[g * 64 + j];
                        for i in 0..4 {
                            codes[g * GROUP + 64 * i + j] = (b >> (2 * i)) & 0b11;
                        }
                    }
                }
                let (p, r) = (row / PANEL, row % PANEL);
                for g in 0..blocks {
                    let base = (p * blocks + g) * 64 + 4 * r;
                    for s in 0..4 {
                        let mut byte = 0u8;
                        for i in 0..4 {
                            byte |= codes[PANEL * g + 4 * i + s] << (2 * i);
                        }
                        panels[base + s] = byte;
                    }
                }
            }
            self.panels = Some(panels);
        }
    }

    /// Whether the panel copy exists.
    pub fn has_prefill(&self) -> bool {
        self.panels.is_some()
    }

    /// Bytes held by the panel copy (0 until [`Self::prepare_prefill`]).
    pub fn prefill_bytes(&self) -> usize {
        self.panels.as_ref().map_or(0, Vec::len)
    }

    pub(super) fn prefill_applies(&self, batch: usize, kernel: TernaryI8Kernel) -> bool {
        self.panels.is_some() && kernel == TernaryI8Kernel::Avx512Vnni && batch >= PREFILL_MIN_BATCH
    }

    /// Exact product via the panel kernel. Caller has validated shapes and kernel availability.
    #[cfg(target_arch = "x86_64")]
    pub(super) fn gemm_prefill(&self, x: &Int8Activations, out: &mut [i32], pool: Option<&SpinPool>) {
        self.gemm_prefill_impl(x, out.as_mut_ptr() as *mut u8, false, pool);
    }

    /// `i32` accumulators (`f32_out == false`) or dequantised `f32` (`true`; each batch row scaled by
    /// its activation scale times the weight scale) written to `out`, which holds `batch x rows` of that type.
    #[cfg(target_arch = "x86_64")]
    fn gemm_prefill_impl(&self, x: &Int8Activations, out: *mut u8, f32_out: bool, pool: Option<&SpinPool>) {
        let panels = self.panels.as_ref().expect("checked by prefill_applies");
        let n_panels = self.rows.div_ceil(PANEL);
        let blocks = self.k_padded / PANEL;
        let batch = x.batch();

        // Tile the batch greedily into 24, 16, 8, 4, 2, 1 and re-lay the activations per tile.
        // A tile of `nr` rows starts at dword `b0 * k_padded / 4` of `xt`.
        let quads = self.k_padded / 4;
        let mut tiles = Vec::new();
        let mut b0 = 0;
        while b0 < batch {
            let nr = [MAX_NR, 16, 8, 4, 2, 1].into_iter().find(|&n| n <= batch - b0).unwrap();
            tiles.push(Tile { b0, nr, off: b0 * quads });
            b0 += nr;
        }
        let mut xt = vec![0u32; batch * quads];
        let xp = x.padded();
        let fill = |tile: &Tile, dst: &mut [u32]| {
            for t in 0..tile.nr {
                let row = &xp[(tile.b0 + t) * self.k_padded..(tile.b0 + t + 1) * self.k_padded];
                for kq in 0..quads {
                    let q = &row[4 * kq..4 * kq + 4];
                    dst[kq * tile.nr + t] = u32::from_le_bytes([q[0] as u8, q[1] as u8, q[2] as u8, q[3] as u8]);
                }
            }
        };
        match pool {
            Some(pool) if pool.threads() > 1 && tiles.len() > 1 => {
                let dst = SharedWords(xt.as_mut_ptr());
                let next = AtomicUsize::new(0);
                pool.run(|_| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= tiles.len() {
                        break;
                    }
                    let tile = &tiles[i];
                    // SAFETY: tiles cover disjoint ranges of `xt`, each claimed once.
                    let d = unsafe { std::slice::from_raw_parts_mut(dst.get().add(tile.off), tile.nr * quads) };
                    fill(tile, d);
                });
            }
            _ => {
                for tile in &tiles {
                    fill(tile, &mut xt[tile.off..tile.off + tile.nr * quads]);
                }
            }
        }

        let out_ptr = SharedOut(out);
        let row_scale: Vec<f32> = if f32_out { x.scales().iter().map(|s| s * self.scale()).collect() } else { Vec::new() };
        let run = |lo: usize, hi: usize| {
            for p in lo..hi {
                let panel = &panels[p * blocks * 64..(p + 1) * blocks * 64];
                let valid = (self.rows - p * PANEL).min(PANEL);
                for tile in &tiles {
                    // SAFETY: AVX-512 VNNI was checked by the caller (`prefill_applies`); `panel`
                    // holds `blocks` 64-byte blocks, `xt[tile.off..]` holds `k_padded / 4 * nr`
                    // dwords, and the writes land in rows `b0..b0+nr`, columns `p*16..p*16+valid`
                    // of `out`, which no other panel touches.
                    unsafe {
                        let xs = xt.as_ptr().add(tile.off);
                        let o = out_ptr.get();
                        let sums = x.row_sums();
                        match tile.nr {
                            24 => micro::<24>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                            16 => micro::<16>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                            8 => micro::<8>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                            4 => micro::<4>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                            2 => micro::<2>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                            _ => micro::<1>(panel, blocks, xs, o, self.rows, p * PANEL, valid, tile.b0, sums, &row_scale),
                        }
                    }
                }
            }
        };

        match pool {
            Some(pool) if pool.threads() > 1 && n_panels >= pool.threads() => {
                let chunk = n_panels.div_ceil(pool.threads() * 4).max(1);
                let chunks = n_panels.div_ceil(chunk);
                let next = AtomicUsize::new(0);
                pool.run(|_| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= chunks {
                        break;
                    }
                    run(i * chunk, ((i + 1) * chunk).min(n_panels));
                });
            }
            _ => run(0, n_panels),
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub(super) fn gemm_prefill(&self, _x: &Int8Activations, _out: &mut [i32], _pool: Option<&SpinPool>) {
        unreachable!("panels are only built on x86-64");
    }

    /// Large-batch forward straight to dequantised `f32` (`out[b][n] = acc * x_scale[b] * weight
    /// scale`), skipping the `i32` buffer. Returns `false`, writing nothing, when the panel kernel
    /// does not apply (no panel copy, batch below [`PREFILL_MIN_BATCH`], not AVX-512 VNNI, or a
    /// shape mismatch); the caller then takes the ordinary path.
    pub fn forward_prefill_f32(&self, x: &Int8Activations, out: &mut [f32], pool: Option<&SpinPool>) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            if x.cols() != self.cols
                || out.len() != x.batch() * self.rows
                || !self.prefill_applies(x.batch(), TernaryI8Kernel::best_available())
            {
                false
            } else {
                self.gemm_prefill_impl(x, out.as_mut_ptr() as *mut u8, true, pool);
                true
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (x, out, pool);
            false
        }
    }
}

struct SharedWords(*mut u32);
// SAFETY: threads fill disjoint tiles; see `gemm_prefill`.
unsafe impl Sync for SharedWords {}
impl SharedWords {
    fn get(&self) -> *mut u32 {
        self.0
    }
}

struct SharedOut(*mut u8);
// SAFETY: threads write disjoint panels (column ranges) of the output; see `gemm_prefill`.
unsafe impl Sync for SharedOut {}
impl SharedOut {
    fn get(&self) -> *mut u8 {
        self.0
    }
}

/// One panel x one batch tile of `NR` rows.
///
/// # Safety
/// CPU must support AVX-512 F/BW/VNNI. `panel` is `blocks` 64-byte blocks, `xt` is
/// `blocks * 4 * NR` dwords laid out `[k-quad][batch row]`, `sums[b0..b0+NR]` exist, and `out`
/// has room for rows `b0..b0+NR` of `rows` values (`i32`, or `f32` when `row_scale` is non-empty,
/// holding one scale per batch row); columns `n0..n0+valid` are written.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
#[allow(clippy::too_many_arguments)]
unsafe fn micro<const NR: usize>(
    panel: &[u8],
    blocks: usize,
    xt: *const u32,
    out: *mut u8,
    rows: usize,
    n0: usize,
    valid: usize,
    b0: usize,
    sums: &[i32],
    row_scale: &[f32],
) {
    use std::arch::x86_64::*;
    let mask = _mm512_set1_epi8(0b11);
    let mut acc = [_mm512_setzero_si512(); NR];
    let base = panel.as_ptr();
    for g in 0..blocks {
        let blk = _mm512_loadu_si512(base.add(g * 64) as *const _);
        let v = [
            _mm512_and_si512(blk, mask),
            _mm512_and_si512(_mm512_srli_epi16(blk, 2), mask),
            _mm512_and_si512(_mm512_srli_epi16(blk, 4), mask),
            _mm512_and_si512(_mm512_srli_epi16(blk, 6), mask),
        ];
        let xs = xt.add(4 * g * NR);
        for (q, vq) in v.iter().enumerate() {
            let xq = xs.add(q * NR);
            for (t, a) in acc.iter_mut().enumerate() {
                *a = _mm512_dpbusd_epi32(*a, *vq, _mm512_set1_epi32(*xq.add(t) as i32));
            }
        }
    }
    let keep: __mmask16 = if valid >= PANEL { 0xFFFF } else { (1u16 << valid) - 1 };
    for (t, a) in acc.iter().enumerate() {
        let b = b0 + t;
        let r = _mm512_sub_epi32(*a, _mm512_set1_epi32(sums[b]));
        if row_scale.is_empty() {
            _mm512_mask_storeu_epi32((out as *mut i32).add(b * rows + n0), keep, r);
        } else {
            let f = _mm512_mul_ps(_mm512_cvtepi32_ps(r), _mm512_set1_ps(row_scale[b]));
            _mm512_mask_storeu_ps((out as *mut f32).add(b * rows + n0), keep, f);
        }
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn ternary(&mut self) -> i8 {
            (self.next() % 3) as i8 - 1
        }
        fn any(&mut self) -> i8 {
            (self.next() >> 56) as u8 as i8
        }
    }

    fn naive(w: &[i8], x: &[i8], rows: usize, cols: usize, batch: usize) -> Vec<i32> {
        let mut out = vec![0; batch * rows];
        for b in 0..batch {
            for n in 0..rows {
                out[b * rows + n] = (0..cols).map(|k| w[n * cols + k] as i32 * x[b * cols + k] as i32).sum();
            }
        }
        out
    }

    #[test]
    fn panel_kernel_matches_naive_and_the_row_layout_for_awkward_shapes() {
        if !TernaryI8Kernel::Avx512Vnni.is_available() {
            return;
        }
        let mut rng = Rng(0xFACE);
        let pools: Vec<SpinPool> = [2, 3, 4].iter().map(|&t| SpinPool::new(t).unwrap()).collect();
        let shapes = [
            (200, 300, 4),
            (203, 513, 7),
            (17, 256, 24),
            (1000, 700, 53),
            (64, 2560, 100),
            (640, 128, 25),
            (1, 3, 5),
            (16, 16, 31),
            (33, 257, 49),
        ];
        for &(rows, cols, batch) in &shapes {
            let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
            let mut x: Vec<i8> = (0..batch * cols).map(|_| rng.any()).collect();
            x[0] = -128; // most negative activation
            let want = naive(&w, &x, rows, cols, batch);
            let acts = Int8Activations::from_i8(&x, batch, cols).unwrap();
            let plain = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap();
            let mut m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap();
            assert!(!m.has_prefill());
            m.prepare_prefill();
            m.prepare_prefill(); // idempotent
            assert!(m.has_prefill() && m.prefill_bytes() >= rows * cols / 4);
            let k = TernaryI8Kernel::Avx512Vnni;

            let mut row_layout = vec![0; batch * rows];
            plain.gemm_i8(&acts, &mut row_layout, k).unwrap();
            assert_eq!(row_layout, want, "row layout {rows}x{cols} B={batch}");

            let mut got = vec![0; batch * rows];
            m.gemm_i8(&acts, &mut got, k).unwrap();
            assert_eq!(got, want, "panel kernel {rows}x{cols} B={batch}");
            for pool in &pools {
                let mut got = vec![0; batch * rows];
                m.gemm_i8_spin(&acts, &mut got, k, pool).unwrap();
                assert_eq!(got, want, "panel kernel, {} threads, {rows}x{cols} B={batch}", pool.threads());
            }
        }
    }

    #[test]
    fn f32_output_equals_dequantised_i32_output() {
        if !TernaryI8Kernel::Avx512Vnni.is_available() {
            return;
        }
        let mut rng = Rng(99);
        let pool = SpinPool::new(3).unwrap();
        for &(rows, cols, batch) in &[(70usize, 300usize, 9usize), (33, 257, 49), (640, 512, 100)] {
            let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
            let xf: Vec<f32> = (0..batch * cols).map(|_| rng.any() as f32 / 7.0).collect();
            let acts = Int8Activations::quantize_f32_bitnet(&xf, batch, cols).unwrap();
            let mut m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap().with_scale(0.37);
            let mut acc = vec![0; batch * rows];
            m.gemm_i8(&acts, &mut acc, TernaryI8Kernel::Avx512Vnni).unwrap();
            let mut f = vec![0f32; batch * rows];
            assert!(!m.forward_prefill_f32(&acts, &mut f, None), "no panel copy yet");
            m.prepare_prefill();
            for p in [None, Some(&pool)] {
                assert!(m.forward_prefill_f32(&acts, &mut f, p));
                for b in 0..batch {
                    let s = acts.scales()[b] * 0.37;
                    for n in 0..rows {
                        assert_eq!(f[b * rows + n], acc[b * rows + n] as f32 * s, "({b},{n}) {rows}x{cols}");
                    }
                }
            }
            assert!(!m.forward_prefill_f32(&acts, &mut f[..1], None), "wrong output length");
        }
    }

    #[test]
    fn small_batches_and_other_kernels_still_use_the_row_layout() {
        if !TernaryI8Kernel::Avx512Vnni.is_available() {
            return;
        }
        let mut rng = Rng(7);
        let (rows, cols) = (50, 300);
        let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
        let mut m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap();
        m.prepare_prefill();
        assert!(!m.prefill_applies(PREFILL_MIN_BATCH - 1, TernaryI8Kernel::Avx512Vnni));
        assert!(m.prefill_applies(PREFILL_MIN_BATCH, TernaryI8Kernel::Avx512Vnni));
        assert!(!m.prefill_applies(64, TernaryI8Kernel::Avx2));
        for kernel in TernaryI8Kernel::all_available() {
            for batch in [1usize, 3, 4, 9] {
                let x: Vec<i8> = (0..batch * cols).map(|_| rng.any()).collect();
                let acts = Int8Activations::from_i8(&x, batch, cols).unwrap();
                let mut got = vec![0; batch * rows];
                m.gemm_i8(&acts, &mut got, kernel).unwrap();
                assert_eq!(got, naive(&w, &x, rows, cols, batch), "{} B={batch}", kernel.name());
            }
        }
    }
}
