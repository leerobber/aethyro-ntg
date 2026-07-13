/// Linkage Disequilibrium (LD) Computation
/// Computes pairwise r² between SNPs with streaming memory efficiency
/// Target: 201K SNPs/sec, ~1.3M high-LD pairs per chromosome

use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct LdPair {
    pub snp1_idx: u32,
    pub snp2_idx: u32,
    pub r_squared: f32,
    pub position1: u32,
    pub position2: u32,
}

#[derive(Debug)]
pub struct LdMatrix {
    pub pairs: Vec<LdPair>,
    pub n_snps: usize,
    pub threshold: f32,
}

pub struct LdComputer {
    verbose: bool,
    threshold: f32,  // Only keep pairs with r² > threshold
}

impl LdComputer {
    pub fn new(verbose: bool, threshold: f32) -> Self {
        LdComputer { verbose, threshold }
    }

    /// Compute LD matrix for genotypes
    /// Uses sliding window approach to avoid O(n²) computation
    /// Window size: 500 SNPs (typical LD decay distance)
    pub fn compute_ld(
        &self,
        genotypes: &[BitstreamGenotypes],
        positions: &[u32],
    ) -> Result<LdMatrix, String> {
        if genotypes.is_empty() || positions.is_empty() {
            return Err("Empty genotype or position data".to_string());
        }

        if genotypes.len() != positions.len() {
            return Err(format!(
                "Genotype count {} != position count {}",
                genotypes.len(),
                positions.len()
            ));
        }

        let n_snps = genotypes.len();
        let mut pairs = Vec::new();

        if self.verbose {
            println!("[*] Computing LD matrix for {} SNPs", n_snps);
            println!("[*] Threshold: r² > {}", self.threshold);
        }

        let start = Instant::now();
        let mut last_progress = Instant::now();
        let mut pairs_computed = 0u64;
        let mut pairs_kept = 0u64;

        // Sliding window: for each SNP, compute correlation with nearby SNPs
        let window_size = 500;  // SNPs within 500bp show LD

        for i in 0..n_snps {
            // Compute correlation with SNPs in forward window
            let window_end = std::cmp::min(i + window_size, n_snps);

            for j in (i + 1)..window_end {
                pairs_computed += 1;

                // Compute r² between SNP i and SNP j
                match self.compute_r_squared(&genotypes[i], &genotypes[j]) {
                    Some(r_sq) if r_sq > self.threshold => {
                        pairs_kept += 1;
                        pairs.push(LdPair {
                            snp1_idx: i as u32,
                            snp2_idx: j as u32,
                            r_squared: r_sq,
                            position1: positions[i],
                            position2: positions[j],
                        });
                    }
                    _ => {}
                }
            }

            // Progress reporting
            if self.verbose && last_progress.elapsed().as_secs() >= 5 {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = i as f64 / elapsed;
                println!(
                    "  [Progress] {} SNPs processed ({:.0} SNPs/sec), {} LD pairs found",
                    i, rate, pairs_kept
                );
                last_progress = Instant::now();
            }
        }

        if self.verbose {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = n_snps as f64 / elapsed;
            let ratio = (pairs_kept as f64 / pairs_computed as f64) * 100.0;
            println!(
                "[OK] Computed {} LD pairs in {:.1}s ({:.0} SNPs/sec)",
                pairs_kept, elapsed, rate
            );
            println!(
                "[*] Pairs computed: {}, kept: {} ({:.2}%)",
                pairs_computed, pairs_kept, ratio
            );
            println!(
                "[*] Data reduction: {}× (only high-LD pairs kept)",
                pairs_computed / (pairs_kept + 1)
            );
        }

        Ok(LdMatrix {
            pairs,
            n_snps,
            threshold: self.threshold,
        })
    }

