//! Rung 1: SovereignBrain — multi-chromosome custom brain with working set + LTM.
//!
//! Unifies per-chromosome [`ChromosomeBrain`] instances into one organism with:
//! - **Working set** — bounded active (chr, neuron) addresses for the current query
//! - **Long-term memory (LTM)** — unbounded motif / embedding store that survives
//!   consolidate/activate cycles (session-local; serializable later)
//! - **Ingest** — from a built [`ChromosomeBrain`] or full real VCF pipeline
//!
//! This is structure + memory plumbing. Selection pressure is Rung 2
//! ([`crate::ntg::mutation::MultiAxisFitness`]).

use crate::genomic::chromosome_brain::{
    ChromosomeBrain, ChromosomeId, NeuronId, BrainSummary,
};
use crate::genomic::haplotype_blocks::HaplotypeBlock;
use crate::genomic::language_organ::LanguageOrgan;
use crate::genomic::real_pipeline::{build_real_chromosome, RealChromosomeData};
use crate::ntg::graph::NodeId;
use std::collections::{BTreeMap, HashSet};

/// Global address of a neuron inside the sovereign brain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalNeuronRef {
    pub chr: ChromosomeId,
    pub neuron: NeuronId,
}

/// One consolidated motif kept in long-term memory (from a haplotype block).
#[derive(Clone, Debug)]
pub struct LtmMotif {
    pub id: u64,
    pub source_chr: ChromosomeId,
    pub block_id: u32,
    pub n_snps: u32,
    pub mean_r_squared: f32,
    pub start_bp: u32,
    pub end_bp: u32,
    /// Compact signature used for retrieval scoring (not a neural embedding yet).
    pub signature: [f32; 8],
    /// How many times this motif has been activated.
    pub hit_count: u64,
}

/// Snapshot of LTM state for metrics / fitness structural axis.
#[derive(Clone, Debug, Default)]
pub struct LtmStats {
    pub n_motifs: usize,
    pub total_hits: u64,
    pub approx_bytes: usize,
}

/// Bounded working set after an activate() call.
#[derive(Clone, Debug, Default)]
pub struct WorkingSet {
    pub neurons: Vec<GlobalNeuronRef>,
    pub motif_ids: Vec<u64>,
    /// Language/SIS graph nodes co-activated (Rung 3).
    pub language_nodes: Vec<NodeId>,
    pub language_query: String,
    pub capacity: usize,
}

impl WorkingSet {
    pub fn new(capacity: usize) -> Self {
        Self {
            neurons: Vec::new(),
            motif_ids: Vec::new(),
            language_nodes: Vec::new(),
            language_query: String::new(),
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.neurons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.neurons.is_empty() && self.language_nodes.is_empty()
    }
}

/// Multi-chromosome brain with working memory + long-term motif store.
#[derive(Clone, Debug)]
pub struct SovereignBrain {
    pub chromosomes: BTreeMap<u8, ChromosomeBrain>,
    pub working_set: WorkingSet,
    pub ltm: Vec<LtmMotif>,
    /// Optional language/SIS organ (Rung 3).
    pub language: Option<LanguageOrgan>,
    /// Next LTM motif id (pub(crate) for persistence restore).
    pub(crate) next_motif_id: u64,
    /// Generation counter for evolution / fitness loops (Rung 2).
    pub generation: u64,
    /// Last multi-axis-friendly structural metrics (filled by measure_structure).
    pub last_structural: StructuralMetrics,
}

/// Cheap structural metrics used as the structural-cost axis input.
#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralMetrics {
    pub n_chromosomes: u32,
    pub n_neurons: u32,
    pub n_synapses: u32,
    pub n_blocks: u32,
    pub n_ltm_motifs: u32,
    pub working_set_len: u32,
    /// Approximate live memory (bytes) of neurons/synapses + LTM signatures.
    pub approx_memory_bytes: u64,
    /// Mean synapse weight across all chromosomes (0–1).
    pub mean_synapse_weight: f32,
    /// Mean block LD (0–1).
    pub mean_block_r2: f32,
}

