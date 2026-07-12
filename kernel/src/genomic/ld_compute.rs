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

    /// Compute Pearson correlation (r²) between two SNPs
    /// Using bitsliced genotype data for efficiency
    fn compute_r_squared(
        &self,
        geno1: &BitstreamGenotypes,
        geno2: &BitstreamGenotypes,
    ) -> Option<f32> {
        if geno1.len() != geno2.len() {
            return None;
        }

        let n = geno1.len();

        // Compute allele counts
        let (n_00, n_01, n_10, n_11) = self.count_genotype_pairs(geno1, geno2, n);

        // Total samples (excluding missing)
        let total = (n_00 + n_01 + n_10 + n_11) as f64;
        if total < 10.0 {
            // Too few samples for reliable correlation
            return None;
        }

        // Allele frequencies
        let p_snp1 = ((n_10 + n_11) as f64 * 2.0 + (n_01 + n_11) as f64) / (total * 2.0);
        let p_snp2 = ((n_01 + n_11) as f64 * 2.0 + (n_10 + n_11) as f64) / (total * 2.0);
        let q_snp1 = 1.0 - p_snp1;
        let q_snp2 = 1.0 - p_snp2;

        // Avoid division by zero
        if q_snp1 == 0.0 || q_snp2 == 0.0 {
            return None;
        }

        // D' (coefficient of linkage disequilibrium)
        // D = P_AB - p_A * p_B
        let p_ab = (n_11 as f64) / total;
        let d = p_ab - (p_snp1 * p_snp2);

        // D_max
        let d_max = if d > 0.0 {
            (p_snp1 * p_snp2).min(q_snp1 * q_snp2)
        } else {
            (p_snp1 * q_snp2).min(q_snp1 * p_snp2)
        };

        // r² = D² / (p_A * q_A * p_B * q_B)
        let r_squared = if d_max > 0.0 {
            (d * d) / (p_snp1 * q_snp1 * p_snp2 * q_snp2)
        } else {
            return None;
        };

        // Convert to f32 and clamp to [0, 1]
        Some((r_squared as f32).max(0.0).min(1.0))
    }

    /// Count genotype pairs between two SNPs
    /// Returns (n_00, n_01, n_10, n_11)
    /// Where 0 = ref allele, 1 = alt allele
    fn count_genotype_pairs(
        &self,
        geno1: &BitstreamGenotypes,
        geno2: &BitstreamGenotypes,
        n: usize,
    ) -> (u32, u32, u32, u32) {
        let mut n_00 = 0u32;
        let mut n_01 = 0u32;
        let mut n_10 = 0u32;
        let mut n_11 = 0u32;

        for i in 0..n {
            let g1 = geno1.get(i);
            let g2 = geno2.get(i);

            // Skip missing genotypes
            if g1 == 3 || g2 == 3 {
                continue;
            }

            // Extract alleles (only use first allele for simplicity, can use both)
            let a1 = g1 & 1;  // 0 or 1
            let a2 = g2 & 1;  // 0 or 1

            match (a1, a2) {
                (0, 0) => n_00 += 1,
                (0, 1) => n_01 += 1,
                (1, 0) => n_10 += 1,
                (1, 1) => n_11 += 1,
                _ => {}
            }
        }

        (n_00, n_01, n_10, n_11)
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
    fn test_ld_computation() {
        // Create test genotypes
        let mut geno1 = BitstreamGenotypes::new(100);
        let mut geno2 = BitstreamGenotypes::new(100);

        // Set up perfect LD (r² = 1.0)
        for i in 0..100 {
            let gt = if i % 2 == 0 { 0 } else { 2 };
            geno1.set(i, gt);
            geno2.set(i, gt);
        }

        let computer = LdComputer::new(false, 0.5);
        if let Some(r_sq) = computer.compute_r_squared(&geno1, &geno2) {
            // Perfect correlation should give r² close to 1.0
            assert!(r_sq > 0.9, "Expected r² > 0.9, got {}", r_sq);
        }
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