    /// Compute r² (squared Pearson correlation) between genotype dosages
    /// (0/1/2 alt-allele count per sample, 3=missing/excluded) for two
    /// SNPs. This is the standard genotypic r², equal in expectation to
    /// haplotype-level r² under random mating, and matches
    /// `synthesis::Genome::ld_r2`'s approach.
    ///
    /// A prior version derived (n_00, n_01, n_10, n_11) counts via
    /// `genotype & 1`, which collapses genotype 2 (homozygous alt) to the
    /// same bit as genotype 0 (homozygous ref) -- every homozygous-alt
    /// sample was silently counted as if it carried no alt allele at all.
    /// That inflated or distorted every r² this LD computer ever produced
    /// for a SNP with any homozygous-alt carriers (i.e. essentially every
    /// common SNP), and went undetected because the one test exercising
    /// it used `if let Some(r_sq) = ...`, which silently skips the
    /// assertion when the buggy formula happened to return `None` (which
    /// it did for that test's exact input) instead of failing.
    ///
    /// Delegates to `BitstreamGenotypes::pearson_r2_bitparallel`, which
    /// computes the identical statistic word-parallel over the packed bit
    /// planes (32 samples per iteration via popcount) rather than one
    /// `get()` per sample. `compute_r_squared_scalar` below preserves the
    /// original per-sample reference and exists so a regression test can
    /// assert the two agree.
    fn compute_r_squared(
        &self,
        geno1: &BitstreamGenotypes,
        geno2: &BitstreamGenotypes,
    ) -> Option<f32> {
        geno1.pearson_r2_bitparallel(geno2, 10)
    }

    /// Original scalar, per-sample reference for genotypic r². Retained
    /// only as the correctness oracle for the word-parallel path in
    /// `compute_r_squared`; not on the hot path.
    #[cfg(test)]
    fn compute_r_squared_scalar(
        geno1: &BitstreamGenotypes,
        geno2: &BitstreamGenotypes,
    ) -> Option<f32> {
        if geno1.len() != geno2.len() {
            return None;
        }

        let n = geno1.len();
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_x2 = 0.0f64;
        let mut sum_y2 = 0.0f64;
        let mut valid = 0.0f64;

        for i in 0..n {
            let g1 = geno1.get(i);
            let g2 = geno2.get(i);
            if g1 == 3 || g2 == 3 {
                continue; // missing
            }

            let x = g1 as f64;
            let y = g2 as f64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
            sum_y2 += y * y;
            valid += 1.0;
        }

        if valid < 10.0 {
            // Too few samples for reliable correlation
            return None;
        }

        let num = valid * sum_xy - sum_x * sum_y;
        let den = ((valid * sum_x2 - sum_x * sum_x) * (valid * sum_y2 - sum_y * sum_y)).sqrt();

        if den <= 0.0 {
            // No variance at one (or both) loci in this sample -- r² is
            // undefined, not zero.
            return None;
        }

        let r = num / den;
        Some(((r * r) as f32).max(0.0).min(1.0))
    }
}

impl LdMatrix {
    /// Get summary statistics
    pub fn summary(&self) -> String {
        if self.pairs.is_empty() {
            return "No high-LD pairs found".to_string();
        }

        let mean_r_sq: f64 = self.pairs.iter().map(|p| p.r_squared as f64).sum::<f64>()
            / self.pairs.len() as f64;

        let min_r_sq = self.pairs.iter().map(|p| p.r_squared).fold(1.0, f32::min);
        let max_r_sq = self.pairs.iter().map(|p| p.r_squared).fold(0.0, f32::max);

        format!(
            "LD Matrix: {} SNPs, {} high-LD pairs (r² > {:.3})\n  Mean r²: {:.4}, Min: {:.4}, Max: {:.4}",
            self.n_snps,
            self.pairs.len(),
            self.threshold,
            mean_r_sq,
            min_r_sq,
            max_r_sq
        )
    }

