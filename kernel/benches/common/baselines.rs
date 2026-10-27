//! Baseline kernels and helpers shared by the ternary benchmarks (included with `#[path]`).
//!
//! * `dense`: int8 dense VNNI control (1 byte per weight, nothing to unpack).
//! * `f32dot`: AVX-512 FMA row-dot f32 kernel (streams each weight row once per batch group).

#![allow(dead_code)]

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn ternary(&mut self) -> i8 {
        (self.next() % 3) as i8 - 1
    }
    pub fn act(&mut self) -> i8 {
        ((self.next() % 255) as i32 - 127) as i8
    }
}

#[cfg(target_arch = "x86_64")]
pub mod dense {
    use std::arch::x86_64::*;

    pub fn available() -> bool {
        is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512bw")
            && is_x86_feature_detected!("avx512vnni")
    }

    /// Dense int8 control: same loop as the ternary VNNI kernel but the weight codes are
    /// already one byte each, so there is nothing to unpack and 4x the weight traffic.
    ///
    /// # Safety
    /// CPU must support AVX-512 F/BW/VNNI; `w` is `rows x k_padded`, `x` is
    /// `batch x k_padded`, `out` is `batch x rows`.
    #[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
    pub unsafe fn gemm(w: &[u8], rows: usize, k_padded: usize, x: &[i8], sums: &[i32], batch: usize, out: &mut [i32]) {
        let mut b0 = 0;
        while b0 < batch {
            let nb = (batch - b0).min(4);
            for n in 0..rows {
                let wrow = w.as_ptr().add(n * k_padded);
                let mut acc = [[_mm512_setzero_si512(); 4]; 4];
                let mut k = 0;
                while k < k_padded {
                    let w0 = _mm512_loadu_si512(wrow.add(k) as *const _);
                    let w1 = _mm512_loadu_si512(wrow.add(k + 64) as *const _);
                    let w2 = _mm512_loadu_si512(wrow.add(k + 128) as *const _);
                    let w3 = _mm512_loadu_si512(wrow.add(k + 192) as *const _);
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = x.as_ptr().add((b0 + bi) * k_padded + k);
                        a[0] = _mm512_dpbusd_epi32(a[0], w0, _mm512_loadu_si512(xp as *const _));
                        a[1] = _mm512_dpbusd_epi32(a[1], w1, _mm512_loadu_si512(xp.add(64) as *const _));
                        a[2] = _mm512_dpbusd_epi32(a[2], w2, _mm512_loadu_si512(xp.add(128) as *const _));
                        a[3] = _mm512_dpbusd_epi32(a[3], w3, _mm512_loadu_si512(xp.add(192) as *const _));
                    }
                    k += 256;
                }
                for bi in 0..nb {
                    let a = &acc[bi];
                    let s = _mm512_add_epi32(_mm512_add_epi32(a[0], a[1]), _mm512_add_epi32(a[2], a[3]));
                    out[(b0 + bi) * rows + n] = _mm512_reduce_add_epi32(s) - sums[b0 + bi];
                }
            }
            b0 += nb;
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub mod f32dot {
    use std::arch::x86_64::*;

    pub fn available() -> bool {
        is_x86_feature_detected!("avx512f")
    }

    /// `out[b][n] = dot(w[n], x[b])`, streaming each weight row once per group of 4 batch
    /// rows with four independent FMA accumulators per batch row.
    ///
    /// # Safety
    /// CPU must support AVX-512F; `w` is `rows x cols`, `x` is `batch x cols`,
    /// `out` is `batch x rows`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn gemm(w: &[f32], rows: usize, cols: usize, x: &[f32], batch: usize, out: &mut [f32]) {
        let mut b0 = 0;
        while b0 < batch {
            let nb = (batch - b0).min(4);
            for n in 0..rows {
                let wrow = w.as_ptr().add(n * cols);
                let mut acc = [[_mm512_setzero_ps(); 4]; 4];
                let mut k = 0;
                while k + 64 <= cols {
                    let w0 = _mm512_loadu_ps(wrow.add(k));
                    let w1 = _mm512_loadu_ps(wrow.add(k + 16));
                    let w2 = _mm512_loadu_ps(wrow.add(k + 32));
                    let w3 = _mm512_loadu_ps(wrow.add(k + 48));
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = x.as_ptr().add((b0 + bi) * cols + k);
                        a[0] = _mm512_fmadd_ps(w0, _mm512_loadu_ps(xp), a[0]);
                        a[1] = _mm512_fmadd_ps(w1, _mm512_loadu_ps(xp.add(16)), a[1]);
                        a[2] = _mm512_fmadd_ps(w2, _mm512_loadu_ps(xp.add(32)), a[2]);
                        a[3] = _mm512_fmadd_ps(w3, _mm512_loadu_ps(xp.add(48)), a[3]);
                    }
                    k += 64;
                }
                for bi in 0..nb {
                    let a = &acc[bi];
                    let s = _mm512_add_ps(_mm512_add_ps(a[0], a[1]), _mm512_add_ps(a[2], a[3]));
                    let mut total = _mm512_reduce_add_ps(s);
                    for kk in k..cols {
                        total += *wrow.add(kk) * x[(b0 + bi) * cols + kk];
                    }
                    out[(b0 + bi) * rows + n] = total;
                }
            }
            b0 += nb;
        }
    }
}

