//! Ternary-weight x int8-activation GEMM.
//!
//! Computes `out[b][n] = sum_k W[n][k] * x[b][k]` exactly in `i32`, where `W` holds
//! `{-1, 0, +1}` and `x` holds `i8`. Weights are packed 2 bits each (16x smaller
//! than f32) and unpacked inside the SIMD loop, so the memory saving is not paid
//! back as a separate decode pass.
//!
//! # Encoding
//!
//! A weight `w` is stored as the code `c = w + 1` in `{0, 1, 2}`. The SIMD kernels
//! multiply the unsigned code by the signed activation (`vpdpbusd` / `vpmaddubsw`
//! take an unsigned first operand) and remove the offset afterwards:
//!
//! ```text
//! sum_k (w_k + 1) * x_k  -  sum_k x_k  =  sum_k w_k * x_k
//! ```
//!
//! The correction `sum_k x_k` depends only on the activation vector, so it is
//! computed once per batch row and shared by every output row.
//!
//! # Layout
//!
//! Each weight row is padded to a multiple of [`GROUP`] (256) elements and stored as
//! 64-byte groups. Byte `j` of a group holds the codes for `k = j`, `j+64`, `j+128`,
//! `j+192` (at bit offsets 0, 2, 4, 6). Shifting and masking a 64-byte vector
//! therefore yields four 64-byte vectors whose lanes line up with four contiguous
//! 64-element activation loads, with no shuffles. Padding weights are code 1 (zero)
//! and padded activations are 0.
//!
//! # Kernels
//!
//! [`TernaryI8Kernel::Avx512Vnni`] (`vpdpbusd`), [`TernaryI8Kernel::Avx2`]
//! (`vpmaddubsw`) and a [`TernaryI8Kernel::Scalar`] reference. All three give
//! bit-identical results; the SIMD ones are chosen at run time.

use crate::ntg::error::NtgError;

/// Elements per packed group (64 bytes x 4 codes per byte).
pub const GROUP: usize = 256;
const GROUP_BYTES: usize = GROUP / 4;

/// Which implementation computes the product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TernaryI8Kernel {
    /// Portable reference.
    Scalar,
    /// 256-bit `vpmaddubsw`.
    Avx2,
    /// 512-bit `vpdpbusd` (AVX-512 VNNI).
    Avx512Vnni,
}

impl TernaryI8Kernel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Avx2 => "avx2",
            Self::Avx512Vnni => "avx512-vnni",
        }
    }

    /// Whether this CPU can run the kernel.
    pub fn is_available(self) -> bool {
        match self {
            Self::Scalar => true,
            #[cfg(target_arch = "x86_64")]
            Self::Avx2 => is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "x86_64")]
            Self::Avx512Vnni => {
                is_x86_feature_detected!("avx512f")
                    && is_x86_feature_detected!("avx512bw")
                    && is_x86_feature_detected!("avx512vnni")
            }
            #[cfg(not(target_arch = "x86_64"))]
            _ => false,
        }
    }

    /// Fastest kernel this CPU supports.
    pub fn best_available() -> Self {
        if Self::Avx512Vnni.is_available() {
            Self::Avx512Vnni
        } else if Self::Avx2.is_available() {
            Self::Avx2
        } else {
            Self::Scalar
        }
    }

    /// All kernels this CPU supports, slowest first.
    pub fn all_available() -> Vec<Self> {
        [Self::Scalar, Self::Avx2, Self::Avx512Vnni]
            .into_iter()
            .filter(|k| k.is_available())
            .collect()
    }
}

fn padded(cols: usize) -> usize {
    cols.div_ceil(GROUP) * GROUP
}

/// Int8 activations for a batch, zero-padded to the weight row width.
#[derive(Clone, Debug)]
pub struct Int8Activations {
    batch: usize,
    cols: usize,
    k_padded: usize,
    data: Vec<i8>,
    sums: Vec<i32>,
    scales: Vec<f32>,
}