    /// Analyze LD decay by distance
    pub fn ld_decay_analysis(&self) -> String {
        if self.pairs.is_empty() {
            return "No pairs to analyze".to_string();
        }

        // Bin pairs by distance
        let mut bins: std::collections::HashMap<u32, (f32, u32)> = std::collections::HashMap::new();
        let bin_size = 1000;  // 1kb bins

        for pair in &self.pairs {
            let distance = (pair.position2 as i32 - pair.position1 as i32).abs() as u32;
            let bin = (distance / bin_size) * bin_size;

            let entry = bins.entry(bin).or_insert((0.0, 0));
            entry.0 += pair.r_squared;
            entry.1 += 1;
        }

        let mut report = String::from("LD Decay by Distance:\n");
        let mut distances: Vec<u32> = bins.keys().copied().collect();
        distances.sort();

        for dist in distances.iter().take(10) {
            if let Some((sum, count)) = bins.get(dist) {
                let mean_r_sq = sum / (*count as f32);
                report.push_str(&format!(
                    "  {}kb-{}kb: {:.4} (n={})\n",
                    dist / 1000,
                    (dist + bin_size) / 1000,
                    mean_r_sq,
                    count
                ));
            }
        }

        report
    }

    /// Export pairs to CSV format
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("snp1_idx,snp2_idx,r_squared,position1,position2,distance\n");

        for pair in &self.pairs {
            let distance = (pair.position2 as i32 - pair.position1 as i32).abs() as u32;
            csv.push_str(&format!(
                "{},{},{:.6},{},{},{}\n",
                pair.snp1_idx, pair.snp2_idx, pair.r_squared, pair.position1, pair.position2,
                distance
            ));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ld_computation_perfect_correlation() {
        // Create test genotypes
        let mut geno1 = BitstreamGenotypes::new(100);
        let mut geno2 = BitstreamGenotypes::new(100);

        // Set up perfect LD (r² = 1.0): identical dosage pattern at both loci.
        for i in 0..100 {
            let gt = if i % 2 == 0 { 0 } else { 2 };
            geno1.set(i, gt);
            geno2.set(i, gt);
        }

        let computer = LdComputer::new(false, 0.5);
        let r_sq = computer
            .compute_r_squared(&geno1, &geno2)
            .expect("perfectly correlated dosages must yield Some(r_sq), not None");
        assert!(r_sq > 0.9, "Expected r² > 0.9, got {}", r_sq);
    }

    /// Regression test for the genotype & 1 bug: homozygous alt (2) was
    /// bitwise-collapsed to the same value as homozygous ref (0), so a
    /// SNP with homozygous-alt carriers had its correlation with another
    /// SNP silently mismeasured. This constructs a case where the
    /// dosages are ANTI-correlated (one locus high when the other is
    /// low) using genotype value 2 specifically, which the old
    /// `g & 1`-based formula could not distinguish from genotype 0.
    #[test]
    fn test_ld_computation_distinguishes_homozygous_alt_from_homozygous_ref() {
        let mut geno1 = BitstreamGenotypes::new(60);
        let mut geno2 = BitstreamGenotypes::new(60);

        for i in 0..60 {
            if i < 30 {
                geno1.set(i, 2); // homozygous alt
                geno2.set(i, 0); // homozygous ref
            } else {
                geno1.set(i, 0);
                geno2.set(i, 2);
            }
        }

        let computer = LdComputer::new(false, 0.5);
        let r_sq = computer
            .compute_r_squared(&geno1, &geno2)
            .expect("perfectly anti-correlated dosages must yield Some(r_sq)");

        // Perfect anti-correlation still gives r² near 1.0 (r² is
        // sign-blind). The old buggy formula, given only genotypes 0 and
        // 2, saw every sample as (0, 0) after `& 1` and returned None.
        assert!(r_sq > 0.9, "Expected r² > 0.9 for perfect anti-correlation, got {}", r_sq);
    }

    #[test]
    fn test_ld_computation_uncorrelated_dosages_score_low() {
        let mut geno1 = BitstreamGenotypes::new(60);
        let mut geno2 = BitstreamGenotypes::new(60);

        // Alternating 0/1/2 vs. a fixed unrelated pattern with no
        // consistent relationship to geno1.
        let pattern2 = [0u8, 2, 1, 1, 0, 2, 2, 0, 1, 0];
        for i in 0..60 {
            geno1.set(i, (i % 3) as u8);
            geno2.set(i, pattern2[i % pattern2.len()]);
        }

        let computer = LdComputer::new(false, 0.5);
        let r_sq = computer.compute_r_squared(&geno1, &geno2).unwrap_or(0.0);
        assert!(r_sq < 0.3, "Expected low r² for unrelated dosage patterns, got {}", r_sq);
    }

    #[test]
    fn test_ld_computation_too_few_samples_returns_none() {
        let mut geno1 = BitstreamGenotypes::new(5);
        let mut geno2 = BitstreamGenotypes::new(5);
        for i in 0..5 {
            geno1.set(i, (i % 2) as u8);
            geno2.set(i, (i % 2) as u8);
        }

        let computer = LdComputer::new(false, 0.5);
        assert!(computer.compute_r_squared(&geno1, &geno2).is_none());
    }

    /// The word-parallel `pearson_r2_bitparallel` (the hot path) must
    /// agree with the original scalar per-sample reference across a wide
    /// range of inputs -- including missing genotypes, monomorphic loci,
    /// and sample counts that leave a partially-filled final plane word
    /// (the padding-bit case the tail mask exists for). Uses a small
    /// deterministic LCG so the test is reproducible without a dep.
    #[test]
    fn test_bitparallel_r2_matches_scalar_reference() {
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };

        // Exercise several sample counts, incl. non-multiples of 32 so the
        // final word is partially filled with padding bits.
        for &n_samples in &[10usize, 31, 32, 33, 100, 257, 2504] {
            for _pair in 0..40 {
                let mut g1 = BitstreamGenotypes::new(n_samples);
                let mut g2 = BitstreamGenotypes::new(n_samples);
                for s in 0..n_samples {
                    // Genotypes 0/1/2 mostly, with occasional missing (3).
                    let pick = |v: u64| -> u8 {
                        match v % 10 {
                            0 => 3, // ~10% missing
                            1..=3 => 2,
                            4..=6 => 1,
                            _ => 0,
                        }
                    };
                    g1.set(s, pick(next()));
                    g2.set(s, pick(next()));
                }

                let fast = g1.pearson_r2_bitparallel(&g2, 10);
                let slow = LdComputer::compute_r_squared_scalar(&g1, &g2);

                match (fast, slow) {
                    (Some(a), Some(b)) => assert!(
                        (a - b).abs() < 1e-5,
                        "n={} fast={} scalar={} differ beyond f32 tolerance",
                        n_samples, a, b
                    ),
                    (None, None) => {}
                    (f, s) => panic!(
                        "n={}: Some/None disagreement fast={:?} scalar={:?}",
                        n_samples, f, s
                    ),
                }
            }
        }
    }

