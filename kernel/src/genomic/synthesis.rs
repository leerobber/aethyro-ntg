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
    /// Per-SNP target allele frequency. Empty means "no real data supplied"
    /// and every locus falls back to a flat 0.3 (the old behavior, kept for
    /// callers that only want a synthetic population with no reference).
    pub allele_freqs: Vec<f32>,
}

impl GenomeSampler {
    pub fn new(n_snps: usize, n_samples: usize, seed: u64) -> Self {
        Self {
            n_snps,
            n_samples,
            seed,
            allele_freqs: Vec::new(),
        }
    }

    /// Build a sampler whose per-locus allele frequencies come from a real
    /// `ChromosomeBrain` (i.e. the real 1000 Genomes-derived MAF at each
    /// SNP, via `GenomicNeuron::allele_freq`), instead of one flat
    /// assumed frequency for the whole chromosome.
    pub fn from_brain(brain: &ChromosomeBrain, n_samples: usize, seed: u64) -> Self {
        let allele_freqs: Vec<f32> = brain.neurons.iter().map(|n| n.allele_freq).collect();
        Self {
            n_snps: allele_freqs.len(),
            n_samples,
            seed,
            allele_freqs,
        }
    }

    fn allele_freq_for(&self, snp_idx: usize) -> f32 {
        self.allele_freqs.get(snp_idx).copied().unwrap_or(0.3)
    }

    /// Deterministic pseudo-random value in [0, 1) for one (genome, SNP,
    /// sample) coordinate. A plain LCG stepped by adding-then-multiplying
    /// each index in turn is linear in the last index folded in (the
    /// per-locus loop only varies sample_idx, so consecutive samples would
    /// land on an arithmetic progression through the state space) --
    /// that pattern survives even after fixing the earlier "multiply by
    /// an index that can be 0" collapse. Folding all three indices in with
    /// distinct odd multipliers and finishing with a full avalanche mix
    /// (splitmix64's finalizer) is what actually makes the extracted bits
    /// behave like independent draws instead of a visible linear sequence.
    fn hash_unit_interval(seed: u64, genome_id: u32, snp_idx: u32, sample_idx: u32) -> f32 {
        let mut x = seed
            ^ (genome_id as u64).wrapping_mul(0x9E3779B97F4A7C15)
            ^ (snp_idx as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
            ^ (sample_idx as u64).wrapping_mul(0x165667B19E3779F9);

        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d049bb133111eb);
        x ^= x >> 31;

        (x >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Sample a single synthetic genome
    pub fn sample(&self, genome_id: u32) -> Genome {
        let mut genome = Genome::new(genome_id, self.n_snps, self.n_samples);

        for snp_idx in 0..self.n_snps {
            let allele_freq = self.allele_freq_for(snp_idx);

            // allele_freq is the ALT-allele frequency (VCF AF convention,
            // matching GenomicNeuron::allele_freq and what Genome::allele_freq
            // measures back out). Hardy-Weinberg: ref/ref = (1-p)^2,
            // het = 2p(1-p), alt/alt = p^2.
            let ref_freq = 1.0 - allele_freq;
            let ref_ref_bound = ref_freq * ref_freq;
            let het_bound = ref_ref_bound + 2.0 * allele_freq * ref_freq;

            for sample_idx in 0..self.n_samples {
                let rand_val = Self::hash_unit_interval(self.seed, genome_id, snp_idx as u32, sample_idx as u32);

                let genotype = if rand_val < ref_ref_bound {
                    0 // ref/ref
                } else if rand_val < het_bound {
                    1 // het
                } else if rand_val < 1.0 {
                    2 // alt/alt
                } else {
                    3 // missing (unreachable at f32 precision, kept for symmetry)
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

    #[test]
    fn test_sampler_without_real_data_falls_back_to_flat_frequency() {
        let sampler = GenomeSampler::new(1, 2000, 7);
        let genome = sampler.sample(0);
        let freq = genome.allele_freq(0);
        assert!((freq - 0.3).abs() < 0.05, "expected ~0.3, got {}", freq);
    }

    #[test]
    fn test_sampler_uses_per_locus_allele_frequency() {
        let sampler = GenomeSampler {
            n_snps: 2,
            n_samples: 2000,
            seed: 7,
            allele_freqs: vec![0.05, 0.95], // rare vs. common, not the old flat 0.3
        };

        let genome = sampler.sample(0);

        let freq0 = genome.allele_freq(0);
        let freq1 = genome.allele_freq(1);

        assert!(freq0 < 0.15, "expected rare locus, got {}", freq0);
        assert!(freq1 > 0.85, "expected common locus, got {}", freq1);
    }

    #[test]
    fn test_from_brain_extracts_real_allele_frequencies() {
        use crate::genomic::chromosome_brain::{
            ChromosomeId as ChrId, EmbeddingLayer, GenomicNeuron, KairosState, NeuronId as NId,
        };

        let brain = ChromosomeBrain {
            chr: ChrId(1),
            neurons: vec![
                GenomicNeuron {
                    id: NId(0),
                    snp_index: 0,
                    position_bp: 100,
                    allele_freq: 0.05,
                    maf: 0.05,
                    is_rare: true,
                },
                GenomicNeuron {
                    id: NId(1),
                    snp_index: 1,
                    position_bp: 200,
                    allele_freq: 0.9,
                    maf: 0.1,
                    is_rare: false,
                },
            ],
            synapses: vec![],
            blocks: vec![],
            embeddings: EmbeddingLayer {
                snp_embeddings: vec![],
                block_embeddings: vec![],
                consolidated: vec![],
            },
            training_cycles: 0,
            kairos_state: KairosState::default(),
        };

        let sampler = GenomeSampler::from_brain(&brain, 500, 3);

        assert_eq!(sampler.n_snps, 2);
        assert_eq!(sampler.allele_freqs, vec![0.05, 0.9]);
    }
}
