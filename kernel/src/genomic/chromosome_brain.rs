/// Phase B: Chromosome Brain Architecture
/// Initializes neurons from SNPs, synapses from LD pairs, embeddings from blocks
/// Ready for KAIROS training and multi-brain coordination

use crate::genomic::{BitstreamGenotypes, LdPair, HaplotypeBlock, SnpRecord};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NeuronId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChromosomeId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SynapseId(pub u32);

#[derive(Clone, Debug)]
pub struct GenomicNeuron {
    pub id: NeuronId,
    pub snp_index: u32,
    pub position_bp: u32,
    pub allele_freq: f32,
    pub maf: f32,
    pub is_rare: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Synapse {
    pub id: SynapseId,
    pub from: NeuronId,
    pub to: NeuronId,
    pub ld_r2: f32,
    pub weight: f32,
    pub plasticity: f32,
}

#[derive(Clone, Debug)]
pub struct EmbeddingLayer {
    pub snp_embeddings: Vec<[f32; 128]>,
    pub block_embeddings: Vec<[f32; 256]>,
    pub consolidated: Vec<[f32; 512]>,
}

#[derive(Clone, Debug)]
pub struct ChromosomeBrain {
    pub chr: ChromosomeId,
    pub neurons: Vec<GenomicNeuron>,
    pub synapses: Vec<Synapse>,
    pub blocks: Vec<HaplotypeBlock>,
    pub embeddings: EmbeddingLayer,
    pub training_cycles: u32,
    pub kairos_state: KairosState,
}

#[derive(Clone, Debug)]
pub struct KairosState {
    pub cycle: u32,
    pub convergence_score: f32,
    pub weight_updates: usize,
    pub last_embedding_drift: f32,
}

impl Default for KairosState {
    fn default() -> Self {
        Self {
            cycle: 0,
            convergence_score: 0.0,
            weight_updates: 0,
            last_embedding_drift: 1.0,
        }
    }
}

/// Initialize chromosome brain from Phase A outputs
pub fn init_chromosome_brain(
    chr: ChromosomeId,
    snps: &[BitstreamGenotypes],
    snp_records: &[SnpRecord],
    ld_pairs: &[LdPair],
    blocks: &[HaplotypeBlock],
) -> Result<ChromosomeBrain, String> {
    let n_snps = snps.len();
    if n_snps == 0 {
        return Err("Cannot initialize brain with zero SNPs".to_string());
    }

    // Layer 0: Neuron initialization from SNPs
    let neurons = init_neurons(snps, snp_records);

    // Layer 1: Synapse creation from LD pairs
    let synapses = init_synapses(&neurons, ld_pairs)?;

    // Layer 2: Embedding initialization
    let embeddings = init_embeddings(&neurons, blocks);

    Ok(ChromosomeBrain {
        chr,
        neurons,
        synapses,
        blocks: blocks.to_vec(),
        embeddings,
        training_cycles: 0,
        kairos_state: KairosState::default(),
    })
}

/// Layer 0: Initialize neurons from SNP list
fn init_neurons(snps: &[BitstreamGenotypes], snp_records: &[SnpRecord]) -> Vec<GenomicNeuron> {
    snps.iter()
        .enumerate()
        .map(|(idx, snp)| {
            // Alternate-allele frequency computed from the real genotype
            // calls for this SNP (VCF "AF" convention), not a fixed stand-in.
            let (_freq_ref, freq_alt, _freq_missing) = snp.allele_frequencies();
            let allele_freq = freq_alt as f32;
            let maf = allele_freq.min(1.0 - allele_freq);
            let is_rare = maf < 0.05;
            let position_bp = snp_records.get(idx).map(|r| r.position).unwrap_or(0);

            GenomicNeuron {
                id: NeuronId(idx as u32),
                snp_index: idx as u32,
                position_bp,
                allele_freq,
                maf,
                is_rare,
            }
        })
        .collect()
}

/// Layer 1: Initialize synapses from LD pairs
fn init_synapses(
    neurons: &[GenomicNeuron],
    ld_pairs: &[LdPair],
) -> Result<Vec<Synapse>, String> {
    let mut synapses = Vec::new();
    let mut synapse_id = 0u32;

    // Build SNP index map for fast lookup
    let mut idx_map = HashMap::new();
    for neuron in neurons {
        idx_map.insert(neuron.snp_index, neuron.id);
    }

    for pair in ld_pairs {
        if let (Some(&from_id), Some(&to_id)) = (
            idx_map.get(&pair.snp1_idx),
            idx_map.get(&pair.snp2_idx),
        ) {
            synapses.push(Synapse {
                id: SynapseId(synapse_id),
                from: from_id,
                to: to_id,
                ld_r2: pair.r_squared,
                weight: pair.r_squared, // Initialize weight to LD strength
                plasticity: 0.01 * pair.r_squared, // Learning rate scaled by LD
            });
            synapse_id += 1;
        }
    }

    if synapses.is_empty() {
        eprintln!("[WARNING] No synapses created from {} LD pairs", ld_pairs.len());
    }

    Ok(synapses)
}

/// Layer 2: Initialize embeddings for SNPs and blocks
fn init_embeddings(
    neurons: &[GenomicNeuron],
    blocks: &[HaplotypeBlock],
) -> EmbeddingLayer {
    // SNP embeddings: 128-dim, seeded from genomic position
    let snp_embeddings: Vec<_> = neurons
        .iter()
        .map(|n| {
            let mut emb = [0.0f32; 128];
            emb[0] = (n.position_bp as f32 * 1e-6).sin(); // Positional encoding
            emb[1] = n.maf;
            emb[2] = if n.is_rare { 1.0 } else { 0.0 };
            emb
        })
        .collect();

    // Block embeddings: 256-dim, seeded from block statistics
    let block_embeddings: Vec<_> = blocks
        .iter()
        .map(|b| {
            let mut emb = [0.0f32; 256];
            let span_bp = b.end_position.saturating_sub(b.start_position);
            emb[0] = b.mean_r_squared;
            emb[1] = (span_bp.max(1) as f32).log2().min(24.0); // Log-scale span
            emb[2] = (b.snp_indices.len() as f32).log2();
            emb
        })
        .collect();

    // Consolidated: 512-dim unification (8 blocks × 64 dims each)
    let consolidated = vec![[0.0f32; 512]; blocks.len().max(1)];

    EmbeddingLayer {
        snp_embeddings,
        block_embeddings,
        consolidated,
    }
}

impl ChromosomeBrain {
    /// Train via KAIROS cycle: synaptic weight adjustment + embedding consolidation
    pub fn train_kairos(&mut self, num_cycles: u32) -> KairosState {
        for _ in 0..num_cycles {
            self.kairos_step();
        }
        self.kairos_state.clone()
    }

