/// OmniSynth-X Genomic Operator for NTG
/// Bitsliced LD/PRS computation with SIMD acceleration
/// Integrated into Neural Ternary Graph for self-evolution

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Core genomic data storage: bitsliced ternary genotypes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenomicOperator {
    /// Raw bitsliced storage: low + high bitplanes per SNP
    pub data: Vec<u64>,

    /// Metadata
    pub num_individuals: usize,
    pub num_snps: usize,
    pub words_per_snp: usize,

    /// Variant metadata
    pub variant_names: Vec<String>, // rs123, rs456, etc.
    pub positions: Vec<u32>,        // Chromosome positions
    pub allele_freqs: Vec<f64>,     // Allele frequency for each variant

    /// Cached statistics
    pub means: Vec<f64>,
    pub std_devs: Vec<f64>,
    pub is_stats_valid: bool,
}

impl GenomicOperator {
    /// Initialize a new genomic operator
    pub fn new(num_individuals: usize, num_snps: usize) -> Self {
        let words_per_snp = (num_individuals + 63) / 64;
        let total_words = num_snps * words_per_snp * 2;

        Self {
            data: vec![0; total_words],
            num_individuals,
            num_snps,
            words_per_snp,
            variant_names: vec![String::new(); num_snps],
            positions: vec![0; num_snps],
            allele_freqs: vec![0.0; num_snps],
            means: vec![0.0; num_snps],
            std_devs: vec![1.0; num_snps],
            is_stats_valid: false,
        }
    }

    /// Get byte offsets for a SNP's low and high bitplanes
    #[inline(always)]
    fn get_offsets(&self, snp_idx: usize, word_idx: usize) -> (usize, usize) {
        let snp_stride = self.words_per_snp * 2;
        let base = snp_idx * snp_stride;
        (base + word_idx, base + self.words_per_snp + word_idx)
    }

    /// Set genotype value (0, 1, 2, or 3 for missing)
    pub fn set(&mut self, snp_idx: usize, ind_idx: usize, val: u8) {
        if snp_idx >= self.num_snps || ind_idx >= self.num_individuals {
            return;
        }

        let word_idx = ind_idx / 64;
        let bit_pos = ind_idx % 64;
        let (low_offset, high_offset) = self.get_offsets(snp_idx, word_idx);

        let low_bit = (val & 1) as u64;
        let high_bit = ((val >> 1) & 1) as u64;

        self.data[low_offset] = (self.data[low_offset] & !(1 << bit_pos)) | (low_bit << bit_pos);
        self.data[high_offset] = (self.data[high_offset] & !(1 << bit_pos)) | (high_bit << bit_pos);

        self.is_stats_valid = false; // Invalidate cache
    }

    /// Get genotype value
    pub fn get(&self, snp_idx: usize, ind_idx: usize) -> u8 {
        if snp_idx >= self.num_snps || ind_idx >= self.num_individuals {
            return 3; // Missing
        }

        let word_idx = ind_idx / 64;
        let bit_pos = ind_idx % 64;
        let (low_offset, high_offset) = self.get_offsets(snp_idx, word_idx);

        let low_bit = (self.data[low_offset] >> bit_pos) & 1;
        let high_bit = (self.data[high_offset] >> bit_pos) & 1;

        (low_bit as u8) | ((high_bit as u8) << 1)
    }

    /// Compute descriptive statistics (means, std_devs)
    pub fn compute_statistics(&mut self) {
        self.means.resize(self.num_snps, 0.0);
        self.std_devs.resize(self.num_snps, 1.0);

        for i in 0..self.num_snps {
            let mut sum_g = 0u64;
            let mut sum_g2 = 0u64;
            let mut total_valid = 0u64;
            let base_i = i * self.words_per_snp * 2;

            for w in 0..self.words_per_snp {
                let l = self.data[base_i + w];
                let h = self.data[base_i + self.words_per_snp + w];

                let mask = if w == self.words_per_snp - 1 {
                    let remainder = self.num_individuals % 64;
                    if remainder == 0 { !0u64 } else { (1u64 << remainder) - 1 }
                } else {
                    !0u64
                };

                let valid_mask = !(l & h) & mask; // Exclude missing (11)
                let l_v = l & valid_mask;
                let h_v = h & valid_mask;

                sum_g += l_v.count_ones() as u64 + ((h_v.count_ones() as u64) << 1);
                sum_g2 += l_v.count_ones() as u64 + ((h_v.count_ones() as u64) << 2);
                total_valid += valid_mask.count_ones() as u64;
            }

            let n = total_valid as f64;
            if n > 0.0 {
                let mean = sum_g as f64 / n;
                let variance = (sum_g2 as f64 / n) - (mean * mean);
                self.means[i] = mean;
                self.std_devs[i] = if variance > 1e-9 { variance.sqrt() } else { 1.0 };
                self.allele_freqs[i] = mean / 2.0; // Allele frequency
            }
        }

        self.is_stats_valid = true;
    }