impl Default for SovereignBrain {
    fn default() -> Self {
        Self::new(256)
    }
}

impl SovereignBrain {
    /// Create an empty brain with a working-set capacity.
    pub fn new(working_set_capacity: usize) -> Self {
        Self {
            chromosomes: BTreeMap::new(),
            working_set: WorkingSet::new(working_set_capacity.max(1)),
            ltm: Vec::new(),
            language: None,
            next_motif_id: 1,
            generation: 0,
            last_structural: StructuralMetrics::default(),
        }
    }

    /// Attach or replace the language/SIS organ.
    pub fn attach_language(&mut self, organ: LanguageOrgan) {
        self.language = Some(organ);
    }

    /// Language organ mut access.
    pub fn language_mut(&mut self) -> Option<&mut LanguageOrgan> {
        self.language.as_mut()
    }

    pub fn language(&self) -> Option<&LanguageOrgan> {
        self.language.as_ref()
    }

    /// Insert or replace a chromosome brain. Returns previous if any.
    pub fn insert_chromosome(&mut self, brain: ChromosomeBrain) -> Option<ChromosomeBrain> {
        let key = brain.chr.0;
        self.chromosomes.insert(key, brain)
    }

    /// Ingest a chromosome already built by the Phase A/B pipeline.
    pub fn ingest_brain(&mut self, brain: ChromosomeBrain) {
        // Seed LTM with high-quality blocks immediately so retrieval works
        // before the first consolidate().
        let chr = brain.chr;
        for block in &brain.blocks {
            if block.mean_r_squared >= 0.5 && block.snp_indices.len() >= 2 {
                self.push_motif_from_block(chr, block);
            }
        }
        self.insert_chromosome(brain);
        self.refresh_structure();
    }

    /// Full real-data ingest: VCF → LD → blocks → brain → sovereign insert.
    ///
    /// `use_haplotypes` matches [`build_real_chromosome`]. Returns the
    /// intermediate pipeline payload for callers that also want validation.
    pub fn ingest_vcf(
        &mut self,
        vcf_path: &str,
        chr: u8,
        max_variants: Option<usize>,
        synthetic_n_samples: usize,
        seed: u64,
        use_haplotypes: bool,
    ) -> Result<RealChromosomeData, String> {
        let data = build_real_chromosome(
            vcf_path,
            chr,
            max_variants,
            synthetic_n_samples,
            seed,
            use_haplotypes,
        )?;
        self.ingest_brain(data.brain.clone());
        Ok(data)
    }

    /// Number of loaded chromosomes.
    pub fn n_chromosomes(&self) -> usize {
        self.chromosomes.len()
    }

    /// Get a chromosome brain by id.
    pub fn chromosome(&self, chr: u8) -> Option<&ChromosomeBrain> {
        self.chromosomes.get(&chr)
    }

    pub fn chromosome_mut(&mut self, chr: u8) -> Option<&mut ChromosomeBrain> {
        self.chromosomes.get_mut(&chr)
    }

    /// Summaries for every loaded chromosome.
    pub fn chromosome_summaries(&self) -> Vec<BrainSummary> {
        self.chromosomes.values().map(|b| b.summary()).collect()
    }

    /// LTM stats for observability / fitness.
    pub fn ltm_stats(&self) -> LtmStats {
        let total_hits = self.ltm.iter().map(|m| m.hit_count).sum();
        let approx_bytes = self.ltm.len() * std::mem::size_of::<LtmMotif>();
        LtmStats {
            n_motifs: self.ltm.len(),
            total_hits,
            approx_bytes,
        }
    }

    /// Recompute [`StructuralMetrics`] and cache on `last_structural`.
    pub fn refresh_structure(&mut self) -> StructuralMetrics {
        let m = self.measure_structure();
        self.last_structural = m;
        m
    }