impl Int8Activations {
    /// Wrap already-quantised activations (`batch x cols`, row-major). Scales are 1.0.
    pub fn from_i8(x: &[i8], batch: usize, cols: usize) -> Result<Self, NtgError> {
        if x.len() != batch * cols {
            return Err(NtgError::ShapeMismatch { expected: batch * cols, got: x.len() });
        }
        let k_padded = padded(cols);
        let mut data = vec![0i8; batch * k_padded];
        let mut sums = vec![0i32; batch];
        for b in 0..batch {
            let row = &x[b * cols..(b + 1) * cols];
            data[b * k_padded..b * k_padded + cols].copy_from_slice(row);
            sums[b] = row.iter().map(|&v| v as i32).sum();
        }
        Ok(Self { batch, cols, k_padded, data, sums, scales: vec![1.0; batch] })
    }

    /// Symmetric per-row quantisation: `scale = max|x| / 127`, `q = round(x / scale)`
    /// (half away from zero). An all-zero row gets scale 1.0.
    pub fn quantize_f32(x: &[f32], batch: usize, cols: usize) -> Result<Self, NtgError> {
        if x.len() != batch * cols {
            return Err(NtgError::ShapeMismatch { expected: batch * cols, got: x.len() });
        }
        let k_padded = padded(cols);
        let mut data = vec![0i8; batch * k_padded];
        let mut sums = vec![0i32; batch];
        let mut scales = vec![1.0f32; batch];
        for b in 0..batch {
            let row = &x[b * cols..(b + 1) * cols];
            let max = row.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
            let scale = if max == 0.0 { 1.0 } else { max / 127.0 };
            scales[b] = scale;
            let dst = &mut data[b * k_padded..b * k_padded + cols];
            let mut sum = 0i32;
            for (d, &v) in dst.iter_mut().zip(row) {
                let q = (v / scale).round().clamp(-127.0, 127.0) as i8;
                *d = q;
                sum += q as i32;
            }
            sums[b] = sum;
        }
        Ok(Self { batch, cols, k_padded, data, sums, scales })
    }

    pub fn batch(&self) -> usize {
        self.batch
    }
    pub fn cols(&self) -> usize {
        self.cols
    }
    /// Per-row dequantisation scales.
    pub fn scales(&self) -> &[f32] {
        &self.scales
    }
    /// Row width after zero-padding to a multiple of [`GROUP`].
    pub fn k_padded(&self) -> usize {
        self.k_padded
    }
    /// Zero-padded activations, `batch x k_padded`, row-major.
    pub fn padded(&self) -> &[i8] {
        &self.data
    }
    /// `sum_k x[b][k]` per batch row (the offset correction).
    pub fn row_sums(&self) -> &[i32] {
        &self.sums
    }
}

/// A ternary weight matrix (`rows x cols`) packed 2 bits per weight.
#[derive(Clone, Debug)]
pub struct PackedTernaryMatrix {
    rows: usize,
    cols: usize,
    k_padded: usize,
    bytes_per_row: usize,
    data: Vec<u8>,
    scale: f32,
}

impl PackedTernaryMatrix {
    /// Pack `weights` (`rows x cols`, row-major, each in `{-1, 0, 1}`).
    pub fn from_i8(weights: &[i8], rows: usize, cols: usize) -> Result<Self, NtgError> {
        if weights.len() != rows * cols {
            return Err(NtgError::ShapeMismatch { expected: rows * cols, got: weights.len() });
        }
        if let Some(&bad) = weights.iter().find(|&&w| !(-1..=1).contains(&w)) {
            return Err(NtgError::InvalidTernaryValue(bad));
        }
        let k_padded = padded(cols);
        let bytes_per_row = k_padded / 4;
        // Code 1 (= weight 0) everywhere, so padding contributes nothing.
        let mut data = vec![0b0101_0101u8; rows * bytes_per_row];
        for r in 0..rows {
            let row = &weights[r * cols..(r + 1) * cols];
            let out = &mut data[r * bytes_per_row..(r + 1) * bytes_per_row];
            for (k, &w) in row.iter().enumerate() {
                let group = k / GROUP;
                let in_group = k % GROUP;
                let byte = group * GROUP_BYTES + in_group % 64;
                let shift = 2 * (in_group / 64);
                let code = (w + 1) as u8;
                out[byte] = (out[byte] & !(0b11 << shift)) | (code << shift);
            }
        }
        Ok(Self { rows, cols, k_padded, bytes_per_row, data, scale: 1.0 })
    }

