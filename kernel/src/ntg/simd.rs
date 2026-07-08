//! Portable "fast path" for ternary matmul (Phase 1.2).
//!
//! **What this is, precisely, and what it isn't:** this is not
//! hand-written AVX2/NEON intrinsics. Writing those correctly requires
//! `unsafe` code this project has no way to compile or test locally
//! (no local rustc/cargo -- CI round-trips are the only verification
//! available, and they've already caught two real bugs in *safe* code
//! this session). NEON specifically could not even be *compile-checked*
//! by this repo's current CI (a single x86_64 runner) --
//! `#[cfg(target_arch = "aarch64")]` code would simply never build on
//! it, let alone run, which would mean shipping genuinely untested
//! `unsafe` code -- exactly what CONTRIBUTING.md rules 5 and 7 exist to
//! prevent. There's also a real correctness subtlety hand-written SIMD
//! reductions usually hit: IEEE 754 float addition isn't associative,
//! so summing the same terms in a different (e.g. tree-reduced) order
//! can produce a different f32 result than the scalar reference's
//! strictly sequential sum -- and DESIGN.md requires bit-identical
//! output, not "close."
//!
//! What this module actually is: a 100% safe, portable rewrite of the
//! same sequential accumulation, using the *exact* same term order as
//! `ternary::matmul_scalar` (bit-identical by construction, not by
//! luck), restructured with iterator-based inner-loop access instead of
//! manual indexing -- what actually gives LLVM's auto-vectorizer the
//! best chance in release builds (removing bounds-check noise the
//! optimizer doesn't always already elide). **Measured result (real,
//! release-mode, via CI): this is currently ~10% *slower* than
//! `matmul_scalar`, not faster** (see docs/EXPERIMENTS.md for the full
//! numbers) -- the iterator restructuring didn't unlock a measurable
//! auto-vectorization win here. Kept for its correctness/error-handling
//! improvements over the original example this was based on, not
//! claimed as a performance win. Real hand-written intrinsics remain
//! explicitly deferred until there's a local dev environment to verify
//! `unsafe` code safely and a multi-arch CI matrix for NEON.

use super::error::NtgError;

/// Semantically identical to `ternary::matmul_scalar`: same shape
/// checks, same term order, same accumulation.
pub fn matmul_fast(a: &[i8], b: &[i8], m: usize, k: usize, n: usize) -> Result<Vec<f32>, NtgError> {
    if a.len() != m * k {
        return Err(NtgError::ShapeMismatch { expected: m * k, got: a.len() });
    }
    if b.len() != k * n {
        return Err(NtgError::ShapeMismatch { expected: k * n, got: b.len() });
    }
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        let a_row = &a[i * k..(i + 1) * k];
        for j in 0..n {
            let mut sum = 0.0f32;
            for (p, &av) in a_row.iter().enumerate() {
                sum += av as f32 * b[p * n + j] as f32;
            }
            c[i * n + j] = sum;
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::ternary::matmul_scalar;

    #[test]
    fn matches_scalar_reference_bit_for_bit_on_existing_case() {
        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];
        assert_eq!(matmul_fast(&a, &b, 2, 2, 2).unwrap(), matmul_scalar(&a, &b, 2, 2, 2).unwrap());
    }

    #[test]
    fn matches_scalar_reference_on_larger_deterministic_input() {
        // Deterministic pseudo-random ternary data via a simple LCG --
        // not the `rand` crate, to avoid a new dependency for a test
        // fixture. Exercises far more cases than the hand-picked one above.
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 61) % 3) as i8 - 1 // -1, 0, or 1
        };
        let (m, k, n) = (7, 13, 5);
        let a: Vec<i8> = (0..m * k).map(|_| next()).collect();
        let b: Vec<i8> = (0..k * n).map(|_| next()).collect();
        assert_eq!(matmul_fast(&a, &b, m, k, n).unwrap(), matmul_scalar(&a, &b, m, k, n).unwrap());
    }

    #[test]
    fn rejects_shape_mismatch_same_as_scalar() {
        let a = vec![1i8, -1];
        let b = vec![1i8, 0, -1, 1];
        assert!(matmul_fast(&a, &b, 2, 2, 2).is_err());
        assert!(matmul_scalar(&a, &b, 2, 2, 2).is_err());
    }
}