    /// Padding bits in the final word (samples beyond `n_samples`) are all
    /// zero in both planes, i.e. would read as genotype 0 (valid ref/ref).
    /// Without the tail mask they would inflate the valid-sample count and
    /// the moments. This pins that they are excluded: two SNPs whose only
    /// real sample is far below the word boundary must give the same r² as
    /// the scalar path, not a padding-contaminated one.
    #[test]
    fn test_bitparallel_r2_excludes_padding_samples() {
        // 33 samples => second word holds sample 32 plus 31 padding bits.
        let mut g1 = BitstreamGenotypes::new(33);
        let mut g2 = BitstreamGenotypes::new(33);
        for s in 0..33 {
            let gt = if s % 2 == 0 { 0 } else { 2 };
            g1.set(s, gt);
            g2.set(s, gt);
        }
        let fast = g1.pearson_r2_bitparallel(&g2, 10).unwrap();
        let slow = LdComputer::compute_r_squared_scalar(&g1, &g2).unwrap();
        assert!((fast - slow).abs() < 1e-5, "fast={} scalar={}", fast, slow);
        assert!(fast > 0.9, "perfectly correlated dosages, got {}", fast);
    }

    #[test]
    fn test_ld_matrix_summary() {
        let pairs = vec![
            LdPair {
                snp1_idx: 0,
                snp2_idx: 1,
                r_squared: 0.8,
                position1: 1000,
                position2: 2000,
            },
            LdPair {
                snp1_idx: 1,
                snp2_idx: 2,
                r_squared: 0.6,
                position1: 2000,
                position2: 3000,
            },
        ];

        let matrix = LdMatrix {
            pairs,
            n_snps: 100,
            threshold: 0.5,
        };

        let summary = matrix.summary();
        assert!(summary.contains("2 high-LD pairs"));
    }
}
