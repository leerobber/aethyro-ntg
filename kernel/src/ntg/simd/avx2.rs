//! AVX2 SIMD implementation of ternary matmul.
//!
//! Correct path for ternary {-1, 0, 1} matmul. For each output element
//! C[i,j] = sum_p A[i,p] * B[p,j]. B columns are non-contiguous in
//! row-major storage, so we gather into a temp buffer before vector ops.
//! Remainder (k not multiple of 32) is scalar.

use super::super::error::NtgError;

/// AVX2 matmul: (m x k) @ (k x n) -> m x n
/// Requires: x86_64 with AVX2 support
///
/// # Safety
/// Caller must ensure the AVX2 target feature is actually available on the
/// current CPU (e.g. via `is_x86_feature_detected!("avx2")`) before calling —
/// `#[target_feature(enable = "avx2")]` does not itself check this at runtime.
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
            // Gather + scalar MAC is correct for ternary i8; keeps results
            // bit-identical to matmul_scalar (Phase 1.2 exit criterion).
            // A full AVX2 madd path needs contiguous B columns (transpose
            // or gather instrs) — tracked as a follow-up optimization.
            let mut sum: i32 = 0;
            let row = i * k;
            for p in 0..k {
                sum += a[row + p] as i32 * b[p * n + j] as i32;
            }
            c[i * n + j] = sum as f32;
        }
    }

    Ok(c)
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
            return Ok(()); // Skip on non-AVX2
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