    /// Compute Linkage Disequilibrium (LD) matrix
    /// Returns upper-triangular correlation matrix as flat vector
    pub fn compute_ld_matrix(&mut self) -> Vec<f64> {
        if !self.is_stats_valid {
            self.compute_statistics();
        }

        let mut ld_matrix = vec![0.0; self.num_snps * self.num_snps];

        for i in 0..self.num_snps {
            let base_i = i * self.words_per_snp * 2;
            let mu_i = self.means[i];
            let sigma_i = self.std_devs[i];

            for j in i..self.num_snps {
                let base_j = j * self.words_per_snp * 2;
                let mut dot_product = 0u64;
                let mut mutual_valid_count = 0u64;

                for w in 0..self.words_per_snp {
                    let l_i = self.data[base_i + w];
                    let h_i = self.data[base_i + self.words_per_snp + w];
                    let l_j = self.data[base_j + w];
                    let h_j = self.data[base_j + self.words_per_snp + w];

                    let mask = if w == self.words_per_snp - 1 {
                        let remainder = self.num_individuals % 64;
                        if remainder == 0 { !0u64 } else { (1u64 << remainder) - 1 }
                    } else {
                        !0u64
                    };

                    // Valid mask: exclude missing data (11 in both planes)
                    let valid_mask = !(l_i & h_i) & !(l_j & h_j) & mask;
                    mutual_valid_count += valid_mask.count_ones() as u64;

                    let l_i_v = l_i & valid_mask;
                    let h_i_v = h_i & valid_mask;
                    let l_j_v = l_j & valid_mask;
                    let h_j_v = h_j & valid_mask;

                    // Branchless polynomial evaluation: G_ik * G_jk
                    dot_product += (l_i_v & l_j_v).count_ones() as u64;                    // 0*0 = 0
                    dot_product += ((l_i_v & h_j_v).count_ones() as u64) << 1;             // 0*2 = 0, 1*1 = 1
                    dot_product += ((h_i_v & l_j_v).count_ones() as u64) << 1;             // 2*1 = 2
                    dot_product += ((h_i_v & h_j_v).count_ones() as u64) << 2;             // 2*2 = 4
                }

                let n_ij = mutual_valid_count as f64;
                if n_ij > 1.0 && sigma_i > 1e-9 && self.std_devs[j] > 1e-9 {
                    let r = ((dot_product as f64 / n_ij) - (mu_i * self.means[j]))
                            / (sigma_i * self.std_devs[j]);

                    // Clamp to [-1, 1]
                    let r_clamped = r.max(-1.0).min(1.0);

                    ld_matrix[i * self.num_snps + j] = r_clamped;
                    ld_matrix[j * self.num_snps + i] = r_clamped; // Symmetric
                } else {
                    ld_matrix[i * self.num_snps + j] = 0.0;
                    ld_matrix[j * self.num_snps + i] = 0.0;
                }
            }
        }

        ld_matrix
    }

    /// Compute Polygenic Risk Scores (PRS)
    /// Scores = sum of (genotype * weight) for each individual
    pub fn compute_prs(&self, weights: &[f64]) -> Vec<f64> {
        let mut prs_scores = vec![0.0; self.num_individuals];

        if weights.len() != self.num_snps {
            return prs_scores; // Dimension mismatch
        }

        for snp_idx in 0..self.num_snps {
            let weight = weights[snp_idx];
            let base = snp_idx * self.words_per_snp * 2;

            for word_idx in 0..self.words_per_snp {
                let low_idx = base + word_idx;
                let high_idx = base + self.words_per_snp + word_idx;

                let low_word = self.data[low_idx];
                let high_word = self.data[high_idx];

                for bit_pos in 0..64 {
                    let ind_idx = word_idx * 64 + bit_pos;
                    if ind_idx >= self.num_individuals { break; }

                    let l = (low_word >> bit_pos) & 1;
                    let h = (high_word >> bit_pos) & 1;

                    // Skip missing data (11)
                    if !(l == 1 && h == 1) {
                        let genotype_val = l + (h << 1);
                        prs_scores[ind_idx] += (genotype_val as f64) * weight;
                    }
                }
            }
        }

        prs_scores
    }

