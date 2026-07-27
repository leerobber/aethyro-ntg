//! Phase D: Validation — Compare Synthetic vs Real Genomes
//! Statistical power analysis and distribution matching
//! Pure Rust implementation

use std::collections::HashMap;

/// Reference genome statistics (1000 Genomes)
#[derive(Clone, Debug)]
pub struct ReferenceGenome {
    pub population: String,
    pub n_samples: usize,
    pub allele_frequencies: HashMap<String, (f32, f32)>, // SNP ID -> (freq_A, freq_B)
    pub ld_matrix: HashMap<(String, String), f32>,       // (SNP1, SNP2) -> r²
    pub mean_ld_r2: f32,
}

impl ReferenceGenome {
    pub fn new(population: String, n_samples: usize) -> Self {
        ReferenceGenome {
            population,
            n_samples,
            allele_frequencies: HashMap::new(),
            ld_matrix: HashMap::new(),
            mean_ld_r2: 0.0,
        }
    }

    pub fn add_snp(&mut self, snp_id: String, freq_a: f32, freq_b: f32) {
        self.allele_frequencies.insert(snp_id, (freq_a, freq_b));
    }

    pub fn add_ld_pair(&mut self, snp1: String, snp2: String, r2: f32) {
        self.ld_matrix.insert((snp1.clone(), snp2.clone()), r2);
        self.ld_matrix.insert((snp2, snp1), r2);
    }

    pub fn finalize(&mut self) {
        if !self.ld_matrix.is_empty() {
            let sum: f32 = self.ld_matrix.values().sum();
            self.mean_ld_r2 = sum / self.ld_matrix.len() as f32;
        }
    }
}

/// Synthetic genome statistics
#[derive(Clone, Debug)]
pub struct SyntheticGenome {
    pub allele_frequencies: HashMap<String, (f32, f32)>,
    pub ld_matrix: HashMap<(String, String), f32>,
    pub mean_ld_r2: f32,
    pub n_samples: usize,
}

impl SyntheticGenome {
    pub fn new(n_samples: usize) -> Self {
        SyntheticGenome {
            allele_frequencies: HashMap::new(),
            ld_matrix: HashMap::new(),
            mean_ld_r2: 0.0,
            n_samples,
        }
    }

    pub fn add_snp(&mut self, snp_id: String, freq_a: f32, freq_b: f32) {
        self.allele_frequencies.insert(snp_id, (freq_a, freq_b));
    }

    pub fn add_ld_pair(&mut self, snp1: String, snp2: String, r2: f32) {
        self.ld_matrix.insert((snp1.clone(), snp2.clone()), r2);
        self.ld_matrix.insert((snp2, snp1), r2);
    }

    pub fn finalize(&mut self) {
        if !self.ld_matrix.is_empty() {
            let sum: f32 = self.ld_matrix.values().sum();
            self.mean_ld_r2 = sum / self.ld_matrix.len() as f32;
        }
    }
}

/// Comparison results
#[derive(Clone, Debug)]
pub struct ValidationResults {
    pub ref_population: String,
    pub allele_freq_distance: f32,
    pub ld_distance: f32,
    pub overall_similarity: f32,
    pub n_snps_compared: usize,
    pub allele_freq_rmse: f32,
    pub ld_pearson_r: f32,
}

impl ValidationResults {
    pub fn summary_string(&self) -> String {
        format!(
            "Validation Report: {} vs Synthetic\n\
             SNPs compared: {}\n\
             Allele frequency RMSE: {:.4}\n\
             LD r² distance: {:.4}\n\
             LD Pearson r: {:.4}\n\
             Overall similarity score: {:.4}",
            self.ref_population,
            self.n_snps_compared,
            self.allele_freq_rmse,
            self.ld_distance,
            self.ld_pearson_r,
            self.overall_similarity
        )
    }
}

/// Validator for comparing synthetic to reference genomes
pub struct GenomeComparator;

impl GenomeComparator {
    /// Compare allele frequencies
    pub fn compare_allele_frequencies(
        reference: &ReferenceGenome,
        synthetic: &SyntheticGenome,
    ) -> (f32, usize) {
        let mut sum_sq_diff = 0.0;
        let mut count = 0;

        for (snp_id, (ref_freq_a, ref_freq_b)) in &reference.allele_frequencies {
            if let Some((syn_freq_a, syn_freq_b)) = synthetic.allele_frequencies.get(snp_id) {
                let diff_a = ref_freq_a - syn_freq_a;
                let diff_b = ref_freq_b - syn_freq_b;
                sum_sq_diff += diff_a * diff_a + diff_b * diff_b;
                count += 1;
            }
        }

        if count == 0 {
            return (0.0, 0);
        }

        let rmse = (sum_sq_diff / (count as f32 * 2.0)).sqrt();
        (rmse, count)
    }