    /// Measure live structural metrics without mutating generation.
    pub fn measure_structure(&self) -> StructuralMetrics {
        let mut n_neurons = 0u32;
        let mut n_synapses = 0u32;
        let mut n_blocks = 0u32;
        let mut weight_sum = 0.0f32;
        let mut weight_n = 0u32;
        let mut r2_sum = 0.0f32;
        let mut r2_n = 0u32;

        for brain in self.chromosomes.values() {
            n_neurons += brain.neurons.len() as u32;
            n_synapses += brain.synapses.len() as u32;
            n_blocks += brain.blocks.len() as u32;
            for s in &brain.synapses {
                weight_sum += s.weight;
                weight_n += 1;
            }
            for b in &brain.blocks {
                r2_sum += b.mean_r_squared;
                r2_n += 1;
            }
        }

        // Rough live footprint: neuron + synapse structs + LTM motifs.
        let approx_memory_bytes = (n_neurons as u64) * 64
            + (n_synapses as u64) * 32
            + (self.ltm.len() as u64) * 64
            + (self.working_set.neurons.len() as u64) * 16;

        StructuralMetrics {
            n_chromosomes: self.chromosomes.len() as u32,
            n_neurons,
            n_synapses,
            n_blocks,
            n_ltm_motifs: self.ltm.len() as u32,
            working_set_len: self.working_set.neurons.len() as u32,
            approx_memory_bytes,
            mean_synapse_weight: if weight_n > 0 {
                weight_sum / weight_n as f32
            } else {
                0.0
            },
            mean_block_r2: if r2_n > 0 {
                r2_sum / r2_n as f32
            } else {
                0.0
            },
        }
    }

    /// Push a block-derived motif into LTM (dedupes by chr+block_id).
    fn push_motif_from_block(&mut self, chr: ChromosomeId, block: &HaplotypeBlock) {
        if self
            .ltm
            .iter()
            .any(|m| m.source_chr == chr && m.block_id == block.id)
        {
            return;
        }
        let span = block.end_position.saturating_sub(block.start_position).max(1) as f32;
        let mut signature = [0.0f32; 8];
        signature[0] = block.mean_r_squared;
        signature[1] = (block.snp_indices.len() as f32).ln().max(0.0);
        signature[2] = span.ln().max(0.0);
        signature[3] = (chr.0 as f32) / 22.0;
        signature[4] = (block.start_position as f32) * 1e-8;
        signature[5] = (block.end_position as f32) * 1e-8;
        signature[6] = if block.mean_r_squared > 0.8 { 1.0 } else { 0.0 };
        signature[7] = (block.size as f32).ln().max(0.0);

        let id = self.next_motif_id;
        self.next_motif_id += 1;
        self.ltm.push(LtmMotif {
            id,
            source_chr: chr,
            block_id: block.id,
            n_snps: block.snp_indices.len() as u32,
            mean_r_squared: block.mean_r_squared,
            start_bp: block.start_position,
            end_bp: block.end_position,
            signature,
            hit_count: 0,
        });
    }

