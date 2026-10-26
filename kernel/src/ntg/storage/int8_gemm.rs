//! Dense int8-weight x int8-activation GEMM for the one matrix that is not ternary: the
//! output (`lm_head`) projection.
//!
//! BitNet b1.58 2B-4T's output matrix is 128256 x 2560 in bf16, 0.66 GB read per decoded
//! token, more than the whole ternary body (0.52 GB). Quantising it per output row to int8
//! halves that traffic.
//!
//! Weights are symmetric per row (`scale = max|w| / 127`). Activations are the same
//! [`Int8Activations`] the ternary kernel uses. The integer product is exact `i32`; AVX-512
//! VNNI computes `sum((x + 128) * w)` with `x + 128` as the unsigned operand and removes
//! `128 * sum(w)` (precomputed per row) afterwards.

use super::spin_pool::SpinPool;
use super::ternary_i8_gemm::{Int8Activations, TernaryI8Kernel, GROUP};
use crate::ntg::error::NtgError;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Batch rows processed per weight pass; keeps the activation tile in L2 for large batches.
const TILE: usize = 32;

/// Row-major int8 weights, rows zero-padded to a multiple of 256 columns.
#[derive(Clone, Debug)]
pub struct Int8Matrix {
    rows: usize,
    cols: usize,
    k_padded: usize,
    data: Vec<i8>,
    row_sums: Vec<i32>,
    scales: Vec<f32>,
}

fn quantize_row(src: impl Iterator<Item = f32> + Clone, dst: &mut [i8]) -> (f32, i32) {
    let max = src.clone().fold(0.0f32, |m, v| m.max(v.abs()));
    let scale = if max == 0.0 { 1.0 } else { max / 127.0 };
    let mut sum = 0i32;
    for (d, v) in dst.iter_mut().zip(src) {
        let q = (v / scale).round().clamp(-127.0, 127.0) as i8;
        *d = q;
        sum += q as i32;
    }
    (scale, sum)
}

impl Int8Matrix {
    fn alloc(rows: usize, cols: usize) -> Result<(usize, Vec<i8>), NtgError> {
        if rows == 0 || cols == 0 {
            return Err(NtgError::InvalidInput("matrix dimensions must be non-zero".into()));
        }
        let k_padded = cols.div_ceil(GROUP) * GROUP;
        let len = rows
            .checked_mul(k_padded)
            .ok_or_else(|| NtgError::InvalidInput("matrix size overflows usize".into()))?;
        Ok((k_padded, vec![0i8; len]))
    }

    /// Quantise `rows x cols` f32 weights per row.
    pub fn from_f32(w: &[f32], rows: usize, cols: usize) -> Result<Self, NtgError> {
        if w.len() != rows.saturating_mul(cols) {
            return Err(NtgError::ShapeMismatch { expected: rows.saturating_mul(cols), got: w.len() });
        }
        let (k_padded, mut data) = Self::alloc(rows, cols)?;
        let mut scales = vec![1.0; rows];
        let mut row_sums = vec![0; rows];
        for r in 0..rows {
            let src = w[r * cols..(r + 1) * cols].iter().copied();
            let (s, sum) = quantize_row(src, &mut data[r * k_padded..r * k_padded + cols]);
            scales[r] = s;
            row_sums[r] = sum;
        }
        Ok(Self { rows, cols, k_padded, data, row_sums, scales })
    }

