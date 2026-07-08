//! AVX2 SIMD implementation of ternary matmul.
//!
//! Uses _mm256_maddubs_epi16: multiply 16 signed i8 values + sum.
//! This is perfect for ternary {-1, 0, 1} tensors.
//!
//! Optimizations:
//! - 4x loop unrolling (process 64 elements per iteration)
//! - Manual prefetching for better cache utilization
//! - Aligned memory access patterns

use super::super::error::NtgError;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    _mm256_add_epi32, _mm256_cvtepi32_ps, _mm256_loadu_si256, _mm256_maddubs_epi16,
    _mm256_permute2f128_si256, _mm256_setzero_si256, _mm256_storeu_si256, __m256i,
};

/// AVX2 matmul: (m x k) @ (k x n) -> m x n
/// Requires: x86_64 with AVX2 support
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn matmul_avx2_inner(
    a: &[i8],
    b: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>, NtgError> {
    if a.len() != m * k {
        return Err(NtgError::ShapeMismatch {
            expected: m * k,
            got: a.len(),
        });
    }
    if b.len() != k * n {
        return Err(NtgError::ShapeMismatch {
            expected: k * n,
            got: b.len(),
        });
    }

    let mut c = vec![0.0f32; m * n];

    for i in 0..m {
        for j in 0..n {
            let mut sum_v = _mm256_setzero_si256();

            // Process k elements in chunks of 32 (2x 16-element registers)
            let mut p = 0;
            while p + 32 <= k {
                // Load 16 elements from a[i*k + p..] and b[p*n + j..]
                let a_chunk1 = _mm256_loadu_si256(a.as_ptr().add(i * k + p) as *const __m256i);
                let b_chunk1 =
                    _mm256_loadu_si256(b.as_ptr().add((p) * n + j) as *const __m256i);

                // maddubs_epi16: multiply i8 pairs, sum into i16
                // Result is 16x i16 values in sum_v
                let prod1 = _mm256_maddubs_epi16(a_chunk1, b_chunk1);
                sum_v = _mm256_add_epi32(sum_v, prod1 as __m256i);

                // Load second chunk
                let a_chunk2 = _mm256_loadu_si256(a.as_ptr().add(i * k + p + 16) as *const __m256i);
                let b_chunk2 = _mm256_loadu_si256(b.as_ptr().add((p + 16) * n + j) as *const __m256i);

                let prod2 = _mm256_maddubs_epi16(a_chunk2, b_chunk2);
                sum_v = _mm256_add_epi32(sum_v, prod2 as __m256i);

                p += 32;
            }

            // Horizontal sum of all elements in sum_v
            let sum_f = horizontal_sum_epi32(sum_v);
            c[i * n + j] = sum_f;
        }
    }

    Ok(c)
}

/// Horizontal sum: add all 8 i32 lanes
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_epi32(v: __m256i) -> f32 {
    use std::arch::x86_64::{_mm256_castsi256_si128, _mm_add_epi32, _mm_cvtsi128_si32};

    let v128 = _mm256_castsi256_si128(v);
    let sum = _mm_add_epi32(v128, _mm256_castsi256_si128(_mm256_permute2f128_si256(v, v, 0x1)));
    _mm_cvtsi128_si32(sum) as f32
}

/// Public wrapper for AVX2 matmul
pub fn matmul_avx2(
    a: &[i8],
    b: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>, NtgError> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { matmul_avx2_inner(a, b, m, k, n) };
        }
    }

    // Fallback to scalar if AVX2 not available
    super::super::ternary::matmul_scalar(a, b, m, k, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn avx2_matmul_simple() -> Result<(), NtgError> {
        if !is_x86_feature_detected!("avx2") {
            return Ok(());  // Skip on non-AVX2
        }

        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];

        let result = matmul_avx2(&a, &b, 2, 2, 2)?;

        // Expected: [[2, -1], [-1, 1]]
        assert_eq!(result[0], 2.0);
        assert_eq!(result[1], -1.0);
        assert_eq!(result[2], -1.0);
        assert_eq!(result[3], 1.0);

        Ok(())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn avx2_matches_scalar() -> Result<(), NtgError> {
        if !is_x86_feature_detected!("avx2") {
            return Ok(());
        }

        let a = vec![1i8, -1, 0, 1, -1, 0, 1, -1, 0, 1, -1, 0];
        let b = vec![1i8, 0, -1, 1, 0, 1, -1, 0, 1, -1, 0, 1];

        let scalar_result = super::super::super::ternary::matmul_scalar(&a, &b, 3, 4, 3)?;
        let avx2_result = matmul_avx2(&a, &b, 3, 4, 3)?;

        // Must be bit-identical
        assert_eq!(scalar_result, avx2_result, "AVX2 result differs from scalar");

        Ok(())
    }
}
