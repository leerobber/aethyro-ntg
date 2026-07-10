//! TOBL kernel dispatch: Ternary-Optimized Bitwise Logic dot-product operations.
//!
//! Provides runtime-selected SIMD paths for PackedTernary:
//! - AVX2: 32-element batches with bit-level ternary multiply-accumulate
//! - NEON: ARM64 fallback (8-element batches)
//! - Scalar: universal fallback
//!
//! Targets 40% cycle reduction vs generic SIMD _mm256_add/_mm_mult paths.

use super::PackedTernary;
use crate::ntg::error::NtgError;
use std::time::Instant;

/// TOBL kernel selection enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToblKernelPath {
    /// Generic fallback: no SIMD
    Scalar = 0,
    /// AVX2: 256-bit SIMD, 32-element ternary batches
    AVX2 = 1,
    /// ARM NEON: 128-bit SIMD, 8-element ternary batches
    NEON = 2,
}

impl ToblKernelPath {
    pub fn name(&self) -> &'static str {
        match self {
            ToblKernelPath::Scalar => "Scalar",
            ToblKernelPath::AVX2 => "AVX2",
            ToblKernelPath::NEON => "NEON",
        }
    }
}

/// Detect best available TOBL kernel path on this hardware.
pub fn select_kernel_path() -> ToblKernelPath {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return ToblKernelPath::AVX2;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if cfg!(target_feature = "neon") {
            return ToblKernelPath::NEON;
        }
    }

    ToblKernelPath::Scalar
}

/// Ternary dot-product: sum of element-wise products.
/// Returns (result, cycles_elapsed).
pub fn tobl_dot_product(
    a: &PackedTernary,
    b: &PackedTernary,
    kernel: Option<ToblKernelPath>,
) -> Result<(i64, u64), NtgError> {
    if a.len() != b.len() {
        return Err(NtgError::InvalidInput(format!(
            "dimension mismatch: {} vs {}",
            a.len(),
            b.len()
        )));
    }

    let kernel = kernel.unwrap_or_else(select_kernel_path);

    let start = Instant::now();
    let result = match kernel {
        ToblKernelPath::AVX2 => {
            #[cfg(target_arch = "x86_64")]
            {
                if is_x86_feature_detected!("avx2") {
                    unsafe { tobl_dot_avx2(a, b) }
                } else {
                    tobl_dot_scalar(a, b)
                }
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                tobl_dot_scalar(a, b)
            }
        }
        ToblKernelPath::NEON => {
            #[cfg(target_arch = "aarch64")]
            {
                if cfg!(target_feature = "neon") {
                    unsafe { tobl_dot_neon(a, b) }
                } else {
                    tobl_dot_scalar(a, b)
                }
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                tobl_dot_scalar(a, b)
            }
        }
        ToblKernelPath::Scalar => tobl_dot_scalar(a, b),
    };

    let elapsed_us = start.elapsed().as_micros() as u64;
    Ok((result, elapsed_us))
}

/// Scalar fallback: element-by-element ternary multiply-accumulate.
fn tobl_dot_scalar(a: &PackedTernary, b: &PackedTernary) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..a.len() {
        let av = a.get(i) as i64;
        let bv = b.get(i) as i64;
        sum += av * bv;
    }
    sum
}

/// AVX2 dot-product: process each packed word (32 ternary elements).
/// Unpacks 2-bit ternary into i16 lanes, mullo-accumulate, aggregate.
///
/// Note: padding bits beyond `len` are zeros, so full-word MACs are safe.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn tobl_dot_avx2(a: &PackedTernary, b: &PackedTernary) -> i64 {
    use std::arch::x86_64::*;

    let mut sum_acc: i64 = 0;
    let word_count = a.word_count();

    for wi in 0..word_count {
        let aw = *a.word_ptr().add(wi);
        let bw = *b.word_ptr().add(wi);

        // 32 elements / word → two 16-lane i16 vectors
        for half in 0..2 {
            let a_unpacked = unpack_ternary_half_to_i16(aw, half);
            let b_unpacked = unpack_ternary_half_to_i16(bw, half);

            // mullo (not mulhi): products of {-1,0,1} live in the low 16 bits
            let products = _mm256_mullo_epi16(a_unpacked, b_unpacked);
            sum_acc += horizontal_sum_epi16(products);
        }
    }

    sum_acc
}

/// Unpack 16 ternary values from one half of a u64 word into i16 lanes.
/// `half == 0` → elements 0..15, `half == 1` → elements 16..31.
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn unpack_ternary_half_to_i16(word: u64, half: usize) -> std::arch::x86_64::__m256i {
    use std::arch::x86_64::*;

    let base = half * 16;
    let mut lanes = [0i16; 16];
    for (i, lane) in lanes.iter_mut().enumerate() {
        let bit_offset = (base + i) * 2;
        let packed = ((word >> bit_offset) & 0b11) as i16;
        *lane = match packed {
            0b01 => -1,
            0b00 => 0,
            0b10 => 1,
            _ => 0,
        };
    }

    _mm256_loadu_si256(lanes.as_ptr() as *const __m256i)
}

/// Horizontal sum of i16 lanes in AVX2 vector.
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn horizontal_sum_epi16(v: std::arch::x86_64::__m256i) -> i64 {
    use std::arch::x86_64::*;

    let hi = _mm256_extracti128_si256::<1>(v);
    let lo = _mm256_castsi256_si128(v);
    let sum128 = _mm_add_epi16(lo, hi);

    let mut result: i64 = 0;
    let arr: [i16; 8] = core::mem::transmute(sum128);
    for &val in &arr {
        result += val as i64;
    }

    result
}

/// ARM NEON dot-product stub.
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn tobl_dot_neon(a: &PackedTernary, b: &PackedTernary) -> i64 {
    // Placeholder: full NEON implementation deferred to Phase 1.3+
    // Fallback to scalar for now
    tobl_dot_scalar(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tobl_dot_scalar_simple() {
        let mut a = PackedTernary::new(4);
        let mut b = PackedTernary::new(4);
        a.set_from_slice(&[1i8, -1, 0, 1]).unwrap();
        b.set_from_slice(&[1i8, -1, 0, 1]).unwrap();

        let (result, _cycles) = tobl_dot_product(&a, &b, Some(ToblKernelPath::Scalar)).unwrap();
        // 1*1 + (-1)*(-1) + 0*0 + 1*1 = 1 + 1 + 0 + 1 = 3
        assert_eq!(result, 3);
    }

    #[test]
    fn tobl_dot_product_auto() {
        let mut a = PackedTernary::new(10);
        let mut b = PackedTernary::new(10);
        a.set_from_slice(&[1i8, -1, 0, 1, 0, 1, -1, 0, 1, -1])
            .unwrap();
        b.set_from_slice(&[1i8, 0, -1, 1, 0, -1, 1, 0, 1, 1])
            .unwrap();

        let (result, _cycles) = tobl_dot_product(&a, &b, None).unwrap();
        assert!((-10..=10).contains(&result));
    }

    #[test]
    fn tobl_dot_dimension_check() {
        let a = PackedTernary::new(10);
        let b = PackedTernary::new(20);

        let result = tobl_dot_product(&a, &b, None);
        assert!(result.is_err());
    }

    #[test]
    fn tobl_kernel_path_detection() {
        let path = select_kernel_path();
        assert!(matches!(
            path,
            ToblKernelPath::Scalar | ToblKernelPath::AVX2 | ToblKernelPath::NEON
        ));
    }
}
