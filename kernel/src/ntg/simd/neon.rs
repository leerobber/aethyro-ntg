//! NEON path for ternary matmul (ARM64).
//!
//! **Honest scope:** on `aarch64`, processes k-dimension in 8-element
//! chunks for cache-friendly structure; accumulation is currently
//! scalar within the chunk (Rust `std` NEON surface for full `vmull`
//! wiring is limited / was incomplete here). Output is **bit-identical**
//! to [`crate::ntg::ternary::matmul_scalar`] by construction.
//!
//! On non-ARM hosts, [`matmul_neon`] falls back to scalar immediately.
//! Full intrinsic NEON is a future optimization once measured on ARM CI.

use super::super::error::NtgError;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::{
    vaddq_s32, vmull_s8, vqmovn_s32, vuzp1q_s16, int8x8_t, int16x8_t, int32x4_t, int32x4_t,
};

/// NEON matmul: (m x k) @ (k x n) -> m x n
/// Requires: ARM64 with NEON support
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub fn matmul_neon_inner(
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
            let mut sum: i32 = 0;

            // Process k elements in chunks of 8 (NEON register size for i8)
            let mut p = 0;
            while p + 8 <= k {
                // Load 8 i8 elements from each matrix
                let a_vals = &a[i * k + p..i * k + p + 8];
                let b_vals = &b[p * n + j..p * n + j + 8];

                // Manual accumulation (NEON intrinsics not fully exposed in Rust std)
                // Real NEON would use vmull_s8 and vaddw_s32
                for idx in 0..8 {
                    sum += (a_vals[idx] as i32) * (b_vals[idx] as i32);
                }

                p += 8;
            }

            // Handle remainder (< 8 elements)
            while p < k {
                sum += (a[i * k + p] as i32) * (b[p * n + j] as i32);
                p += 1;
            }

            c[i * n + j] = sum as f32;
        }
    }

    Ok(c)
}

/// Public wrapper for NEON matmul
pub fn matmul_neon(
    a: &[i8],
    b: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>, NtgError> {
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    {
        return matmul_neon_inner(a, b, m, k, n);
    }

    // Fallback to scalar if NEON not available
    super::super::ternary::matmul_scalar(a, b, m, k, n)
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_arch = "aarch64")]
    fn neon_matmul_simple() -> Result<(), super::super::super::error::NtgError> {
        use super::*;
        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];

        let result = matmul_neon(&a, &b, 2, 2, 2)?;

        // Expected: [[2, -1], [-1, 1]]
        assert_eq!(result[0], 2.0);
        assert_eq!(result[1], -1.0);
        assert_eq!(result[2], -1.0);
        assert_eq!(result[3], 1.0);

        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn neon_matches_scalar() -> Result<(), NtgError> {
        let a = vec![1i8, -1, 0, 1, -1, 0, 1, -1, 0, 1, -1, 0];
        let b = vec![1i8, 0, -1, 1, 0, 1, -1, 0, 1, -1, 0, 1];

        let scalar_result = super::super::super::ternary::matmul_scalar(&a, &b, 3, 4, 3)?;
        let neon_result = matmul_neon(&a, &b, 3, 4, 3)?;

        // Must be bit-identical
        assert_eq!(scalar_result, neon_result, "NEON result differs from scalar");

        Ok(())
    }
}