    /// Compare LD structures using Pearson correlation
    pub fn compare_ld_structures(
        reference: &ReferenceGenome,
        synthetic: &SyntheticGenome,
    ) -> (f32, f32) {
        let mut ref_values = Vec::new();
        let mut syn_values = Vec::new();

        for (pair, ref_r2) in &reference.ld_matrix {
            if let Some(syn_r2) = synthetic.ld_matrix.get(pair) {
                ref_values.push(*ref_r2);
                syn_values.push(*syn_r2);
            }
        }

        if ref_values.is_empty() {
            return (0.0, 0.0);
        }

        let mean_ref = ref_values.iter().sum::<f32>() / ref_values.len() as f32;
        let mean_syn = syn_values.iter().sum::<f32>() / syn_values.len() as f32;

        let mut cov_sum = 0.0;
        let mut var_ref_sum = 0.0;
        let mut var_syn_sum = 0.0;

        for (ref_r2, syn_r2) in ref_values.iter().zip(syn_values.iter()) {
            let ref_dev = ref_r2 - mean_ref;
            let syn_dev = syn_r2 - mean_syn;

            cov_sum += ref_dev * syn_dev;
            var_ref_sum += ref_dev * ref_dev;
            var_syn_sum += syn_dev * syn_dev;
        }

        if var_ref_sum <= 0.0 || var_syn_sum <= 0.0 {
            return (0.0, 0.0);
        }

        let pearson_r =
            cov_sum / (var_ref_sum.sqrt() * var_syn_sum.sqrt());
        let distance = (1.0 - pearson_r.abs()).abs();

        (pearson_r, distance)
    }

    /// Full validation comparison
    pub fn validate(
        reference: &ReferenceGenome,
        synthetic: &SyntheticGenome,
    ) -> ValidationResults {
        let (allele_freq_rmse, n_snps_compared) =
            Self::compare_allele_frequencies(reference, synthetic);

        let (ld_pearson_r, ld_distance) = Self::compare_ld_structures(reference, synthetic);

        let mut allele_freq_score = 1.0 - (allele_freq_rmse / 0.1).min(1.0);
        if allele_freq_rmse > 0.1 {
            allele_freq_score = (0.1 / (allele_freq_rmse + 0.01)).min(1.0);
        }

        let ld_score = ld_pearson_r.abs().clamp(0.0, 1.0);

        let overall_similarity = (0.6 * allele_freq_score + 0.4 * ld_score).clamp(0.0, 1.0);

        ValidationResults {
            ref_population: reference.population.clone(),
            allele_freq_distance: allele_freq_rmse,
            ld_distance,
            overall_similarity,
            n_snps_compared,
            allele_freq_rmse,
            ld_pearson_r,
        }
    }
}

/// Power analysis for association studies
pub struct PowerAnalysis;

impl PowerAnalysis {
    /// Calculate statistical power for a given effect size
    pub fn calculate_power(
        n_samples: usize,
        effect_size: f32,
        alpha: f32,
    ) -> f32 {
        let n = n_samples as f32;
        let lambda = effect_size * effect_size * n;

        if lambda < 1.0 {
            return alpha;
        }

        let z_alpha = inverse_normal_cdf(1.0 - alpha);
        let power = 1.0 - normal_cdf(z_alpha - lambda.sqrt());

        power.clamp(0.0, 1.0)
    }

    /// Minimum sample size needed for given power
    pub fn min_sample_size(effect_size: f32, power_target: f32, alpha: f32) -> usize {
        let z_alpha = inverse_normal_cdf(1.0 - alpha);
        let z_beta = inverse_normal_cdf(power_target);

        let n = ((z_alpha + z_beta) / effect_size).powi(2);
        (n.ceil() as usize).max(2)
    }
}

/// Standard normal CDF
fn normal_cdf(x: f32) -> f32 {
    0.5 * (1.0 + error_function(x / std::f32::consts::SQRT_2))
}

/// Inverse normal CDF (quantile function)
fn inverse_normal_cdf(p: f32) -> f32 {
    if p <= 0.0 {
        return f32::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f32::INFINITY;
    }

    let p_low = if p < 0.5 { p } else { 1.0 - p };
    let t = (-2.0 * p_low.ln()).sqrt();

    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let num = c0 + c1 * t + c2 * t * t;
    let den = 1.0 + d1 * t + d2 * t * t + d3 * t * t * t;

    let z = t - num / den;

    if p < 0.5 {
        -z
    } else {
        z
    }
}

/// Error function
fn error_function(x: f32) -> f32 {
    let a1 = 0.254_829_6;
    let a2 = -0.284_496_72;
    let a3 = 1.421_413_8;
    let a4 = -1.453_152_1;
    let a5 = 1.061_405_4;
    let p = 0.3275911;

    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x_abs = x.abs();

    let t = 1.0 / (1.0 + p * x_abs);
    let y = 1.0
        - (((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t) * (-x_abs * x_abs).exp();

    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allele_frequency_comparison() {
        let mut ref_genome = ReferenceGenome::new("1000G".to_string(), 2504);
        ref_genome.add_snp("SNP1".to_string(), 0.5, 0.5);
        ref_genome.add_snp("SNP2".to_string(), 0.3, 0.7);

        let mut syn_genome = SyntheticGenome::new(100);
        syn_genome.add_snp("SNP1".to_string(), 0.52, 0.48);
        syn_genome.add_snp("SNP2".to_string(), 0.31, 0.69);

        let (rmse, count) = GenomeComparator::compare_allele_frequencies(&ref_genome, &syn_genome);
        assert_eq!(count, 2);
        assert!(rmse < 0.05);
    }

    #[test]
    fn test_power_analysis() {
        let power = PowerAnalysis::calculate_power(1000, 0.1, 0.05);
        assert!(power > 0.8);

        let n = PowerAnalysis::min_sample_size(0.1, 0.8, 0.05);
        assert!(n > 100);
    }

    #[test]
    fn test_validation_results() {
        let results = ValidationResults {
            ref_population: "CEU".to_string(),
            allele_freq_distance: 0.02,
            ld_distance: 0.05,
            overall_similarity: 0.95,
            n_snps_compared: 1000,
            allele_freq_rmse: 0.02,
            ld_pearson_r: 0.95,
        };

        let summary = results.summary_string();
        assert!(summary.contains("CEU"));
        assert!(summary.contains("1000"));
    }
}