    /// Single KAIROS step: weight update + embedding drift
    fn kairos_step(&mut self) {
        self.kairos_state.cycle += 1;

        // Phase 1: Synaptic weight adjustment (Hebbian-like)
        let mut weight_delta_sum = 0.0f32;
        for syn in &mut self.synapses {
            let target_weight = syn.ld_r2; // LD strength is learning target
            let delta = syn.plasticity * (target_weight - syn.weight);
            syn.weight = (syn.weight + delta).max(0.0).min(1.0);
            weight_delta_sum += delta.abs();
            if delta.abs() > 1e-4 {
                self.kairos_state.weight_updates += 1;
            }
        }

        // Phase 2: Embedding consolidation
        for (block_idx, block) in self.blocks.iter().enumerate() {
            if block_idx < self.embeddings.consolidated.len() {
                let emb = &mut self.embeddings.consolidated[block_idx];
                emb[0] = (block.mean_r_squared * self.kairos_state.cycle as f32 / 1000.0).min(1.0);
                emb[1] += 0.001; // Drift accumulation
            }
        }

        // Phase 3: Convergence scoring
        let avg_weight = if self.synapses.is_empty() {
            0.0
        } else {
            self.synapses.iter().map(|s| s.weight).sum::<f32>() / self.synapses.len() as f32
        };
        self.kairos_state.convergence_score = avg_weight;
        self.kairos_state.last_embedding_drift = weight_delta_sum / (self.synapses.len() as f32 + 1.0);
    }

    /// Query brain for neuron by position
    pub fn neuron_at_position(&self, pos_bp: u32) -> Option<&GenomicNeuron> {
        self.neurons.iter().find(|n| n.position_bp == pos_bp)
    }

    /// Get all synapses touching a neuron
    pub fn synapses_for_neuron(&self, neuron_id: NeuronId) -> Vec<&Synapse> {
        self.synapses
            .iter()
            .filter(|s| s.from == neuron_id || s.to == neuron_id)
            .collect()
    }

    /// Population-signal score without allocating a ChromosomeAgent (no clone).
    pub fn population_signal_score(&self) -> f32 {
        let n_rare = self.neurons.iter().filter(|n| n.is_rare).count();
        let rare_frac = n_rare as f32 / (self.neurons.len() as f32 + 1.0);
        rare_frac.clamp(0.0, 1.0)
    }

    /// Disease-risk style score over SNP indices without agent clone.
    pub fn disease_risk_score(&self, snp_indices: &[u32]) -> f32 {
        let mut score = 0.0f32;
        let mut hits = 0usize;
        for &target_idx in snp_indices {
            if let Some(neuron) = self.neurons.iter().find(|n| n.snp_index == target_idx) {
                hits += 1;
                score += 0.15;
                for block in &self.blocks {
                    if block.snp_indices.contains(&target_idx) {
                        score += block.mean_r_squared * 0.05;
                    }
                }
                let connected: f32 = self
                    .synapses_for_neuron(neuron.id)
                    .iter()
                    .map(|s| s.ld_r2)
                    .sum();
                score += (connected / (self.synapses.len() as f32 + 1.0)) * 0.1;
            }
        }
        if hits == 0 {
            0.0
        } else {
            (score / 2.0).clamp(0.0, 1.0)
        }
    }