    /// Quantise `rows x cols` bf16 weights (raw `u16` bit patterns) per row, without
    /// materialising an f32 copy of the whole matrix.
    pub fn from_bf16_bits(w: &[u16], rows: usize, cols: usize) -> Result<Self, NtgError> {
        if w.len() != rows.saturating_mul(cols) {
            return Err(NtgError::ShapeMismatch { expected: rows.saturating_mul(cols), got: w.len() });
        }
        let (k_padded, mut data) = Self::alloc(rows, cols)?;
        let mut scales = vec![1.0; rows];
        let mut row_sums = vec![0; rows];
        for r in 0..rows {
            let src = w[r * cols..(r + 1) * cols].iter().map(|&b| f32::from_bits((b as u32) << 16));
            let (s, sum) = quantize_row(src, &mut data[r * k_padded..r * k_padded + cols]);
            scales[r] = s;
            row_sums[r] = sum;
        }
        Ok(Self { rows, cols, k_padded, data, row_sums, scales })
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Per-row dequantisation scales.
    pub fn scales(&self) -> &[f32] {
        &self.scales
    }

    /// Bytes of weight storage (padding included).
    pub fn bytes(&self) -> usize {
        self.data.len()
    }

    fn validate(&self, x: &Int8Activations, out_len: usize, kernel: TernaryI8Kernel) -> Result<(), NtgError> {
        if x.cols() != self.cols {
            return Err(NtgError::ShapeMismatch { expected: self.cols, got: x.cols() });
        }
        if out_len != x.batch() * self.rows {
            return Err(NtgError::ShapeMismatch { expected: x.batch() * self.rows, got: out_len });
        }
        if !kernel.is_available() {
            return Err(NtgError::InvalidInput(format!("kernel {} is not supported on this CPU", kernel.name())));
        }
        Ok(())
    }

    /// Exact integer product into `out` (`batch x rows`, row-major).
    pub fn gemm_i8(&self, x: &Int8Activations, out: &mut [i32], kernel: TernaryI8Kernel) -> Result<(), NtgError> {
        self.validate(x, out.len(), kernel)?;
        let xu = to_unsigned(x);
        self.gemm_rows(x, &xu, out.as_mut_ptr(), kernel, 0, self.rows);
        Ok(())
    }

    /// Same result as [`Self::gemm_i8`] on a [`SpinPool`] (rows claimed in chunks).
    pub fn gemm_i8_spin(
        &self,
        x: &Int8Activations,
        out: &mut [i32],
        kernel: TernaryI8Kernel,
        pool: &SpinPool,
    ) -> Result<(), NtgError> {
        self.validate(x, out.len(), kernel)?;
        let xu = to_unsigned(x);
        let threads = pool.threads();
        if threads <= 1 || self.rows < threads * 16 {
            self.gemm_rows(x, &xu, out.as_mut_ptr(), kernel, 0, self.rows);
            return Ok(());
        }
        let chunk = self.rows.div_ceil(threads * 8).next_multiple_of(8).max(64);
        let chunks = self.rows.div_ceil(chunk);
        let next = AtomicUsize::new(0);
        let dst = SharedOut(out.as_mut_ptr());
        pool.run(|_| loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= chunks {
                break;
            }
            let n0 = i * chunk;
            self.gemm_rows(x, &xu, dst.get(), kernel, n0, (n0 + chunk).min(self.rows));
        });
        Ok(())
    }

    /// Rows `n0..n1` for every batch row, written straight into `out` (`batch x self.rows`).
    ///
    /// `out` is a raw pointer because pool threads write disjoint column ranges of every
    /// batch row.
    fn gemm_rows(&self, x: &Int8Activations, xu: &[u8], out: *mut i32, kernel: TernaryI8Kernel, n0: usize, n1: usize) {
        assert!(n0 <= n1 && n1 <= self.rows, "row range out of bounds");
        let batch = x.batch();
        let mut t0 = 0;
        while t0 < batch {
            let t1 = (t0 + TILE).min(batch);
            match kernel {
                #[cfg(target_arch = "x86_64")]
                TernaryI8Kernel::Avx512Vnni => {
                    // SAFETY: availability was checked by `validate`; `n0..n1` and `t0..t1` are
                    // in bounds (asserted / derived above) and `out` has `batch * rows` slots.
                    unsafe { x86::vnni_tile(self, xu, out, t0, t1, n0, n1) }
                }
                #[cfg(target_arch = "x86_64")]
                TernaryI8Kernel::Avx2 => {
                    // SAFETY: as above, for AVX2.
                    unsafe { x86::avx2_tile(self, x.padded(), out, t0, t1, n0, n1) }
                }
                _ => self.scalar_tile(x.padded(), out, t0, t1, n0, n1),
            }
            t0 = t1;
        }
    }

