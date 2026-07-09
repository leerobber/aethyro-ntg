//! Scalar reference implementation of ternary tensor primitives.
//!
//! Weights are constrained to {-1, 0, +1} -- the same quantization shape
//! as BitNet b1.58 (see docs/LITERATURE.md). This module is the portable,
//! pure-function baseline: no SIMD, no unsafe, no RNG. Determinism here
//! is what makes replay-safety (ADR 0002) possible at every layer above
//! it -- every faster path added later (SIMD, FFI-exposed) must match
//! this module bit-for-bit in tests, not just "be close."

use super::error::NtgError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ternary {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Ternary {
    pub const fn from_i8(v: i8) -> Result<Self, NtgError> {
        match v {
            -1 => Ok(Self::Neg),
            0 => Ok(Self::Zero),
            1 => Ok(Self::Pos),
            other => Err(NtgError::InvalidTernaryValue(other)),
        }
    }

    pub const fn to_i8(self) -> i8 {
        self as i8
    }
}

/// Encode f32 weights to ternary using an absmean-style threshold -- the
/// same shape of quantization BitNet b1.58 uses (scale the cutoff by the
/// tensor's own average magnitude, not a fixed constant).
pub fn encode(weights: &[f32]) -> Vec<i8> {
    let mean_abs = if weights.is_empty() {
        0.0
    } else {
        weights.iter().map(|w| w.abs()).sum::<f32>() / weights.len() as f32
    };
    let threshold = mean_abs * 0.5;
    weights
        .iter()
        .map(|&w| {
            if w > threshold {
                1
            } else if w < -threshold {
                -1
            } else {
                0
            }
        })
        .collect()
}

/// Fixed-threshold constants for `encode_fixed`, empirically validated
/// (see docs/EXPERIMENTS.md 2026-07-08 follow-up): centered on the a-z
/// byte range since that's where most English text mass sits. This
/// gives 'a' and 'z' *opposite* ternary values, which is the whole
/// point -- see `encode_fixed`'s doc comment.
const FIXED_CENTER: f32 = 109.5;
const FIXED_SCALE: f32 = 13.0;
const FIXED_THRESHOLD: f32 = 0.33;

/// Fixed-threshold, cross-string-comparable byte-to-ternary encoding.
///
/// `encode()` above thresholds each input against *its own* mean
/// magnitude -- correct for BitNet-style single-tensor quantization,
/// but this makes any two strings with similar byte-value profiles
/// collapse to the identical ternary pattern (e.g. "aaaaa" and "zzzzz"
/// both encode to all `-1` under `encode()` -- a real failure,
/// diagnosed in docs/EXPERIMENTS.md, not a hypothetical one).
/// `encode_fixed` uses a fixed global threshold instead, so a given
/// byte always maps to the same ternary value regardless of what
/// string it appears in, which is what makes
/// `Graph::edge_interaction_score` (in `interaction.rs`) meaningful.
///
/// This is **not a balanced quantization scheme** -- the distribution
/// over the full printable-ASCII range is skewed (mostly `-1`; see
/// docs/EXPERIMENTS.md), because it's centered for lowercase-letter
/// distinguishability, not for even bit-usage. Don't use this as a
/// drop-in replacement for `encode()`'s quantization use case.
pub fn encode_fixed(text: &str) -> Vec<i8> {
    text.bytes()
        .map(|b| {
            let v = (b as f32 - FIXED_CENTER) / FIXED_SCALE;
            if v > FIXED_THRESHOLD {
                1
            } else if v < -FIXED_THRESHOLD {
                -1
            } else {
                0
            }
        })
        .collect()
}

/// Scalar reference matmul: (m x k) @ (k x n) -> m x n, accumulated in
/// f32. This is the golden reference every faster path must match.
pub fn matmul_scalar(
    a: &[i8],
    b: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>, NtgError> {
    if a.len() != m * k {
        return Err(NtgError::ShapeMismatch { expected: m * k, got: a.len() });
    }
    if b.len() != k * n {
        return Err(NtgError::ShapeMismatch { expected: k * n, got: b.len() });
    }
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f32;
            for p in 0..k {
                sum += a[i * k + p] as f32 * b[p * n + j] as f32;
            }
            c[i * n + j] = sum;
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ternary_roundtrip() {
        for v in [-1i8, 0, 1] {
            let t = Ternary::from_i8(v).unwrap();
            assert_eq!(t.to_i8(), v);
        }
    }

    #[test]
    fn ternary_rejects_invalid() {
        assert!(Ternary::from_i8(2).is_err());
        assert!(Ternary::from_i8(-5).is_err());
    }

    #[test]
    fn encode_separates_by_threshold() {
        let w = vec![-1.0, -0.05, 0.0, 0.05, 1.0];
        let t = encode(&w);
        assert_eq!(t[0], -1);
        assert_eq!(t[4], 1);
        assert_eq!(t[2], 0);
    }

    #[test]
    fn encode_empty_is_empty() {
        let t = encode(&[]);
        assert!(t.is_empty());
    }

    #[test]
    fn matmul_zero_vector_is_zero() {
        let a = vec![0i8; 4];
        let b = vec![1i8, -1, 0, 1];
        let out = matmul_scalar(&a, &b, 2, 2, 2).unwrap();
        assert!(out.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn matmul_matches_hand_computed_reference() {
        // a = [[1,-1],[0,1]], b = [[1,0],[-1,1]]
        // c[0][0] = 1*1 + (-1)*(-1) = 2
        // c[0][1] = 1*0 + (-1)*1   = -1
        // c[1][0] = 0*1 + 1*(-1)   = -1
        // c[1][1] = 0*0 + 1*1     = 1
        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];
        let out = matmul_scalar(&a, &b, 2, 2, 2).unwrap();
        assert_eq!(out, vec![2.0, -1.0, -1.0, 1.0]);
    }

    #[test]
    fn encode_fixed_distinguishes_the_exact_failure_case() {
        // "aaaaa" and "zzzzz" collapsed to the identical pattern under
        // encode()'s per-string threshold (see docs/EXPERIMENTS.md).
        // encode_fixed must not repeat that failure.
        assert_ne!(encode_fixed("aaaaa"), encode_fixed("zzzzz"));
        assert_eq!(encode_fixed("aaaaa"), vec![-1i8; 5]);
        assert_eq!(encode_fixed("zzzzz"), vec![1i8; 5]);
    }

    #[test]
    fn encode_fixed_is_context_independent() {
        // Unlike encode(), the same byte must encode the same way
        // regardless of what string it's in.
        let a1 = encode_fixed("a");
        let a2 = encode_fixed("aaaa");
        assert_eq!(a1[0], a2[0]);
        assert_eq!(a1[0], a2[3]);
    }

    #[test]
    fn matmul_rejects_shape_mismatch() {
        let a = vec![1i8, -1];
        let b = vec![1i8, 0, -1, 1];
        assert!(matmul_scalar(&a, &b, 2, 2, 2).is_err());
    }
}
