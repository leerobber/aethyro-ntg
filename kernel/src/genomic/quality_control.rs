/// Phase D: Quality Control — Statistical Validation
/// Compare synthetic genomes to real 1000 Genomes data
/// Pure Rust, no dependencies

/// Statistics for a single locus
#[derive(Clone, Debug)]
pub struct LocusStats {
    pub snp_id: String,
    pub allele_freq_a: f32,
    pub allele_freq_b: f32,
    pub hardy_weinberg_p: f32,
    pub genotype_counts: (usize, usize, usize), // AA, AB, BB
}

/// Population-level statistics
#[derive(Clone, Debug)]
pub struct PopulationStats {
    pub n_samples: usize,
    pub n_snps: usize,
    pub mean_maf: f32,
    pub mean_he: f32,
    pub mean_pi: f32,
}

impl PopulationStats {
    pub fn from_loci(loci: &[LocusStats]) -> Self {
        if loci.is_empty() {
            return PopulationStats {
                n_samples: 0,
                n_snps: 0,
                mean_maf: 0.0,
                mean_he: 0.0,
                mean_pi: 0.0,
            };
        }

        let n_snps = loci.len();
        let mut total_maf = 0.0;
        let mut total_he = 0.0;
        let mut total_pi = 0.0;

        for locus in loci {
            let p = locus.allele_freq_a;
            let q = 1.0 - p;
            let maf = p.min(q);
            let he = 2.0 * p * q;
            let pi = p * (1.0 - p) + q * (1.0 - q);

            total_maf += maf;
            total_he += he;
            total_pi += pi;
        }

        let (aa, ab, bb) = loci[0].genotype_counts;
        let n_samples = aa + ab + bb;

        PopulationStats {
            n_samples,
            n_snps,
            mean_maf: total_maf / n_snps as f32,
            mean_he: total_he / n_snps as f32,
            mean_pi: total_pi / n_snps as f32,
        }
    }
}

/// Quality control metrics
#[derive(Clone, Debug)]
pub struct QCMetrics {
    pub population_stats: PopulationStats,
    pub missingness_rate: f32,
    pub hwe_violations: usize,
    pub low_maf_count: usize,
    pub mean_ld_r2: f32,
    pub quality_score: f32,
}

impl QCMetrics {
    pub fn new(
        population_stats: PopulationStats,
        hwe_violations: usize,
        low_maf_count: usize,
        mean_ld_r2: f32,
    ) -> Self {
        let missingness_rate = 0.0; // synthetic genomes have no missing data
        let total_snps = population_stats.n_snps.max(1);

        let hwe_penalty = (hwe_violations as f32) / (total_snps as f32);
        let maf_penalty = (low_maf_count as f32) / (total_snps as f32);
        let ld_penalty = (1.0 - mean_ld_r2).abs();

        let quality_score = 1.0 - (0.3 * hwe_penalty + 0.2 * maf_penalty + 0.2 * ld_penalty);

        QCMetrics {
            population_stats,
            missingness_rate,
            hwe_violations,
            low_maf_count,
            mean_ld_r2,
            quality_score: quality_score.max(0.0).min(1.0),
        }
    }
}

/// Validator for synthetic genomes
pub struct GenomeValidator;

impl GenomeValidator {
    /// Compute allele frequency from genotype counts
    pub fn allele_frequency(genotype_counts: (usize, usize, usize)) -> (f32, f32) {
        let (aa, ab, bb) = genotype_counts;
        let total_alleles = (aa + ab + bb) * 2;

        if total_alleles == 0 {
            return (0.5, 0.5);
        }

        let freq_a = (aa * 2 + ab) as f32 / total_alleles as f32;
        let freq_b = 1.0 - freq_a;

        (freq_a, freq_b)
    }

    /// Test Hardy-Weinberg equilibrium
    /// Chi-square test with 1 degree of freedom
    pub fn hardy_weinberg_test(genotype_counts: (usize, usize, usize)) -> f32 {
        let (obs_aa, obs_ab, obs_bb) = genotype_counts;
        let n = obs_aa + obs_ab + obs_bb;

        if n < 2 {
            return 1.0;
        }

        let (p, q) = Self::allele_frequency(genotype_counts);

        let exp_aa = p * p * n as f32;
        let exp_ab = 2.0 * p * q * n as f32;
        let exp_bb = q * q * n as f32;

        let mut chi_sq = 0.0;
        if exp_aa > 0.0 {
            chi_sq += (obs_aa as f32 - exp_aa).powi(2) / exp_aa;
        }
        if exp_ab > 0.0 {
            chi_sq += (obs_ab as f32 - exp_ab).powi(2) / exp_ab;
        }
        if exp_bb > 0.0 {
            chi_sq += (obs_bb as f32 - exp_bb).powi(2) / exp_bb;
        }

        chi_sq_to_p_value(chi_sq)
    }

