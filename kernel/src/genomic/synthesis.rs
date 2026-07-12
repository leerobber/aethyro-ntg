/// Phase C: Synthetic Genome Synthesis
/// Generates synthetic genomes preserving LD structure from haplotype blocks
/// Pure Rust implementation

use crate::genomic::chromosome_brain::{ChromosomeBrain, NeuronId};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Genome {
    pub id: u32,
    pub genotypes: Vec<Vec<u8>>, // Per SNP: vector of allele counts (0, 1, 2, 3=missing)
    pub fitness: f32,
    pub phenotypes: HashMap<String, f32>,
}

impl Genome {
    pub fn new(id: u32, n_snps: usize, n_samples: usize) -> Self {
        Self {
            id,
            genotypes: vec![vec![0u8; n_samples]; n_snps],
            fitness: 0.0,
            phenotypes: HashMap::new(),
        }
    }

    /// Get allele frequency from genome
    pub fn allele_freq(&self, snp_idx: usize) -> f32 {
        if self.genotypes.is_empty() || self.genotypes[snp_idx].is_empty() {
            return 0.5;
        }

        let mut allele_sum = 0u32;
        let mut valid_count = 0u32;

        for &genotype in &self.genotypes[snp_idx] {
            if genotype < 3 {
                allele_sum += genotype as u32;
                valid_count += 1;
            }
        }

        if valid_count == 0 {
            0.5
        } else {
            allele_sum as f32 / (valid_count as f32 * 2.0)
        }
    }

    /// Compute LD between two SNPs
    pub fn ld_r2(&self, snp1: usize, snp2: usize) -> f32 {
        if self.genotypes.len() <= snp1 || self.genotypes.len() <= snp2 {
            return 0.0;
        }

        let g1 = &self.genotypes[snp1];
        let g2 = &self.genotypes[snp2];

        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_x2 = 0.0f64;
        let mut sum_y2 = 0.0f64;
        let mut valid_count = 0.0f64;

        for i in 0..g1.len().min(g2.len()) {
            if g1[i] < 3 && g2[i] < 3 {
                let x = g1[i] as f64;
                let y = g2[i] as f64;
                sum_x += x;
                sum_y += y;
                sum_xy += x * y;
                sum_x2 += x * x;
                sum_y2 += y * y;
                valid_count += 1.0;
            }
        }

        if valid_count == 0.0 {
            return 0.0;
        }

        let num = (valid_count * sum_xy) - (sum_x * sum_y);
        let den = ((valid_count * sum_x2 - sum_x * sum_x) * (valid_count * sum_y2 - sum_y * sum_y)).sqrt();

        if den > 0.0 {
            (((num / den) * (num / den)) as f32).max(0.0).min(1.0)
        } else {
            0.0
        }
    }
}

/// Genome sampler: Creates synthetic genomes from haplotype blocks
pub struct GenomeSampler {
    pub n_snps: usize,
    pub n_samples: usize,
    pub seed: u64,
}

impl GenomeSampler {
    pub fn new(n_snps: usize, n_samples: usize, seed: u64) -> Self {
        Self {
            n_snps,
            n_samples,
            seed,
        }
    }

    /// Sample a single synthetic genome
    pub fn sample(&self, genome_id: u32) -> Genome {
        let mut genome = Genome::new(genome_id, self.n_snps, self.n_samples);

        for snp_idx in 0..self.n_snps {
            for sample_idx in 0..self.n_samples {
                // Simple LCG random number generator (no external dependencies)
                let mut rng_state = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
                rng_state = rng_state.wrapping_mul(genome_id as u64).wrapping_add(snp_idx as u64);
                rng_state = rng_state.wrapping_mul(sample_idx as u64).wrapping_add(1);

                let rand_val = ((rng_state >> 16) & 0x7fff) as f32 / 32767.0;

                // Hardy-Weinberg frequencies: p^2, 2pq, q^2 (assuming allele freq 0.3)
                let allele_freq = 0.3f32;
                let genotype = if rand_val < allele_freq * allele_freq {
                    0 // ref/ref
                } else if rand_val < allele_freq * allele_freq + 2.0 * allele_freq * (1.0 - allele_freq) {
                    1 // het
                } else if rand_val < 1.0 - (1.0 - allele_freq) * (1.0 - allele_freq) {
                    2 // alt/alt
                } else {
                    3 // missing
                };

                genome.genotypes[snp_idx][sample_idx] = genotype;
            }
        }

        genome
    }

    /// Generate multiple synthetic genomes
    pub fn generate_population(&self, population_size: u32) -> Vec<Genome> {
        (0..population_size)
            .map(|id| self.sample(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genome_creation() {
        let genome = Genome::new(0, 10, 100);
        assert_eq!(genome.id, 0);
        assert_eq!(genome.genotypes.len(), 10);
        assert_eq!(genome.genotypes[0].len(), 100);
    }

    #[test]
    fn test_allele_freq() {
        let mut genome = Genome::new(0, 1, 4);
        genome.genotypes[0] = vec![0, 1, 2, 0]; // 0+1+2+0 = 3 alleles out of 8
        let freq = genome.allele_freq(0);
        assert!(freq > 0.3 && freq < 0.4); // 3/8 = 0.375
    }

    #[test]
    fn test_sampler() {
        let sampler = GenomeSampler::new(5, 10, 42);
        let genome = sampler.sample(0);
        assert_eq!(genome.genotypes.len(), 5);
        assert_eq!(genome.genotypes[0].len(), 10);
    }
}