    fn scalar_tile(&self, x: &[i8], out: *mut i32, t0: usize, t1: usize, n0: usize, n1: usize) {
        for b in t0..t1 {
            let xr = &x[b * self.k_padded..(b + 1) * self.k_padded];
            for n in n0..n1 {
                let wr = &self.data[n * self.k_padded..(n + 1) * self.k_padded];
                let acc: i32 = wr.iter().zip(xr).map(|(&w, &v)| w as i32 * v as i32).sum();
                // SAFETY: (b, n) is inside `batch x rows` and owned by this call's row range.
                unsafe { *out.add(b * self.rows + n) = acc };
            }
        }
    }
}

/// `x + 128` as bytes, for the unsigned operand of `vpdpbusd`.
fn to_unsigned(x: &Int8Activations) -> Vec<u8> {
    x.padded().iter().map(|&v| (v as i16 + 128) as u8).collect()
}

struct SharedOut(*mut i32);
// SAFETY: threads write disjoint column ranges; see `gemm_i8_spin`.
unsafe impl Sync for SharedOut {}
impl SharedOut {
    fn get(&self) -> *mut i32 {
        self.0
    }
}

#[cfg(target_arch = "x86_64")]
mod x86 {
    use super::*;
    use std::arch::x86_64::*;

    /// # Safety
    /// CPU must support AVX-512 F/BW/VNNI. `xu` is `batch x k_padded`; `out` has room for
    /// `batch x m.rows` values; `t0..t1` and `n0..n1` are in range.
    #[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
    pub unsafe fn vnni_tile(
        m: &Int8Matrix,
        xu: &[u8],
        out: *mut i32,
        t0: usize,
        t1: usize,
        n0: usize,
        n1: usize,
    ) {
        let kp = m.k_padded;
        for n in n0..n1 {
            let wrow = m.data.as_ptr().add(n * kp) as *const u8;
            let correction = 128 * m.row_sums[n];
            let mut b = t0;
            while b < t1 {
                let nb = (t1 - b).min(4);
                let mut acc = [[_mm512_setzero_si512(); 2]; 4];
                let mut k = 0;
                while k < kp {
                    let w0 = _mm512_loadu_si512(wrow.add(k) as *const _);
                    let w1 = _mm512_loadu_si512(wrow.add(k + 64) as *const _);
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = xu.as_ptr().add((b + bi) * kp + k);
                        a[0] = _mm512_dpbusd_epi32(a[0], _mm512_loadu_si512(xp as *const _), w0);
                        a[1] = _mm512_dpbusd_epi32(a[1], _mm512_loadu_si512(xp.add(64) as *const _), w1);
                    }
                    k += 128;
                }
                for (bi, a) in acc.iter().enumerate().take(nb) {
                    let s = _mm512_add_epi32(a[0], a[1]);
                    *out.add((b + bi) * m.rows + n) = _mm512_reduce_add_epi32(s) - correction;
                }
                b += nb;
            }
        }
    }

    /// # Safety
    /// CPU must support AVX2. `x` is `batch x k_padded` i8; other bounds as for `vnni_tile`.
    #[target_feature(enable = "avx2")]
    pub unsafe fn avx2_tile(m: &Int8Matrix, x: &[i8], out: *mut i32, t0: usize, t1: usize, n0: usize, n1: usize) {
        let kp = m.k_padded;
        for n in n0..n1 {
            let wrow = m.data.as_ptr().add(n * kp);
            for b in t0..t1 {
                let xp = x.as_ptr().add(b * kp);
                let mut acc = _mm256_setzero_si256();
                let mut k = 0;
                while k < kp {
                    // Widen to i16 first: vpmaddubsw would saturate for full-range int8.
                    let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(wrow.add(k) as *const _));
                    let v = _mm256_cvtepi8_epi16(_mm_loadu_si128(xp.add(k) as *const _));
                    acc = _mm256_add_epi32(acc, _mm256_madd_epi16(w, v));
                    k += 16;
                }
                let lo = _mm256_castsi256_si128(acc);
                let hi = _mm256_extracti128_si256(acc, 1);
                let s = _mm_add_epi32(lo, hi);
                let s = _mm_add_epi32(s, _mm_shuffle_epi32(s, 0b01_00_11_10));
                let s = _mm_add_epi32(s, _mm_shuffle_epi32(s, 0b10_11_00_01));
                *out.add(b * m.rows + n) = _mm_cvtsi128_si32(s);
            }
        }
    }
}