    /// Identify high-LD SNP pairs (r² > threshold)
    pub fn find_ld_clusters(&mut self, r2_threshold: f64) -> HashMap<usize, Vec<usize>> {
        let ld_matrix = self.compute_ld_matrix();
        let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();

        for i in 0..self.num_snps {
            clusters.insert(i, Vec::new());

            for j in (i + 1)..self.num_snps {
                let r = ld_matrix[i * self.num_snps + j];
                let r2 = r * r;

                if r2 > r2_threshold {
                    clusters.get_mut(&i).unwrap().push(j);
                }
            }
        }

        clusters
    }

    /// Get summary statistics
    pub fn summary(&self) -> GenomicSummary {
        GenomicSummary {
            num_snps: self.num_snps,
            num_individuals: self.num_individuals,
            total_genotypes: self.num_snps * self.num_individuals,
            missing_rate: self.estimate_missing_rate(),
            mean_maf: self.allele_freqs.iter().sum::<f64>() / self.allele_freqs.len() as f64,
        }
    }

    fn estimate_missing_rate(&self) -> f64 {
        let mut missing_count = 0u64;
        let total_count = (self.num_snps * self.num_individuals) as u64;

        for snp_idx in 0..self.num_snps {
            let base = snp_idx * self.words_per_snp * 2;
            for w in 0..self.words_per_snp {
                let l = self.data[base + w];
                let h = self.data[base + self.words_per_snp + w];
                missing_count += (l & h).count_ones() as u64; // Count 11 bits
            }
        }

        missing_count as f64 / total_count as f64
    }
}

/// Genomic summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicSummary {
    pub num_snps: usize,
    pub num_individuals: usize,
    pub total_genotypes: usize,
    pub missing_rate: f64,
    pub mean_maf: f64,
}

/// NTG GraphNode wrapper for genomic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicNode {
    pub node_id: usize,
    pub operator: GenomicOperator,
    pub operator_type: String,
}

impl GenomicNode {
    pub fn new(node_id: usize, num_individuals: usize, num_snps: usize) -> Self {
        Self {
            node_id,
            operator: GenomicOperator::new(num_individuals, num_snps),
            operator_type: "genomic".to_string(),
        }
    }

    /// Get summary of this genomic node
    pub fn summary(&self) -> GenomicSummary {
        self.operator.summary()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genomic_operator_creation() {
        let op = GenomicOperator::new(1000, 100);
        assert_eq!(op.num_individuals, 1000);
        assert_eq!(op.num_snps, 100);
    }

    #[test]
    fn test_set_get_genotype() {
        let mut op = GenomicOperator::new(100, 10);
        op.set(0, 0, 2);
        assert_eq!(op.get(0, 0), 2);

        op.set(5, 50, 1);
        assert_eq!(op.get(5, 50), 1);
    }

    #[test]
    fn test_prs_computation() {
        let mut op = GenomicOperator::new(100, 10);

        // Set some genotypes
        for i in 0..10 {
            op.set(i, 0, 1); // Set first individual to heterozygous
        }

        let weights = vec![0.5; 10];
        let prs = op.compute_prs(&weights);

        assert_eq!(prs.len(), 100);
        assert!(prs[0] > 0.0); // First individual should have non-zero PRS
    }

    #[test]
    fn test_statistics() {
        let mut op = GenomicOperator::new(1000, 50);

        // Fill with random data
        for snp in 0..50 {
            for ind in 0..1000 {
                op.set(snp, ind, ((snp + ind) % 3) as u8);
            }
        }

        op.compute_statistics();

        assert_eq!(op.means.len(), 50);
        assert_eq!(op.std_devs.len(), 50);
        assert!(op.is_stats_valid);
    }
}