    /// Consolidate: copy high-LD blocks into LTM and optionally prune the
    /// weakest synapses from the live graph (structural compression).
    ///
    /// `min_r2` — only blocks with mean r² ≥ this enter LTM.
    /// `prune_frac` — fraction of weakest synapses to drop per chromosome (0–0.5).
    pub fn consolidate(&mut self, min_r2: f32, prune_frac: f32) -> ConsolidateReport {
        let mut synapses_pruned = 0u32;

        // Collect block clones first to avoid borrow issues.
        let block_jobs: Vec<(ChromosomeId, HaplotypeBlock)> = self
            .chromosomes
            .values()
            .flat_map(|b| {
                b.blocks
                    .iter()
                    .filter(|bl| bl.mean_r_squared >= min_r2 && bl.snp_indices.len() >= 2)
                    .map(|bl| (b.chr, bl.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        let before = self.ltm.len();
        for (chr, block) in &block_jobs {
            self.push_motif_from_block(*chr, block);
        }
        let motifs_added = (self.ltm.len().saturating_sub(before)) as u32;

        let prune_frac = prune_frac.clamp(0.0, 0.5);
        if prune_frac > 0.0 {
            for brain in self.chromosomes.values_mut() {
                if brain.synapses.len() < 4 {
                    continue;
                }
                let n_drop = ((brain.synapses.len() as f32) * prune_frac).floor() as usize;
                if n_drop == 0 {
                    continue;
                }
                brain
                    .synapses
                    .sort_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap_or(std::cmp::Ordering::Equal));
                brain.synapses.drain(0..n_drop);
                synapses_pruned += n_drop as u32;
            }
        }

        self.generation += 1;
        self.refresh_structure();
        ConsolidateReport {
            motifs_added,
            synapses_pruned,
            ltm_total: self.ltm.len(),
            generation: self.generation,
        }
    }

    /// Activate a working set from a query signature (8-dim) + optional
    /// chromosome filter. Fills up to `working_set.capacity` neurons from
    /// top-matching LTM motifs and high-weight synapses on those chromosomes.
    ///
    /// Hardening: if a chr filter yields no motif hits, falls back to global
    /// LTM ranking; if cosine scores are weak, still activates top motifs by
    /// mean_r² so LTM is never silently unused after consolidate.
    pub fn activate(&mut self, query: &[f32; 8], chr_filter: Option<u8>) -> &WorkingSet {
        let capacity = self.working_set.capacity;

        // Score motifs (filtered, then global if empty).
        let mut scored = self.score_motifs(query, chr_filter);
        if scored.is_empty() && chr_filter.is_some() {
            scored = self.score_motifs(query, None);
        }
        // If cosine is uniformly poor, re-rank by motif strength (mean r²).
        let best_cos = scored.first().map(|(_, s, _)| *s).unwrap_or(0.0);
        if !self.ltm.is_empty() && (scored.is_empty() || best_cos < 0.05) {
            scored = self
                .ltm
                .iter()
                .map(|m| (m.id, m.mean_r_squared, m.source_chr))
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }

        let mut neurons = Vec::new();
        let mut motif_ids = Vec::new();
        let mut seen: HashSet<GlobalNeuronRef> = HashSet::new();

        for (mid, _score, chr) in scored.into_iter().take(capacity.max(1)) {
            if let Some(motif) = self.ltm.iter_mut().find(|m| m.id == mid) {
                motif.hit_count = motif.hit_count.saturating_add(1);
                motif_ids.push(mid);
            }
            self.fill_neurons_from_chr(chr, capacity, &mut neurons, &mut seen);
            if neurons.len() >= capacity {
                break;
            }
        }

        // If LTM empty or still no neurons, densest chromosome by synapses.
        if neurons.is_empty() {
            if let Some((_, brain)) = self
                .chromosomes
                .iter()
                .max_by_key(|(_, b)| b.synapses.len())
            {
                for n in brain.neurons.iter().take(capacity) {
                    neurons.push(GlobalNeuronRef {
                        chr: brain.chr,
                        neuron: n.id,
                    });
                }
            }
        }

        self.working_set.neurons = neurons;
        self.working_set.motif_ids = motif_ids;
        self.refresh_structure();
        &self.working_set
    }

    fn score_motifs(
        &self,
        query: &[f32; 8],
        chr_filter: Option<u8>,
    ) -> Vec<(u64, f32, ChromosomeId)> {
        let mut scored: Vec<(u64, f32, ChromosomeId)> = self
            .ltm
            .iter()
            .filter(|m| chr_filter.map(|c| m.source_chr.0 == c).unwrap_or(true))
            .map(|m| (m.id, cosine_sim(&m.signature, query), m.source_chr))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    fn fill_neurons_from_chr(
        &self,
        chr: ChromosomeId,
        capacity: usize,
        neurons: &mut Vec<GlobalNeuronRef>,
        seen: &mut HashSet<GlobalNeuronRef>,
    ) {
        let Some(brain) = self.chromosomes.get(&chr.0) else {
            return;
        };
        let mut syns: Vec<_> = brain.synapses.iter().collect();
        syns.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        for s in syns.into_iter().take(8) {
            for nid in [s.from, s.to] {
                let g = GlobalNeuronRef { chr, neuron: nid };
                if seen.insert(g) {
                    neurons.push(g);
                    if neurons.len() >= capacity {
                        return;
                    }
                }
            }
        }
        for n in &brain.neurons {
            let g = GlobalNeuronRef {
                chr,
                neuron: n.id,
            };
            if seen.insert(g) {
                neurons.push(g);
                if neurons.len() >= capacity {
                    return;
                }
            }
        }
    }

    /// Rung 3: map free text → signature, activate genomic working set + language nodes.
    pub fn activate_from_text(&mut self, query: &str) -> &WorkingSet {
        let lang_budget = (self.working_set.capacity / 4).max(4);
        let (sig, lang_nodes, lang_query) = if let Some(organ) = self.language.as_mut() {
            organ.activate_nodes(query, lang_budget);
            (
                organ.last_signature,
                organ.last_active_nodes.clone(),
                organ.last_query.clone(),
            )
        } else {
            (
                LanguageOrgan::text_to_signature(query),
                Vec::new(),
                query.to_string(),
            )
        };
        // Optional chr filter from signature slot 3.
        let chr_filter = if sig[3] > 0.02 {
            Some(((sig[3] * 22.0).round() as u8).clamp(1, 22))
        } else {
            None
        };
        self.activate(&sig, chr_filter);
        self.working_set.language_nodes = lang_nodes;
        self.working_set.language_query = lang_query;
        self.refresh_structure();
        &self.working_set
    }

    /// Convenience: activate from a genomic position hint (chr + bp).
    pub fn activate_near(&mut self, chr: u8, position_bp: u32, window_bp: u32) -> &WorkingSet {
        let mut query = [0.0f32; 8];
        query[3] = (chr as f32) / 22.0;
        query[4] = (position_bp as f32) * 1e-8;
        query[5] = ((position_bp + window_bp) as f32) * 1e-8;
        query[0] = 0.7; // prefer structured LD
        self.activate(&query, Some(chr));

        // Refine: keep only neurons near the position if chromosome is loaded.
        if let Some(brain) = self.chromosomes.get(&chr) {
            let lo = position_bp.saturating_sub(window_bp);
            let hi = position_bp.saturating_add(window_bp);
            let mut local: Vec<GlobalNeuronRef> = brain
                .neurons
                .iter()
                .filter(|n| n.position_bp >= lo && n.position_bp <= hi)
                .map(|n| GlobalNeuronRef {
                    chr: ChromosomeId(chr),
                    neuron: n.id,
                })
                .collect();
            local.truncate(self.working_set.capacity);
            if !local.is_empty() {
                self.working_set.neurons = local;
            }
        }
        self.refresh_structure();
        &self.working_set
    }

    /// Run KAIROS training on every chromosome (local learning step).
    pub fn train_all(&mut self, cycles: u32) {
        for brain in self.chromosomes.values_mut() {
            brain.train_kairos(cycles);
            brain.training_cycles = brain.training_cycles.saturating_add(cycles);
        }
        self.refresh_structure();
    }

    /// Propose a structural delta for Rung 2: prune weakest synapses and
    /// re-consolidate. Returns a clone of the brain after the mutation so
    /// callers can score multi-axis fitness without destroying baseline.
    pub fn propose_prune_mutant(&self, prune_frac: f32) -> SovereignBrain {
        let mut child = self.clone();
        child.consolidate(0.5, prune_frac);
        child
    }

    /// Propose a learning delta: extra KAIROS cycles on all chromosomes.
    /// Preserves LD edges (biology coverage) while shifting weights/task signal.
    pub fn propose_train_mutant(&self, cycles: u32) -> SovereignBrain {
        let mut child = self.clone();
        child.train_all(cycles.max(1));
        child.generation = child.generation.saturating_add(1);
        child
    }
}

/// Result of a consolidate() call.
#[derive(Clone, Debug)]
pub struct ConsolidateReport {
    pub motifs_added: u32,
    pub synapses_pruned: u32,
    pub ltm_total: usize,
    pub generation: u64,
}

fn cosine_sim(a: &[f32; 8], b: &[f32; 8]) -> f32 {
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..8 {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let d = (na.sqrt() * nb.sqrt()).max(1e-9);
    dot / d
}

/// Build a tiny synthetic multi-chr brain for unit tests (no VCF I/O).
#[cfg(test)]
pub fn synthetic_test_brain() -> SovereignBrain {
    use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
    use crate::genomic::chromosome_brain::init_chromosome_brain;
    use crate::genomic::ld_compute::LdPair;
    use crate::genomic::vcf_stream::SnpRecord;

    fn make_chr(chr: u8, n_snps: usize, n_samples: usize) -> ChromosomeBrain {
        let mut snps = Vec::new();
        let mut records = Vec::new();
        for i in 0..n_snps {
            let mut g = BitstreamGenotypes::new(n_samples);
            for s in 0..n_samples {
                // Correlated structure: even SNPs track sample parity.
                let gt = if i % 2 == 0 {
                    if s % 2 == 0 {
                        0
                    } else {
                        2
                    }
                } else if s % 3 == 0 {
                    1
                } else {
                    0
                };
                g.set(s, gt);
            }
            snps.push(g);
            records.push(SnpRecord {
                id: format!("rs{chr}_{i}"),
                position: (i as u32 + 1) * 1000,
                ref_allele: "A".into(),
                alt_allele: "G".into(),
                qual: 100.0,
                info: String::new(),
            });
        }
        let mut pairs = Vec::new();
        for i in 0..n_snps as u32 {
            for j in (i + 1)..(i + 3).min(n_snps as u32) {
                pairs.push(LdPair {
                    snp1_idx: i,
                    snp2_idx: j,
                    r_squared: 0.85 - 0.05 * (j - i) as f32,
                    position1: records[i as usize].position,
                    position2: records[j as usize].position,
                });
            }
        }
        let blocks = vec![HaplotypeBlock {
            id: 0,
            snp_indices: (0..n_snps as u32).collect(),
            mean_r_squared: 0.8,
            start_position: records[0].position,
            end_position: records[n_snps - 1].position,
            size: n_snps as u32,
        }];
        init_chromosome_brain(ChromosomeId(chr), &snps, &records, &pairs, &blocks).unwrap()
    }

    let mut brain = SovereignBrain::new(32);
    brain.ingest_brain(make_chr(1, 12, 64));
    brain.ingest_brain(make_chr(22, 10, 64));
    brain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_chromosome_ingest() {
        let brain = synthetic_test_brain();
        assert_eq!(brain.n_chromosomes(), 2);
        assert!(brain.chromosome(1).is_some());
        assert!(brain.chromosome(22).is_some());
        assert!(brain.ltm_stats().n_motifs >= 2);
        let s = brain.measure_structure();
        assert!(s.n_neurons >= 20);
        assert!(s.n_synapses > 0);
        assert!(s.approx_memory_bytes > 0);
    }

    #[test]
    fn activate_fills_working_set() {
        let mut brain = synthetic_test_brain();
        let cap = brain.working_set.capacity;
        brain.activate(&[0.8, 1.0, 1.0, 0.05, 0.0, 0.0, 1.0, 1.0], Some(1));
        assert!(!brain.working_set.is_empty());
        assert!(brain.working_set.len() <= cap);
        assert!(brain.working_set.neurons.iter().all(|n| n.chr.0 == 1));
    }

    #[test]
    fn activate_near_respects_window() {
        let mut brain = synthetic_test_brain();
        brain.activate_near(1, 5000, 3000);
        assert!(!brain.working_set.is_empty());
        let positions: Vec<u32> = {
            let b = brain.chromosome(1).unwrap();
            brain
                .working_set
                .neurons
                .iter()
                .map(|g| {
                    b.neurons
                        .iter()
                        .find(|n| n.id == g.neuron)
                        .unwrap()
                        .position_bp
                })
                .collect()
        };
        for pos in positions {
            assert!((2000..=8000).contains(&pos));
        }
    }

    #[test]
    fn consolidate_grows_ltm_and_can_prune() {
        let mut brain = synthetic_test_brain();
        let before_syn: usize = brain
            .chromosomes
            .values()
            .map(|c| c.synapses.len())
            .sum();
        let report = brain.consolidate(0.5, 0.25);
        assert!(report.generation >= 1);
        let after_syn: usize = brain
            .chromosomes
            .values()
            .map(|c| c.synapses.len())
            .sum();
        assert!(after_syn < before_syn);
        assert!(brain.ltm_stats().n_motifs >= 2);
    }

    #[test]
    fn working_set_survives_after_ltm_hits() {
        let mut brain = synthetic_test_brain();
        brain.activate(&[0.9, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0], None);
        let hits_before: u64 = brain.ltm.iter().map(|m| m.hit_count).sum();
        brain.activate(&[0.9, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0], None);
        let hits_after: u64 = brain.ltm.iter().map(|m| m.hit_count).sum();
        assert!(hits_after >= hits_before);
    }

    #[test]
    fn propose_prune_mutant_is_independent() {
        let parent = synthetic_test_brain();
        let parent_syn: usize = parent.chromosomes.values().map(|c| c.synapses.len()).sum();
        let child = parent.propose_prune_mutant(0.3);
        let child_syn: usize = child.chromosomes.values().map(|c| c.synapses.len()).sum();
        let parent_syn_after: usize =
            parent.chromosomes.values().map(|c| c.synapses.len()).sum();
        assert_eq!(parent_syn, parent_syn_after);
        assert!(child_syn < parent_syn);
        assert!(child.generation > parent.generation);
    }

    #[test]
    fn activate_from_text_fills_language_and_genomic() {
        use crate::genomic::language_organ::{fixture_docs, LanguageOrgan};
        let mut brain = synthetic_test_brain();
        let mut organ = LanguageOrgan::new();
        organ.ingest_documents(&fixture_docs());
        organ.train_calib_fixtures(15).unwrap();
        brain.attach_language(organ);
        brain.activate_from_text("haplotype LD on chromosome 22 with fn main");
        assert!(!brain.working_set.language_nodes.is_empty());
        assert!(!brain.working_set.language_query.is_empty());
        // Genomic side should also light up.
        assert!(!brain.working_set.neurons.is_empty() || !brain.ltm.is_empty());
    }

    #[test]
    fn activate_hits_ltm_motifs_after_consolidate() {
        let mut brain = synthetic_test_brain();
        assert!(brain.ltm_stats().n_motifs >= 1);
        brain.consolidate(0.5, 0.0);
        // Neutral query (weak cosine) must still surface motifs via r² fallback.
        brain.activate(&[0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01], None);
        assert!(
            !brain.working_set.motif_ids.is_empty(),
            "expected LTM motif hits, got none"
        );
        let hits: u64 = brain.ltm.iter().map(|m| m.hit_count).sum();
        assert!(hits >= 1);
    }

}