#[cfg(test)]
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
        fn f32(&mut self) -> f32 {
            ((self.next() >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 4.0
        }
        fn i8_any(&mut self) -> i8 {
            (self.next() >> 56) as u8 as i8
        }
    }

    fn naive(m: &Int8Matrix, x: &[i8], batch: usize, cols: usize) -> Vec<i32> {
        let mut out = vec![0; batch * m.rows];
        for b in 0..batch {
            for n in 0..m.rows {
                out[b * m.rows + n] = (0..cols)
                    .map(|k| m.data[n * m.k_padded + k] as i32 * x[b * cols + k] as i32)
                    .sum();
            }
        }
        out
    }

    #[test]
    fn every_kernel_and_pool_matches_naive_including_extreme_activations() {
        let mut rng = Rng(0xFEED);
        let pools: Vec<SpinPool> = [2, 3].iter().map(|&t| SpinPool::new(t).unwrap()).collect();
        for &(rows, cols, batch) in &[(130, 300, 1), (257, 513, 5), (96, 256, 40), (1000, 2560, 2)] {
            let w: Vec<f32> = (0..rows * cols).map(|_| rng.f32()).collect();
            let m = Int8Matrix::from_f32(&w, rows, cols).unwrap();
            let mut x: Vec<i8> = (0..batch * cols).map(|_| rng.i8_any()).collect();
            x[0] = -128; // the offset trick must survive the most negative activation
            x[1] = 127;
            let want = naive(&m, &x, batch, cols);
            let acts = Int8Activations::from_i8(&x, batch, cols).unwrap();
            for kernel in TernaryI8Kernel::all_available() {
                let mut got = vec![0; batch * rows];
                m.gemm_i8(&acts, &mut got, kernel).unwrap();
                assert_eq!(got, want, "{} {rows}x{cols} batch {batch}", kernel.name());
                for pool in &pools {
                    let mut got = vec![0; batch * rows];
                    m.gemm_i8_spin(&acts, &mut got, kernel, pool).unwrap();
                    assert_eq!(got, want, "{} pool {} {rows}x{cols} batch {batch}", kernel.name(), pool.threads());
                }
            }
        }
    }

    #[test]
    fn bf16_and_f32_quantisation_agree_and_dequantise_close() {
        let mut rng = Rng(7);
        let (rows, cols) = (50, 700);
        let w: Vec<f32> = (0..rows * cols).map(|_| rng.f32()).collect();
        // Truncate to bf16 so both inputs carry the same values.
        let bits: Vec<u16> = w.iter().map(|v| (v.to_bits() >> 16) as u16).collect();
        let wt: Vec<f32> = bits.iter().map(|&b| f32::from_bits((b as u32) << 16)).collect();
        let a = Int8Matrix::from_f32(&wt, rows, cols).unwrap();
        let b = Int8Matrix::from_bf16_bits(&bits, rows, cols).unwrap();
        assert_eq!(a.data, b.data);
        assert_eq!(a.scales, b.scales);
        for r in 0..rows {
            for c in 0..cols {
                let deq = a.data[r * a.k_padded + c] as f32 * a.scales[r];
                assert!((deq - wt[r * cols + c]).abs() <= a.scales[r] * 0.5 + 1e-6);
            }
        }
    }

    #[test]
    fn rejects_bad_shapes() {
        assert!(Int8Matrix::from_f32(&[0.0; 5], 2, 3).is_err());
        assert!(Int8Matrix::from_f32(&[], 0, 3).is_err());
        let m = Int8Matrix::from_f32(&[1.0; 6], 2, 3).unwrap();
        let acts = Int8Activations::from_i8(&[1; 4], 1, 4).unwrap();
        assert!(m.gemm_i8(&acts, &mut [0; 2], TernaryI8Kernel::Scalar).is_err());
    }
}