    /// Attach the weight dequantisation scale (for example BitNet's mean |w|).
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cols(&self) -> usize {
        self.cols
    }
    /// Bytes of packed weight storage.
    pub fn packed_bytes(&self) -> usize {
        self.data.len()
    }

    /// Weight at `(r, c)`.
    pub fn get(&self, r: usize, c: usize) -> i8 {
        assert!(r < self.rows && c < self.cols, "index out of bounds");
        let group = c / GROUP;
        let in_group = c % GROUP;
        let byte = self.data[r * self.bytes_per_row + group * GROUP_BYTES + in_group % 64];
        ((byte >> (2 * (in_group / 64))) & 0b11) as i8 - 1
    }

    /// Exact integer product into `out` (`batch x rows`, row-major).
    pub fn gemm_i8(
        &self,
        x: &Int8Activations,
        out: &mut [i32],
        kernel: TernaryI8Kernel,
    ) -> Result<(), NtgError> {
        if x.cols != self.cols {
            return Err(NtgError::ShapeMismatch { expected: self.cols, got: x.cols });
        }
        if out.len() != x.batch * self.rows {
            return Err(NtgError::ShapeMismatch { expected: x.batch * self.rows, got: out.len() });
        }
        if !kernel.is_available() {
            return Err(NtgError::InvalidInput(format!(
                "kernel {} is not supported on this CPU",
                kernel.name()
            )));
        }
        debug_assert_eq!(x.k_padded, self.k_padded);
        match kernel {
            TernaryI8Kernel::Scalar => self.gemm_scalar(x, out),
            #[cfg(target_arch = "x86_64")]
            TernaryI8Kernel::Avx2 => {
                let mut b0 = 0;
                while b0 < x.batch {
                    let nb = (x.batch - b0).min(4);
                    // SAFETY: availability was checked above; slice bounds are
                    // validated by the shape checks and by `x86::*` index math.
                    unsafe {
                        match nb {
                            4 => x86::avx2_group::<4>(self, x, out, b0),
                            3 => x86::avx2_group::<3>(self, x, out, b0),
                            2 => x86::avx2_group::<2>(self, x, out, b0),
                            _ => x86::avx2_group::<1>(self, x, out, b0),
                        }
                    }
                    b0 += nb;
                }
            }
            #[cfg(target_arch = "x86_64")]
            TernaryI8Kernel::Avx512Vnni => {
                let mut b0 = 0;
                while b0 < x.batch {
                    let nb = (x.batch - b0).min(4);
                    // SAFETY: as above.
                    unsafe {
                        match nb {
                            4 => x86::vnni_group::<4>(self, x, out, b0),
                            3 => x86::vnni_group::<3>(self, x, out, b0),
                            2 => x86::vnni_group::<2>(self, x, out, b0),
                            _ => x86::vnni_group::<1>(self, x, out, b0),
                        }
                    }
                    b0 += nb;
                }
            }
            #[cfg(not(target_arch = "x86_64"))]
            _ => unreachable!("is_available() is false on this architecture"),
        }
        Ok(())
    }

    /// Dequantised layer output: `acc * x_scale[b] * weight_scale` (`batch x rows`).
    pub fn forward_f32(
        &self,
        x: &Int8Activations,
        out: &mut [f32],
        kernel: TernaryI8Kernel,
    ) -> Result<(), NtgError> {
        let mut acc = vec![0i32; x.batch * self.rows];
        self.gemm_i8(x, &mut acc, kernel)?;
        if out.len() != acc.len() {
            return Err(NtgError::ShapeMismatch { expected: acc.len(), got: out.len() });
        }
        for b in 0..x.batch {
            for n in 0..self.rows {
                out[b * self.rows + n] = acc[b * self.rows + n] as f32 * x.scales[b] * self.scale;
            }
        }
        Ok(())
    }

