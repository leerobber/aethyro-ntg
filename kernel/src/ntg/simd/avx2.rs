//! AVX2 SIMD implementation of ternary matmul.
//!
//! Correctness notes (this file previously compiled but computed wrong
//! results -- always zero for the k < 32 case actually exercised by its
//! own tests, and wrong for k >= 32 too). Four separate bugs, all fixed
//! here and pinned down by `avx2_matches_scalar` / `avx2_matmul_simple`:
//!
//! 1. No scalar remainder handling: the loop only processed full 32-wide
//!    chunks and silently dropped everything else, including all of k < 32
//!    (i.e. every case the existing tests actually used) -- output was
//!    unconditionally zero.
//! 2. `_mm256_maddubs_epi16` (VPMADDUBSW) requires its FIRST operand's
//!    bytes to be unsigned and its second operand's signed. Ternary values
//!    are {-1, 0, 1}; passing a raw signed -1 (byte 0xFF) as the first
//!    operand makes the hardware read it as 255, not -1. Fixed by shifting
//!    the first operand's values by +1 (giving {0, 1, 2}, unsigned-safe)
//!    and correcting algebraically: sum(a*b) = sum((a_shifted-1)*b)
//!    = sum(a_shifted*b) - sum(b).
//! 3. B is stored row-major (k x n); a dot product over k needs B[p][j]
//!    for p = 0..k at fixed j, a strided read (stride n) -- not the
//!    contiguous read the old code did. Fixed by transposing B once per
//!    call into a row-major (n x k) buffer so each row is contiguous in k.
//! 4. VPMADDUBSW's i16 output was being accumulated by bit-casting into
//!    i32 lanes via `_mm256_add_epi32`, which does not correctly widen or
//!    pair i16 values. Fixed with `_mm256_madd_epi16` against an all-ones
//!    i16 vector, which correctly pairs and sums into real i32 lanes.
//! 5. The horizontal reduction only recovered 2 of the 8 i32 accumulator
//!    lanes (extracted lane 0 of a partially-reduced 4-lane vector,
//!    discarding the other 3). Fixed with a full lo+hi then two
//!    `_mm256_hadd_epi32` reductions.

use super::super::error::NtgError;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    __m256i, _mm256_add_epi32, _mm256_add_epi8, _mm256_castsi256_si128, _mm256_extracti128_si256,
    _mm256_loadu_si256, _mm256_madd_epi16, _mm256_maddubs_epi16, _mm256_set1_epi16,
    _mm256_set1_epi8, _mm256_setzero_si256, _mm_add_epi32, _mm_cvtsi128_si32, _mm_hadd_epi32,
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

    // Transpose B (k x n) -> b_t (n x k) so each row of b_t is contiguous
    // over the k-dimension, matching A's row layout. O(k*n), done once
    // per call rather than once per output cell.
    let mut b_t = vec![0i8; n * k];
    for p in 0..k {
        for j in 0..n {
            b_t[j * k + p] = b[p * n + j];
        }
    }

    let mut c = vec![0.0f32; m * n];
    let ones_i8 = _mm256_set1_epi8(1);
    let ones_i16 = _mm256_set1_epi16(1);

    for i in 0..m {
        for j in 0..n {
            let mut sum_shifted_v = _mm256_setzero_si256();
            let mut sum_b_v = _mm256_setzero_si256();

            let mut p = 0usize;
            while p + 32 <= k {
                let a_chunk = _mm256_loadu_si256(a.as_ptr().add(i * k + p) as *const __m256i);
                let bt_chunk = _mm256_loadu_si256(b_t.as_ptr().add(j * k + p) as *const __m256i);

                // Shift a's ternary values {-1,0,1} -> {0,1,2}: safely
                // unsigned for VPMADDUBSW's first-operand requirement.
                let a_shifted = _mm256_add_epi8(a_chunk, ones_i8);

                // sum(a_shifted * b): maddubs pairwise-multiplies into i16,
                // then madd_epi16 against all-ones widens/pairs into i32.
                let prod16 = _mm256_maddubs_epi16(a_shifted, bt_chunk);
                let prod32 = _mm256_madd_epi16(prod16, ones_i16);
                sum_shifted_v = _mm256_add_epi32(sum_shifted_v, prod32);

                // sum(b): same trick with an all-ones unsigned first
                // operand (1 * b[i] = b[i]) to get the correction term.
                let prod_b16 = _mm256_maddubs_epi16(ones_i8, bt_chunk);
                let prod_b32 = _mm256_madd_epi16(prod_b16, ones_i16);
                sum_b_v = _mm256_add_epi32(sum_b_v, prod_b32);

                p += 32;
            }

            let total_shifted = horizontal_sum_epi32(sum_shifted_v);
            let total_b = horizontal_sum_epi32(sum_b_v);
            let mut total = total_shifted - total_b;

            // Scalar tail for any remainder -- including all of k < 32,
            // which is every case the unit tests below actually exercise.
            while p < k {
                total += a[i * k + p] as i32 * b_t[j * k + p] as i32;
                p += 1;
            }

            c[i * n + j] = total as f32;
        }
    }

    Ok(c)
}

/// Horizontal sum: reduce all 8 i32 lanes to one scalar (not just 2 of 8).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_epi32(v: __m256i) -> i32 {
    let lo = _mm256_castsi256_si128(v);
    let hi = _mm256_extracti128_si256(v, 1);
    let sum128 = _mm_add_epi32(lo, hi); // 4 lanes: [0+4, 1+5, 2+6, 3+7]
    let sum64 = _mm_hadd_epi32(sum128, sum128); // [(0+4)+(1+5), (2+6)+(3+7), ..]
    let sum32 = _mm_hadd_epi32(sum64, sum64); // [total, total, total, total]
    _mm_cvtsi128_si32(sum32)
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

    /// The two tests above both use k < 32, so they never execute the
    /// vectorized 32-wide chunk loop at all -- only the scalar tail path.
    /// This test uses k values that are exact multiples of 32, multiples
    /// of 32 plus a remainder, and non-multiples, with randomized ternary
    /// inputs, specifically to exercise and validate the SIMD path itself.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn avx2_matches_scalar_for_k_at_and_above_simd_width() -> Result<(), NtgError> {
        if !is_x86_feature_detected!("avx2") {
            return Ok(());
        }

        // Deterministic xorshift so failures are reproducible without
        // pulling in a dependency on `rand`.
        let mut state: u64 = 0xD1CE_5EED_C0FF_EE01;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut rand_ternary = |len: usize| -> Vec<i8> {
            (0..len)
                .map(|_| match next() % 3 {
                    0 => -1i8,
                    1 => 0i8,
                    _ => 1i8,
                })
                .collect()
        };

        for &k in &[32usize, 33, 63, 64, 65, 96, 100, 128] {
            for &(m, n) in &[(1usize, 1usize), (2, 3), (5, 5), (1, 7)] {
                let a = rand_ternary(m * k);
                let b = rand_ternary(k * n);

                let scalar_result = super::super::super::ternary::matmul_scalar(&a, &b, m, k, n)?;
                let avx2_result = matmul_avx2(&a, &b, m, k, n)?;

                assert_eq!(
                    scalar_result, avx2_result,
                    "AVX2 result differs from scalar at m={m} k={k} n={n}"
                );
            }
        }

        Ok(())
    }
}
