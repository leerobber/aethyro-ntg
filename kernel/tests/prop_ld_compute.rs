//! Property-based tests for LD computation.
//! Validates mathematical properties: idempotency, determinism, bounds.

use ntg_kernel::genomic::{LdComputer, BitstreamGenotypes};
use proptest::prelude::*;

prop_compose! {
    /// Generate a random LD matrix with 10-100 samples and 5-20 SNPs.
    fn arb_ld_data()(
        n_samples in 10usize..100,
        n_snps in 5usize..20,
    ) -> (Vec<BitstreamGenotypes>, Vec<u32>) {
        let mut snps = Vec::new();
        let mut positions = Vec::new();

        for i in 0..n_snps {
            let mut g = BitstreamGenotypes::new(n_samples);
            for s in 0..n_samples {
                let gt = (s as u32).wrapping_mul(7).wrapping_add(i as u32) % 3;
                g.set(s, gt as u8);
            }
            snps.push(g);
            positions.push((i as u32) * 1000 + 100);
        }
        (snps, positions)
    }
}

proptest! {
    #[test]
    fn prop_ld_computation_idempotent(
        (snps, positions) in arb_ld_data()
    ) {
        // Compute LD twice with same params; results should be identical
        let computer1 = LdComputer::new(false, 0.0);
        let matrix1 = computer1.compute_ld(&snps, &positions)
            .expect("compute_ld failed");

        let computer2 = LdComputer::new(false, 0.0);
        let matrix2 = computer2.compute_ld(&snps, &positions)
            .expect("compute_ld failed");

        // Same number of pairs found
        prop_assert_eq!(matrix1.pairs.len(), matrix2.pairs.len(),
            "LD computation is not idempotent");

        // Same r² values (within floating point tolerance)
        for (p1, p2) in matrix1.pairs.iter().zip(matrix2.pairs.iter()) {
            prop_assert_eq!(p1.snp1_idx, p2.snp1_idx);
            prop_assert_eq!(p1.snp2_idx, p2.snp2_idx);
            prop_assert!(
                (p1.r_squared - p2.r_squared).abs() < 1e-6,
                "r² mismatch: {} vs {}",
                p1.r_squared,
                p2.r_squared
            );
        }
    }

    #[test]
    fn prop_ld_scores_within_bounds(
        (snps, positions) in arb_ld_data()
    ) {
        let computer = LdComputer::new(false, 0.0);
        let matrix = computer.compute_ld(&snps, &positions)
            .expect("compute_ld failed");

        for pair in &matrix.pairs {
            // r² must be in (0.0, 1.0]
            prop_assert!(pair.r_squared > 0.0, "r² must be > 0.0, got {}", pair.r_squared);
            prop_assert!(pair.r_squared <= 1.0, "r² must be <= 1.0, got {}", pair.r_squared);
        }
    }

    #[test]
    fn prop_ld_threshold_filtering(
        (snps, positions) in arb_ld_data()
    ) {
        let threshold = 0.5f32;
        let computer = LdComputer::new(false, threshold);
        let matrix = computer.compute_ld(&snps, &positions)
            .expect("compute_ld failed");

        // All pairs should exceed threshold
        for pair in &matrix.pairs {
            prop_assert!(
                pair.r_squared > threshold,
                "Pair ({}, {}) below threshold: r²={} <= {}",
                pair.snp1_idx, pair.snp2_idx, pair.r_squared, threshold
            );
        }
    }
}

#[cfg(test)]
mod deterministic_tests {
    use super::*;

    #[test]
    fn simple_ld_computation() {
        let mut snps = Vec::new();
        let mut positions = Vec::new();

        for i in 0..10 {
            let mut g = BitstreamGenotypes::new(20);
            for s in 0..20 {
                g.set(s, ((s + i) % 3) as u8);
            }
            snps.push(g);
            positions.push((i as u32) * 1000);
        }

        let computer = LdComputer::new(false, 0.0);
        let matrix = computer.compute_ld(&snps, &positions)
            .expect("compute_ld failed");

        // Should find some LD pairs
        assert!(!matrix.pairs.is_empty());

        // All scores should be valid
        for pair in &matrix.pairs {
            assert!(pair.r_squared > 0.0 && pair.r_squared <= 1.0);
        }
    }
}