    fn gemm_scalar(&self, x: &Int8Activations, out: &mut [i32]) {
        let mut codes = vec![0u8; self.k_padded];
        for n in 0..self.rows {
            let row = &self.data[n * self.bytes_per_row..(n + 1) * self.bytes_per_row];
            for (g, chunk) in row.chunks_exact(GROUP_BYTES).enumerate() {
                for (j, &byte) in chunk.iter().enumerate() {
                    for i in 0..4 {
                        codes[g * GROUP + 64 * i + j] = (byte >> (2 * i)) & 0b11;
                    }
                }
            }
            for b in 0..x.batch {
                let xs = &x.data[b * self.k_padded..(b + 1) * self.k_padded];
                let dot: i32 = codes.iter().zip(xs).map(|(&c, &v)| c as i32 * v as i32).sum();
                out[b * self.rows + n] = dot - x.sums[b];
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod x86 {
    use super::{Int8Activations, PackedTernaryMatrix};
    use std::arch::x86_64::*;

    /// `NB` batch rows at once, so each unpacked weight vector is reused `NB` times.
    ///
    /// # Safety
    /// The CPU must support AVX-512 F/BW/VNNI. `out`, `x` and `w` must have the
    /// shapes validated by [`PackedTernaryMatrix::gemm_i8`], and `b0 + NB <= batch`.
    #[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
    pub unsafe fn vnni_group<const NB: usize>(
        w: &PackedTernaryMatrix,
        x: &Int8Activations,
        out: &mut [i32],
        b0: usize,
    ) {
        let m3 = _mm512_set1_epi8(3);
        for n in 0..w.rows {
            let wrow = w.data.as_ptr().add(n * w.bytes_per_row);
            // Four independent accumulators per batch row hide dpbusd latency.
            let mut acc = [[_mm512_setzero_si512(); 4]; NB];
            let mut c = 0;
            while c < w.bytes_per_row {
                let v = _mm512_loadu_si512(wrow.add(c) as *const _);
                let w0 = _mm512_and_si512(v, m3);
                let w1 = _mm512_and_si512(_mm512_srli_epi16::<2>(v), m3);
                let w2 = _mm512_and_si512(_mm512_srli_epi16::<4>(v), m3);
                let w3 = _mm512_and_si512(_mm512_srli_epi16::<6>(v), m3);
                let kbase = c * 4;
                for (bi, a) in acc.iter_mut().enumerate() {
                    let xp = x.data.as_ptr().add((b0 + bi) * x.k_padded + kbase);
                    a[0] = _mm512_dpbusd_epi32(a[0], w0, _mm512_loadu_si512(xp as *const _));
                    a[1] = _mm512_dpbusd_epi32(a[1], w1, _mm512_loadu_si512(xp.add(64) as *const _));
                    a[2] = _mm512_dpbusd_epi32(a[2], w2, _mm512_loadu_si512(xp.add(128) as *const _));
                    a[3] = _mm512_dpbusd_epi32(a[3], w3, _mm512_loadu_si512(xp.add(192) as *const _));
                }
                c += 64;
            }
            for (bi, a) in acc.iter().enumerate() {
                let s = _mm512_add_epi32(_mm512_add_epi32(a[0], a[1]), _mm512_add_epi32(a[2], a[3]));
                out[(b0 + bi) * w.rows + n] = _mm512_reduce_add_epi32(s) - x.sums[b0 + bi];
            }
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn hsum_epi32(v: __m256i) -> i32 {
        let lo = _mm256_castsi256_si128(v);
        let hi = _mm256_extracti128_si256::<1>(v);
        let s = _mm_add_epi32(lo, hi);
        let s = _mm_add_epi32(s, _mm_shuffle_epi32::<0b01_00_11_10>(s));
        let s = _mm_add_epi32(s, _mm_shuffle_epi32::<0b10_11_00_01>(s));
        _mm_cvtsi128_si32(s)
    }

    /// AVX2 version: `vpmaddubsw` on 32-byte halves of each 64-byte group.
    ///
    /// # Safety
    /// The CPU must support AVX2, with the same shape preconditions as [`vnni_group`].
    #[target_feature(enable = "avx2")]
    pub unsafe fn avx2_group<const NB: usize>(
        w: &PackedTernaryMatrix,
        x: &Int8Activations,
        out: &mut [i32],
        b0: usize,
    ) {
        let m3 = _mm256_set1_epi8(3);
        let ones = _mm256_set1_epi16(1);
        for n in 0..w.rows {
            let wrow = w.data.as_ptr().add(n * w.bytes_per_row);
            let mut acc = [_mm256_setzero_si256(); NB];
            let mut c = 0;
            while c < w.bytes_per_row {
                let v = _mm256_loadu_si256(wrow.add(c) as *const __m256i);
                let w0 = _mm256_and_si256(v, m3);
                let w1 = _mm256_and_si256(_mm256_srli_epi16::<2>(v), m3);
                let w2 = _mm256_and_si256(_mm256_srli_epi16::<4>(v), m3);
                let w3 = _mm256_and_si256(_mm256_srli_epi16::<6>(v), m3);
                // Bytes c.. cover k = (c/64)*256 + (c%64) + 64*i.
                let kbase = (c / 64) * 256 + (c % 64);
                for (bi, a) in acc.iter_mut().enumerate() {
                    let xp = x.data.as_ptr().add((b0 + bi) * x.k_padded + kbase);
                    // Each maddubs lane is at most 2 * (2 * 128) = 512, so summing four
                    // of them in i16 (<= 2048) cannot overflow before widening.
                    let p0 = _mm256_maddubs_epi16(w0, _mm256_loadu_si256(xp as *const __m256i));
                    let p1 = _mm256_maddubs_epi16(w1, _mm256_loadu_si256(xp.add(64) as *const __m256i));
                    let p2 = _mm256_maddubs_epi16(w2, _mm256_loadu_si256(xp.add(128) as *const __m256i));
                    let p3 = _mm256_maddubs_epi16(w3, _mm256_loadu_si256(xp.add(192) as *const __m256i));
                    let s = _mm256_add_epi16(_mm256_add_epi16(p0, p1), _mm256_add_epi16(p2, p3));
                    *a = _mm256_add_epi32(*a, _mm256_madd_epi16(s, ones));
                }
                c += 32;
            }
            for (bi, a) in acc.iter().enumerate() {
                out[(b0 + bi) * w.rows + n] = hsum_epi32(*a) - x.sums[b0 + bi];
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
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn ternary(&mut self) -> i8 {
            (self.next() % 3) as i8 - 1
        }
        fn i8_any(&mut self) -> i8 {
            // Bias toward the extremes that stress overflow behaviour.
            match self.next() % 8 {
                0 => -128,
                1 => 127,
                _ => (self.next() % 256) as u8 as i8,
            }
        }
    }

    fn naive(w: &[i8], x: &[i8], rows: usize, cols: usize, batch: usize) -> Vec<i32> {
        let mut out = vec![0i32; batch * rows];
        for b in 0..batch {
            for n in 0..rows {
                out[b * rows + n] = (0..cols)
                    .map(|k| w[n * cols + k] as i32 * x[b * cols + k] as i32)
                    .sum();
            }
        }
        out
    }

    #[test]
    fn pack_roundtrip_across_shapes() {
        let mut rng = Rng(1);
        for &(rows, cols) in &[(1, 1), (3, 255), (2, 256), (4, 257), (5, 700), (1, 1024)] {
            let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
            let m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap();
            for r in 0..rows {
                for c in 0..cols {
                    assert_eq!(m.get(r, c), w[r * cols + c], "({rows}x{cols}) at ({r},{c})");
                }
            }
        }
    }

    #[test]
    fn packed_size_is_two_bits_per_weight() {
        let w = vec![0i8; 4 * 1024];
        let m = PackedTernaryMatrix::from_i8(&w, 4, 1024).unwrap();
        assert_eq!(m.packed_bytes(), 4 * 1024 / 4);
    }

    #[test]
    fn every_kernel_matches_naive_integer_math() {
        let mut rng = Rng(0xABCDEF);
        let shapes = [
            (1, 1, 1),
            (3, 255, 1),
            (2, 256, 2),
            (5, 257, 3),
            (7, 700, 4),
            (4, 1024, 5),
            (9, 513, 7),
            (3, 2048, 9),
        ];
        for &(rows, cols, batch) in &shapes {
            let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
            let x: Vec<i8> = (0..batch * cols).map(|_| rng.i8_any()).collect();
            let want = naive(&w, &x, rows, cols, batch);
            let m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap();
            let acts = Int8Activations::from_i8(&x, batch, cols).unwrap();
            for kernel in TernaryI8Kernel::all_available() {
                let mut got = vec![0i32; batch * rows];
                m.gemm_i8(&acts, &mut got, kernel).unwrap();
                assert_eq!(got, want, "{} on {rows}x{cols} batch {batch}", kernel.name());
            }
        }
    }

    #[test]
    fn extreme_activations_do_not_overflow() {
        // All -128 against all +1 / all -1 rows: the i16 intermediate in the AVX2
        // path is at its bound here.
        for fill in [-128i8, 127] {
            let cols = 1024;
            let w: Vec<i8> = (0..2 * cols).map(|i| if i < cols { 1 } else { -1 }).collect();
            let x = vec![fill; cols];
            let want = naive(&w, &x, 2, cols, 1);
            let m = PackedTernaryMatrix::from_i8(&w, 2, cols).unwrap();
            let acts = Int8Activations::from_i8(&x, 1, cols).unwrap();
            for kernel in TernaryI8Kernel::all_available() {
                let mut got = vec![0i32; 2];
                m.gemm_i8(&acts, &mut got, kernel).unwrap();
                assert_eq!(got, want, "{} fill {fill}", kernel.name());
            }
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(matches!(
            PackedTernaryMatrix::from_i8(&[0, 2], 1, 2),
            Err(NtgError::InvalidTernaryValue(2))
        ));
        assert!(matches!(
            PackedTernaryMatrix::from_i8(&[0, 1, 0], 2, 2),
            Err(NtgError::ShapeMismatch { .. })
        ));
        let m = PackedTernaryMatrix::from_i8(&[1, 0, -1, 1], 2, 2).unwrap();
        let acts = Int8Activations::from_i8(&[1, 2, 3], 1, 3).unwrap();
        let mut out = vec![0i32; 2];
        assert!(m.gemm_i8(&acts, &mut out, TernaryI8Kernel::Scalar).is_err());
        let acts = Int8Activations::from_i8(&[1, 2], 1, 2).unwrap();
        let mut short = vec![0i32; 1];
        assert!(m.gemm_i8(&acts, &mut short, TernaryI8Kernel::Scalar).is_err());
    }

    #[test]
    fn quantize_f32_is_symmetric_and_bounded() {
        let x = [0.5f32, -1.0, 0.25, 1.0];
        let a = Int8Activations::quantize_f32(&x, 1, 4).unwrap();
        assert_eq!(&a.data[..4], &[64, -127, 32, 127]);
        assert!((a.scales()[0] - 1.0 / 127.0).abs() < 1e-9);
        let z = Int8Activations::quantize_f32(&[0.0; 4], 1, 4).unwrap();
        assert_eq!(z.scales()[0], 1.0);
        assert!(z.data.iter().all(|&v| v == 0));
    }

    #[test]
    fn forward_f32_applies_both_scales() {
        let m = PackedTernaryMatrix::from_i8(&[1, -1, 0, 1], 2, 2).unwrap().with_scale(0.5);
        let acts = Int8Activations::quantize_f32(&[2.0, 1.0], 1, 2).unwrap();
        let mut out = vec![0.0f32; 2];
        m.forward_f32(&acts, &mut out, TernaryI8Kernel::Scalar).unwrap();
        // x_q = [127, 64]; row0 = 127 - 64 = 63; row1 = 64; scale = (2/127) * 0.5
        let s = (2.0f32 / 127.0) * 0.5;
        assert!((out[0] - 63.0 * s).abs() < 1e-5, "{}", out[0]);
        assert!((out[1] - 64.0 * s).abs() < 1e-5, "{}", out[1]);
    }

    #[test]
    fn best_available_is_available_and_listed() {
        let best = TernaryI8Kernel::best_available();
        assert!(best.is_available());
        assert!(TernaryI8Kernel::all_available().contains(&best));
    }
}