/// bf16 row-dot baseline: bf16 weights (2 bytes each), `vdpbf16ps`. This is the practical
/// baseline for a model shipped in bf16 (the official BitNet path holds unpacked weights in
/// bf16). Ternary values and small-integer activations are exact in bf16, so results are
/// bit-identical to the other kernels.
#[cfg(target_arch = "x86_64")]
pub mod bf16dot {
    use std::arch::x86_64::*;

    pub fn available() -> bool {
        is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bf16")
    }

    /// Row length used by [`gemm`]: padded with zeros to a multiple of 128 elements.
    pub fn padded(cols: usize) -> usize {
        cols.next_multiple_of(128)
    }

    /// Truncating f32 -> bf16; exact for the small integers used here.
    pub fn to_bf16(v: f32) -> u16 {
        (v.to_bits() >> 16) as u16
    }

    /// `out[b][n] = dot(w[n], x[b])` for bf16 `w` (`rows x cols_p`) and `x` (`batch x cols_p`).
    ///
    /// # Safety
    /// CPU must support AVX-512F and AVX512-BF16; `cols_p` must be a multiple of 128;
    /// `w`, `x`, `out` must have `rows * cols_p`, `batch * cols_p`, `batch * rows` elements.
    #[target_feature(enable = "avx512f,avx512bf16")]
    pub unsafe fn gemm(w: &[u16], rows: usize, cols_p: usize, x: &[u16], batch: usize, out: &mut [f32]) {
        let mut b0 = 0;
        while b0 < batch {
            let nb = (batch - b0).min(4);
            for n in 0..rows {
                let wrow = w.as_ptr().add(n * cols_p);
                let mut acc = [[_mm512_setzero_ps(); 4]; 4];
                let mut k = 0;
                while k < cols_p {
                    let w0: __m512bh = std::mem::transmute(_mm512_loadu_si512(wrow.add(k) as *const _));
                    let w1: __m512bh = std::mem::transmute(_mm512_loadu_si512(wrow.add(k + 32) as *const _));
                    let w2: __m512bh = std::mem::transmute(_mm512_loadu_si512(wrow.add(k + 64) as *const _));
                    let w3: __m512bh = std::mem::transmute(_mm512_loadu_si512(wrow.add(k + 96) as *const _));
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = x.as_ptr().add((b0 + bi) * cols_p + k);
                        let x0: __m512bh = std::mem::transmute(_mm512_loadu_si512(xp as *const _));
                        let x1: __m512bh = std::mem::transmute(_mm512_loadu_si512(xp.add(32) as *const _));
                        let x2: __m512bh = std::mem::transmute(_mm512_loadu_si512(xp.add(64) as *const _));
                        let x3: __m512bh = std::mem::transmute(_mm512_loadu_si512(xp.add(96) as *const _));
                        a[0] = _mm512_dpbf16_ps(a[0], w0, x0);
                        a[1] = _mm512_dpbf16_ps(a[1], w1, x1);
                        a[2] = _mm512_dpbf16_ps(a[2], w2, x2);
                        a[3] = _mm512_dpbf16_ps(a[3], w3, x3);
                    }
                    k += 128;
                }
                for (bi, a) in acc.iter().enumerate().take(nb) {
                    let s = _mm512_add_ps(_mm512_add_ps(a[0], a[1]), _mm512_add_ps(a[2], a[3]));
                    out[(b0 + bi) * rows + n] = _mm512_reduce_add_ps(s);
                }
            }
            b0 += nb;
        }
    }
}