    /// Validate a single locus
    pub fn validate_locus(
        snp_id: String,
        genotype_counts: (usize, usize, usize),
    ) -> LocusStats {
        let (allele_freq_a, allele_freq_b) = Self::allele_frequency(genotype_counts);
        let hardy_weinberg_p = Self::hardy_weinberg_test(genotype_counts);

        LocusStats {
            snp_id,
            allele_freq_a,
            allele_freq_b,
            hardy_weinberg_p,
            genotype_counts,
        }
    }

    /// Compute LD r² between two SNPs
    pub fn ld_r2(
        genotypes_1: &[(u8, u8)],
        genotypes_2: &[(u8, u8)],
    ) -> f32 {
        if genotypes_1.len() != genotypes_2.len() || genotypes_1.is_empty() {
            return 0.0;
        }

        let n = genotypes_1.len() as f32;
        let mut sum_d = 0.0;
        let mut sum_d_sq = 0.0;

        for (g1, g2) in genotypes_1.iter().zip(genotypes_2.iter()) {
            let allele_1 = (g1.0 + g1.1) as f32;
            let allele_2 = (g2.0 + g2.1) as f32;

            let d = allele_1 * allele_2;
            sum_d += d;
            sum_d_sq += d * d;
        }

        let mean_d = sum_d / n;
        let var_d = (sum_d_sq / n) - (mean_d * mean_d);

        if var_d <= 0.0 {
            return 0.0;
        }

        (var_d / (2.0 * 2.0)).max(0.0).min(1.0)
    }

    /// Generate QC report
    pub fn generate_report(loci: &[LocusStats], mean_ld_r2: f32) -> QCMetrics {
        let hwe_violations = loci.iter().filter(|l| l.hardy_weinberg_p < 0.05).count();
        let low_maf_count = loci.iter().filter(|l| {
            let maf = l.allele_freq_a.min(l.allele_freq_b);
            maf < 0.05
        }).count();

        let population_stats = PopulationStats::from_loci(loci);

        QCMetrics::new(population_stats, hwe_violations, low_maf_count, mean_ld_r2)
    }
}

/// Chi-square CDF approximation (1 degree of freedom)
fn chi_sq_to_p_value(chi_sq: f32) -> f32 {
    if chi_sq <= 0.0 {
        return 1.0;
    }
    if chi_sq >= 10.0 {
        return 0.001;
    }

    let z = chi_sq.sqrt();
    1.0 - erf_approx(z / std::f32::consts::SQRT_2)
}

/// Error function approximation
fn erf_approx(x: f32) -> f32 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0
        - (((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t) * (-x * x).exp();

    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allele_frequency() {
        // (AA, AB, BB) = (100, 50, 0) -> A alleles = 2*100+50 = 250 of 300 total.
        let counts = (100, 50, 0);
        let (p, q) = GenomeValidator::allele_frequency(counts);
        assert!((p - 250.0 / 300.0).abs() < 0.01);
        assert!((q - 50.0 / 300.0).abs() < 0.01);
    }

    #[test]
    fn test_hardy_weinberg() {
        // (AA, AB, BB) = (100, 200, 100), n=400, p=q=0.5: exactly matches HWE
        // expected counts (100, 200, 100), so chi-sq = 0 and p-value = 1.0.
        let counts = (100, 200, 100);
        let p_val = GenomeValidator::hardy_weinberg_test(counts);
        assert!(p_val > 0.05);
    }

    #[test]
    fn test_population_stats() {
        let locus = LocusStats {
            snp_id: "SNP1".to_string(),
            allele_freq_a: 0.5,
            allele_freq_b: 0.5,
            hardy_weinberg_p: 0.5,
            genotype_counts: (50, 50, 50),
        };

        let stats = PopulationStats::from_loci(&[locus]);
        assert_eq!(stats.n_snps, 1);
        assert!(stats.mean_maf > 0.0);
    }
}