    /// Fraction of neurons incident to at least one synapse.
    pub fn connectivity_score(&self) -> f32 {
        if self.neurons.is_empty() {
            return 0.0;
        }
        let mut touched = std::collections::HashSet::new();
        for s in &self.synapses {
            touched.insert(s.from.0);
            touched.insert(s.to.0);
        }
        (touched.len() as f32 / self.neurons.len() as f32).clamp(0.0, 1.0)
    }

    /// Summary statistics
    pub fn summary(&self) -> BrainSummary {
        let total_ld: f32 = self.synapses.iter().map(|s| s.ld_r2).sum();
        let avg_weight: f32 = if self.synapses.is_empty() {
            0.0
        } else {
            self.synapses.iter().map(|s| s.weight).sum::<f32>() / self.synapses.len() as f32
        };

        BrainSummary {
            chromosome: self.chr,
            n_neurons: self.neurons.len() as u32,
            n_synapses: self.synapses.len() as u32,
            n_blocks: self.blocks.len() as u32,
            total_ld: total_ld,
            avg_weight,
            training_cycles: self.training_cycles,
            convergence: self.kairos_state.convergence_score,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BrainSummary {
    pub chromosome: ChromosomeId,
    pub n_neurons: u32,
    pub n_synapses: u32,
    pub n_blocks: u32,
    pub total_ld: f32,
    pub avg_weight: f32,
    pub training_cycles: u32,
    pub convergence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_init() {
        let snps = vec![];
        let snp_records = vec![];
        let neurons = init_neurons(&snps, &snp_records);
        assert_eq!(neurons.len(), 0);
    }

    fn snp_record(id: &str, position: u32) -> SnpRecord {
        SnpRecord {
            id: id.to_string(),
            position,
            ref_allele: "A".to_string(),
            alt_allele: "G".to_string(),
            qual: 100.0,
            info: String::new(),
        }
    }

    #[test]
    fn test_neuron_init_uses_real_allele_frequencies() {
        // rs0: all ref/ref -> alt freq 0.0 (rare)
        let mut snp0 = BitstreamGenotypes::new(4);
        for i in 0..4 {
            snp0.set(i, 0);
        }

        // rs1: all alt/alt -> alt freq 1.0 (rare, monomorphic the other way)
        let mut snp1 = BitstreamGenotypes::new(4);
        for i in 0..4 {
            snp1.set(i, 2);
        }

        // rs2: half het, half ref/ref -> alt freq 0.25
        let mut snp2 = BitstreamGenotypes::new(4);
        snp2.set(0, 1);
        snp2.set(1, 1);
        snp2.set(2, 0);
        snp2.set(3, 0);

        let snps = vec![snp0, snp1, snp2];
        let snp_records = vec![
            snp_record("rs0", 100),
            snp_record("rs1", 200),
            snp_record("rs2", 300),
        ];

        let neurons = init_neurons(&snps, &snp_records);

        assert_eq!(neurons.len(), 3);
        assert!((neurons[0].allele_freq - 0.0).abs() < 1e-6);
        assert!((neurons[1].allele_freq - 1.0).abs() < 1e-6);
        assert!((neurons[2].allele_freq - 0.25).abs() < 1e-6);

        // Not every neuron collapses to the same constant anymore.
        assert!(neurons[0].allele_freq != neurons[2].allele_freq);
        assert!(neurons[0].is_rare); // MAF 0.0
        assert!(neurons[1].is_rare); // MAF 0.0 (monomorphic alt)
        assert!(!neurons[2].is_rare); // MAF 0.25, common
    }

    #[test]
    fn test_kairos_convergence() {
        let mut brain = ChromosomeBrain {
            chr: ChromosomeId(1),
            neurons: vec![GenomicNeuron {
                id: NeuronId(0),
                snp_index: 0,
                position_bp: 1000,
                allele_freq: 0.25,
                maf: 0.25,
                is_rare: false,
            }],
            synapses: vec![Synapse {
                id: SynapseId(0),
                from: NeuronId(0),
                to: NeuronId(0),
                ld_r2: 0.8,
                weight: 0.1,
                plasticity: 0.01,
            }],
            blocks: vec![],
            embeddings: EmbeddingLayer {
                snp_embeddings: vec![[0.0; 128]],
                block_embeddings: vec![],
                consolidated: vec![],
            },
            training_cycles: 0,
            kairos_state: KairosState::default(),
        };

        let initial_weight = brain.synapses[0].weight;
        brain.train_kairos(100);
        let final_weight = brain.synapses[0].weight;

        assert!(final_weight > initial_weight);
        assert!(brain.kairos_state.cycle > 0);
    }
}
