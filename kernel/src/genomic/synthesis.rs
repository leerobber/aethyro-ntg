/// Phase C: Synthetic Genome Synthesis
///
/// Two sampling modes:
/// - Independent per-locus (the original approach, still the fallback):
///   each SNP drawn under Hardy-Weinberg at its own real allele frequency
///   (via `from_brain`), with no cross-locus correlation. Matches a real
///   reference's allele frequencies but not its LD pattern -- Phase D's
///   real-vs-synthetic validation surfaced this directly (allele-frequency
///   RMSE small, LD correlation near zero).
/// - Haplotype-block-based (`from_brain_with_haplotypes`, using
///   `HaplotypePool`): real 1000 Genomes VCFs are phased ("0|1", not
///   "0/1" -- see `VcfParser::parse_vcf_phased_limited`), so for each
///   `HaplotypeBlock` we can extract the *actual* observed haplotype
///   fragments real samples carry across that block's SNPs, and
///   synthesize new individuals by drawing two real fragments (with
///   replacement) per block, the same way a real diploid genome is two
///   inherited chromosome copies. This preserves real within-block LD by
///   construction, because it isn't modeling LD from summary statistics
///   at all -- it's recombining real co-inherited fragments. SNPs not
///   covered by any block (or when no phased data was supplied) still
///   fall back to independent per-locus sampling. Cross-block LD is not
///   modeled either way; blocks are themselves defined as maximal
///   LD-connected components, so that's a reasonable simplification, not
///   an omission of anything the block structure itself would capture.
/// Pure Rust implementation

use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use crate::genomic::chromosome_brain::ChromosomeBrain;
use crate::genomic::haplotype_blocks::HaplotypeBlock;
use std::collections::HashMap;

/// Probability that a haplotype copy continues with the same real donor
/// when moving from one block to the genomically-next one, rather than
/// drawing a fresh independent donor (see `GenomeSampler::sample`'s
/// donor-persistence logic). A tunable heuristic, not derived from any
/// real recombination-rate data.
const DONOR_PERSISTENCE: f32 = 0.7;

/// Real observed haplotype fragments for one `HaplotypeBlock`, used to
/// synthesize genomes that preserve that block's real LD structure
/// instead of sampling each locus independently.
#[derive(Clone, Debug)]
pub struct HaplotypePool {
    /// Global SNP indices covered by this pool, in the same order as
    /// each entry in `observed_haplotypes`.
    pub snp_indices: Vec<usize>,
    /// Each entry is one real chromosome copy's alleles (0/1) across
    /// `snp_indices`, drawn from real phased samples. Up to
    /// `2 * n_real_samples` entries; a copy is skipped entirely (not
    /// filled with a guess) if it had a missing call anywhere in this
    /// block's SNPs.
    pub observed_haplotypes: Vec<Vec<u8>>,
    /// Real (sample_idx, is_copy_b) provenance for each entry in
    /// `observed_haplotypes`, same order/index. Used by
    /// `GenomeSampler::sample`'s donor-persistence logic to let adjacent
    /// blocks continue with the same real donor instead of always
    /// resampling independently at every block boundary (which mimics an
    /// artificial recombination event at every single boundary).
    pub donor_ids: Vec<(usize, bool)>,
    donor_index: HashMap<(usize, bool), usize>,
}

impl HaplotypePool {
    /// Build a pool for one block from phased VCF data (`hap_a`/`hap_b`:
    /// one `BitstreamGenotypes` per SNP, indexed the same way as
    /// `VcfChromosome::genotypes`).
    pub fn from_phased(
        block: &HaplotypeBlock,
        hap_a: &[BitstreamGenotypes],
        hap_b: &[BitstreamGenotypes],
        n_real_samples: usize,
    ) -> Self {
        let snp_indices: Vec<usize> = block.snp_indices.iter().map(|&i| i as usize).collect();
        let mut observed_haplotypes = Vec::new();
        let mut donor_ids = Vec::new();

        for sample_idx in 0..n_real_samples {
            for (is_b, hap) in [(false, hap_a), (true, hap_b)] {
                let mut fragment = Vec::with_capacity(snp_indices.len());
                let mut ok = true;
                for &snp_idx in &snp_indices {
                    let Some(snp_hap) = hap.get(snp_idx) else {
                        ok = false;
                        break;
                    };
                    let allele = snp_hap.get(sample_idx);
                    if allele > 1 {
                        // 3 = missing on this copy (2 is never written by
                        // the phased parser for a single haplotype strand).
                        ok = false;
                        break;
                    }
                    fragment.push(allele);
                }
                if ok {
                    observed_haplotypes.push(fragment);
                    donor_ids.push((sample_idx, is_b));
                }
            }
        }

        let donor_index: HashMap<(usize, bool), usize> = donor_ids
            .iter()
            .enumerate()
            .map(|(idx, &donor)| (donor, idx))
            .collect();

        HaplotypePool {
            snp_indices,
            observed_haplotypes,
            donor_ids,
            donor_index,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.observed_haplotypes.is_empty()
    }

    fn pick_index(&self, unit_draw: f32) -> usize {
        let n = self.observed_haplotypes.len();
        ((unit_draw.clamp(0.0, 0.999_999) * n as f32) as usize).min(n.saturating_sub(1))
    }

    /// Pick one observed haplotype fragment via an external draw in
    /// [0, 1); the caller supplies randomness so this stays deterministic
    /// and reproducible given the same seed, matching the rest of this
    /// module.
    pub fn pick(&self, unit_draw: f32) -> &[u8] {
        &self.observed_haplotypes[self.pick_index(unit_draw)]
    }

    /// Fragment index contributed by real donor (sample_idx, is_copy_b),
    /// if this pool has one (it won't if that copy had a missing call
    /// anywhere in this block, or the donor doesn't exist at this scale).
    pub fn find_donor(&self, donor: (usize, bool)) -> Option<usize> {
        self.donor_index.get(&donor).copied()
    }
}

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
    /// Real-haplotype pools for LD-preserving sampling (see module doc).
    /// Empty means "no phased data supplied" -- every locus falls back to
    /// independent per-locus sampling, the original behavior.
    pub haplotype_pools: Vec<HaplotypePool>,
}

impl GenomeSampler {
    pub fn new(n_snps: usize, n_samples: usize, seed: u64) -> Self {
        Self {
            n_snps,
            n_samples,
            seed,
            allele_freqs: Vec::new(),
            haplotype_pools: Vec::new(),
        }
    }

    /// Build a sampler with explicit per-locus allele frequencies (and no
    /// haplotype pools). Mainly for tests; `from_brain`/
    /// `from_brain_with_haplotypes` are the real-data constructors.
    pub fn with_allele_freqs(n_samples: usize, seed: u64, allele_freqs: Vec<f32>) -> Self {
        Self {
            n_snps: allele_freqs.len(),
            n_samples,
            seed,
            allele_freqs,
            haplotype_pools: Vec::new(),
        }
    }

    /// Build a sampler whose per-locus allele frequencies come from a real
    /// `ChromosomeBrain` (i.e. the real 1000 Genomes-derived MAF at each
    /// SNP, via `GenomicNeuron::allele_freq`), instead of one flat
    /// assumed frequency for the whole chromosome. Samples each locus
    /// independently -- use `from_brain_with_haplotypes` to also preserve
    /// real LD structure within haplotype blocks.
    pub fn from_brain(brain: &ChromosomeBrain, n_samples: usize, seed: u64) -> Self {
        let allele_freqs: Vec<f32> = brain.neurons.iter().map(|n| n.allele_freq).collect();
        Self {
            n_snps: allele_freqs.len(),
            n_samples,
            seed,
            allele_freqs,
            haplotype_pools: Vec::new(),
        }
    }

    /// Same as `from_brain`, but also builds a `HaplotypePool` per block
    /// from real phased VCF data (`hap_a`/`hap_b`, from
    /// `VcfParser::parse_vcf_phased_limited`), so SNPs inside a block are
    /// sampled by drawing real haplotype fragments instead of
    /// independently. SNPs the brain's blocks don't cover still fall back
    /// to independent per-locus sampling.
    pub fn from_brain_with_haplotypes(
        brain: &ChromosomeBrain,
        hap_a: &[BitstreamGenotypes],
        hap_b: &[BitstreamGenotypes],
        n_real_samples: usize,
        n_samples: usize,
        seed: u64,
    ) -> Self {
        let allele_freqs: Vec<f32> = brain.neurons.iter().map(|n| n.allele_freq).collect();
        let haplotype_pools: Vec<HaplotypePool> = brain
            .blocks
            .iter()
            .filter(|b| b.snp_indices.len() > 1) // singleton blocks carry no LD to preserve
            .map(|b| HaplotypePool::from_phased(b, hap_a, hap_b, n_real_samples))
            .filter(|p| !p.is_empty())
            .collect();
        Self {
            n_snps: allele_freqs.len(),
            n_samples,
            seed,
            allele_freqs,
            haplotype_pools,
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

    /// Choose a fragment for one haplotype copy within a pool. With
    /// `DONOR_PERSISTENCE` probability, continues with `current_donor` if
    /// that real donor also has a fragment in this pool; otherwise (no
    /// current donor, donor absent from this pool, or the persistence
    /// roll fails) draws a fresh fragment. Returns the chosen fragment's
    /// index and its donor identity, so the caller can carry it forward
    /// into the next pool.
    fn choose_fragment(
        pool: &HaplotypePool,
        current_donor: Option<(usize, bool)>,
        seed: u64,
        genome_id: u32,
        draw_salt: u32,
        persist_salt: u32,
        sample_idx: u32,
    ) -> (usize, (usize, bool)) {
        if let Some(donor) = current_donor {
            if let Some(frag_idx) = pool.find_donor(donor) {
                let persist_draw = Self::hash_unit_interval(seed, genome_id, persist_salt, sample_idx);
                if persist_draw < DONOR_PERSISTENCE {
                    return (frag_idx, donor);
                }
            }
        }
        let draw = Self::hash_unit_interval(seed, genome_id, draw_salt, sample_idx);
        let frag_idx = pool.pick_index(draw);
        (frag_idx, pool.donor_ids[frag_idx])
    }

    /// Sample a single synthetic genome
    pub fn sample(&self, genome_id: u32) -> Genome {
        let mut genome = Genome::new(genome_id, self.n_snps, self.n_samples);
        let mut covered = vec![false; self.n_snps];

        // Process non-empty pools in genomic order (by their smallest SNP
        // index) so donor persistence (below) means "the block
        // immediately before this one," not an arbitrary construction
        // order.
        let mut pool_order: Vec<usize> = (0..self.haplotype_pools.len())
            .filter(|&i| !self.haplotype_pools[i].is_empty())
            .collect();
        pool_order.sort_by_key(|&i| {
            self.haplotype_pools[i]
                .snp_indices
                .iter()
                .min()
                .copied()
                .unwrap_or(usize::MAX)
        });

        for &pool_idx in &pool_order {
            for &snp_idx in &self.haplotype_pools[pool_idx].snp_indices {
                if snp_idx < covered.len() {
                    covered[snp_idx] = true;
                }
            }
        }

        // Haplotype-block-based sampling: draw two real observed
        // haplotype fragments per synthetic sample per block (mirroring
        // diploid inheritance), preserving that block's real LD by
        // construction. Pool draws use a SNP-index namespace disjoint
        // from real SNP indices (counting down from u32::MAX) so they
        // never coincide with the independent-locus draws below, which
        // would otherwise silently correlate two unrelated loci.
        //
        // Donor persistence: resampling a fully independent donor at
        // every block boundary mimics an artificial recombination event
        // at every single boundary, even where the real chromosome had
        // none. With DONOR_PERSISTENCE probability, each haplotype copy
        // continues with the same real donor into the next block (when
        // that donor also has a fragment there) instead of always
        // drawing fresh. This is a heuristic, not derived from any real
        // recombination-rate data (no genetic map is available in this
        // pipeline -- same caveat as extended_validation.rs's
        // recombination-rate proxy).
        for sample_idx in 0..self.n_samples {
            let mut current_donor_a: Option<(usize, bool)> = None;
            let mut current_donor_b: Option<(usize, bool)> = None;

            for &pool_idx in &pool_order {
                let pool = &self.haplotype_pools[pool_idx];
                let salt_base = u32::MAX - (pool_idx as u32) * 4;

                let (idx_a, donor_a) = Self::choose_fragment(
                    pool,
                    current_donor_a,
                    self.seed,
                    genome_id,
                    salt_base,
                    salt_base.wrapping_sub(1),
                    sample_idx as u32,
                );
                let (idx_b, donor_b) = Self::choose_fragment(
                    pool,
                    current_donor_b,
                    self.seed,
                    genome_id,
                    salt_base.wrapping_sub(2),
                    salt_base.wrapping_sub(3),
                    sample_idx as u32,
                );

                let hap_a = &pool.observed_haplotypes[idx_a];
                let hap_b = &pool.observed_haplotypes[idx_b];

                for (local_idx, &snp_idx) in pool.snp_indices.iter().enumerate() {
                    if snp_idx >= self.n_snps {
                        continue;
                    }
                    genome.genotypes[snp_idx][sample_idx] = hap_a[local_idx] + hap_b[local_idx];
                }

                current_donor_a = Some(donor_a);
                current_donor_b = Some(donor_b);
            }
        }

        // Independent per-locus fallback for everything a pool didn't
        // cover (no phased data supplied, or a singleton SNP outside any
        // block).
        for snp_idx in 0..self.n_snps {
            if covered[snp_idx] {
                continue;
            }
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
        // rare vs. common, not the old flat 0.3
        let sampler = GenomeSampler::with_allele_freqs(2000, 7, vec![0.05, 0.95]);

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

    fn perfectly_linked_block() -> (HaplotypeBlock, Vec<BitstreamGenotypes>, Vec<BitstreamGenotypes>) {
        // 8 real samples (16 haplotype copies). Every real copy is either
        // ref/ref at both SNPs or alt/alt at both -- the two loci always
        // travel together, i.e. perfect LD (r² = 1) in the real data.
        let n_samples = 8;
        let mut hap_a0 = BitstreamGenotypes::new(n_samples);
        let mut hap_b0 = BitstreamGenotypes::new(n_samples);
        let mut hap_a1 = BitstreamGenotypes::new(n_samples);
        let mut hap_b1 = BitstreamGenotypes::new(n_samples);

        for sample_idx in 0..n_samples {
            // Alternate ref-carrying / alt-carrying samples so the pool
            // has real variety in both directions, not a monomorphic site.
            let carries_alt = sample_idx % 2 == 0;
            let allele = if carries_alt { 1 } else { 0 };
            hap_a0.set(sample_idx, allele);
            hap_b0.set(sample_idx, allele);
            hap_a1.set(sample_idx, allele); // SNP1 always matches SNP0
            hap_b1.set(sample_idx, allele);
        }

        let block = HaplotypeBlock {
            id: 0,
            snp_indices: vec![0, 1],
            mean_r_squared: 1.0,
            start_position: 100,
            end_position: 200,
            size: 2,
        };

        (block, vec![hap_a0, hap_a1], vec![hap_b0, hap_b1])
    }

    #[test]
    fn test_haplotype_pool_from_phased_extracts_real_fragments() {
        let (block, hap_a, hap_b) = perfectly_linked_block();
        let pool = HaplotypePool::from_phased(&block, &hap_a, &hap_b, 8);

        assert_eq!(pool.snp_indices, vec![0, 1]);
        // 8 samples * 2 copies = 16 real haplotype fragments, none missing.
        assert_eq!(pool.observed_haplotypes.len(), 16);
        // Every fragment has the two loci matching (perfectly linked).
        for frag in &pool.observed_haplotypes {
            assert_eq!(frag.len(), 2);
            assert_eq!(frag[0], frag[1], "fragment {:?} breaks perfect linkage", frag);
        }
    }

    #[test]
    fn test_haplotype_pool_skips_missing_calls() {
        let n_samples = 4;
        let mut hap_a0 = BitstreamGenotypes::new(n_samples);
        let mut hap_b0 = BitstreamGenotypes::new(n_samples);
        for i in 0..n_samples {
            hap_a0.set(i, 0);
            hap_b0.set(i, 0);
        }
        hap_a0.set(1, 3); // missing on this copy for sample 1

        let block = HaplotypeBlock {
            id: 0,
            snp_indices: vec![0],
            mean_r_squared: 0.0,
            start_position: 0,
            end_position: 0,
            size: 1,
        };

        let pool = HaplotypePool::from_phased(&block, &[hap_a0], &[hap_b0], n_samples);
        // 4 samples * 2 copies = 8, minus the 1 missing copy = 7.
        assert_eq!(pool.observed_haplotypes.len(), 7);
    }

    #[test]
    fn test_haplotype_based_sampling_preserves_real_ld() {
        let (block, hap_a, hap_b) = perfectly_linked_block();

        let mut sampler = GenomeSampler::with_allele_freqs(300, 11, vec![0.5, 0.5]);
        sampler.haplotype_pools = vec![HaplotypePool::from_phased(&block, &hap_a, &hap_b, 8)];

        let genome = sampler.sample(0);
        let ld = genome.ld_r2(0, 1);

        assert!(ld > 0.9, "expected near-perfect LD preserved from real haplotypes, got {ld}");
    }

    #[test]
    fn test_independent_sampling_without_pools_does_not_preserve_ld() {
        // Same target allele frequencies as the perfectly-linked block
        // above, but with no haplotype pool -- this is the pre-existing
        // behavior, kept as a contrast so the difference above is
        // attributable to the new mechanism, not to the specific
        // frequencies chosen.
        let sampler = GenomeSampler::with_allele_freqs(300, 11, vec![0.5, 0.5]);
        let genome = sampler.sample(0);
        let ld = genome.ld_r2(0, 1);

        assert!(ld < 0.3, "expected near-zero LD from independent sampling, got {ld}");
    }

    #[test]
    fn test_from_brain_with_haplotypes_skips_singleton_blocks() {
        use crate::genomic::chromosome_brain::{
            ChromosomeId as ChrId, EmbeddingLayer, GenomicNeuron, KairosState, NeuronId as NId,
        };

        let (block, hap_a, hap_b) = perfectly_linked_block();
        let singleton = HaplotypeBlock {
            id: 1,
            snp_indices: vec![2],
            mean_r_squared: 0.0,
            start_position: 300,
            end_position: 300,
            size: 1,
        };

        let brain = ChromosomeBrain {
            chr: ChrId(1),
            neurons: vec![
                GenomicNeuron { id: NId(0), snp_index: 0, position_bp: 100, allele_freq: 0.5, maf: 0.5, is_rare: false },
                GenomicNeuron { id: NId(1), snp_index: 1, position_bp: 200, allele_freq: 0.5, maf: 0.5, is_rare: false },
                GenomicNeuron { id: NId(2), snp_index: 2, position_bp: 300, allele_freq: 0.5, maf: 0.5, is_rare: false },
            ],
            synapses: vec![],
            blocks: vec![block, singleton],
            embeddings: EmbeddingLayer { snp_embeddings: vec![], block_embeddings: vec![], consolidated: vec![] },
            training_cycles: 0,
            kairos_state: KairosState::default(),
        };

        let sampler = GenomeSampler::from_brain_with_haplotypes(&brain, &hap_a, &hap_b, 8, 300, 11);

        assert_eq!(sampler.haplotype_pools.len(), 1, "singleton block should not produce a pool");
        assert_eq!(sampler.haplotype_pools[0].snp_indices, vec![0, 1]);

        // SNP 2 (the singleton) isn't pool-covered, so it must still fall
        // back to independent sampling rather than being left at zero.
        let genome = sampler.sample(0);
        let freq2 = genome.allele_freq(2);
        assert!((freq2 - 0.5).abs() < 0.15, "expected fallback sampling near target freq, got {freq2}");
    }

    fn two_donor_pool() -> HaplotypePool {
        let donor_ids = vec![(0usize, false), (1usize, false)];
        let donor_index = donor_ids.iter().copied().enumerate().map(|(i, d)| (d, i)).collect();
        HaplotypePool {
            snp_indices: vec![0],
            observed_haplotypes: vec![vec![0u8], vec![1u8]],
            donor_ids,
            donor_index,
        }
    }

    #[test]
    fn test_find_donor() {
        let pool = two_donor_pool();
        assert_eq!(pool.find_donor((0, false)), Some(0));
        assert_eq!(pool.find_donor((1, false)), Some(1));
        assert_eq!(pool.find_donor((2, false)), None);
        assert_eq!(pool.find_donor((0, true)), None); // different copy, not in this pool
    }

    #[test]
    fn test_choose_fragment_no_current_donor_draws_fresh() {
        let pool = two_donor_pool();
        // No current donor: must draw via draw_salt, never touch persist_salt's path.
        let (frag_idx, donor) = GenomeSampler::choose_fragment(&pool, None, 42, 0, 100, 101, 0);
        assert_eq!(pool.donor_ids[frag_idx], donor);
    }

    #[test]
    fn test_choose_fragment_absent_donor_falls_back_to_fresh_draw() {
        let pool = two_donor_pool();
        // current_donor (5, false) doesn't exist in this pool.
        let (frag_idx, donor) = GenomeSampler::choose_fragment(&pool, Some((5, false)), 42, 0, 100, 101, 0);
        assert_eq!(pool.donor_ids[frag_idx], donor);
        assert_ne!(donor, (5, false));
    }

    #[test]
    fn test_choose_fragment_persists_at_expected_rate() {
        // 100 donors: a "fresh" fallback draw has only ~1% chance of
        // coincidentally landing back on the same donor, so the observed
        // rate closely tracks DONOR_PERSISTENCE itself rather than being
        // inflated by chance re-picks (as it would be with very few
        // donors -- e.g. 2 donors gives a fresh-draw coincidence rate of
        // 50%, which dominates the measurement).
        let n_donors = 100;
        let donor_ids: Vec<(usize, bool)> = (0..n_donors).map(|i| (i, false)).collect();
        let donor_index = donor_ids.iter().copied().enumerate().map(|(i, d)| (d, i)).collect();
        let pool = HaplotypePool {
            snp_indices: vec![0],
            observed_haplotypes: (0..n_donors).map(|i| vec![(i % 2) as u8]).collect(),
            donor_ids,
            donor_index,
        };

        let seed = 99u64;
        let genome_id = 0u32;
        let n = 5000u32;
        let mut persisted = 0u32;

        for sample_idx in 0..n {
            let (_, donor) =
                GenomeSampler::choose_fragment(&pool, Some((0, false)), seed, genome_id, 100, 101, sample_idx);
            if donor == (0, false) {
                persisted += 1;
            }
        }

        let rate = persisted as f32 / n as f32;
        assert!(
            (rate - DONOR_PERSISTENCE).abs() < 0.05,
            "expected persistence rate near {DONOR_PERSISTENCE}, got {rate}"
        );
    }

    #[test]
    fn test_donor_persistence_reduces_switch_rate_across_adjacent_blocks() {
        // Two adjacent blocks where every real donor has a fragment in
        // both -- the scenario donor persistence exists for.
        let n_samples = 40;
        let mut hap_a0 = BitstreamGenotypes::new(n_samples);
        let mut hap_b0 = BitstreamGenotypes::new(n_samples);
        let mut hap_a1 = BitstreamGenotypes::new(n_samples);
        let mut hap_b1 = BitstreamGenotypes::new(n_samples);
        for i in 0..n_samples {
            let v = (i % 2) as u8;
            hap_a0.set(i, v);
            hap_b0.set(i, v);
            hap_a1.set(i, v);
            hap_b1.set(i, v);
        }

        let block0 = HaplotypeBlock {
            id: 0,
            snp_indices: vec![0, 1],
            mean_r_squared: 1.0,
            start_position: 0,
            end_position: 100,
            size: 2,
        };
        let block1 = HaplotypeBlock {
            id: 1,
            snp_indices: vec![2, 3],
            mean_r_squared: 1.0,
            start_position: 200,
            end_position: 300,
            size: 2,
        };

        let hap_a = vec![hap_a0.clone(), hap_a0, hap_a1.clone(), hap_a1];
        let hap_b = vec![hap_b0.clone(), hap_b0, hap_b1.clone(), hap_b1];

        let pool0 = HaplotypePool::from_phased(&block0, &hap_a, &hap_b, n_samples);
        let pool1 = HaplotypePool::from_phased(&block1, &hap_a, &hap_b, n_samples);

        let n_trials = 3000u32;
        let mut same_donor_count = 0u32;
        for sample_idx in 0..n_trials {
            let (_, donor0) =
                GenomeSampler::choose_fragment(&pool0, None, 7, 0, 200, 201, sample_idx);
            let (_, donor1) =
                GenomeSampler::choose_fragment(&pool1, Some(donor0), 7, 0, 300, 301, sample_idx);
            if donor0 == donor1 {
                same_donor_count += 1;
            }
        }

        let observed_rate = same_donor_count as f32 / n_trials as f32;
        // With full donor overlap between pools, observed same-donor rate
        // should track persistence + the small chance a fresh draw lands
        // on the same donor by chance (~1/(2*n_samples) here) -- well
        // above what a fully independent draw (no persistence) would give.
        assert!(
            observed_rate > DONOR_PERSISTENCE * 0.8,
            "expected same-donor rate near {DONOR_PERSISTENCE}, got {observed_rate}"
        );

        let independent_baseline = 1.0 / (2.0 * n_samples as f32);
        assert!(
            observed_rate > independent_baseline * 5.0,
            "persistence should be clearly better than independent resampling's baseline {independent_baseline}, got {observed_rate}"
        );
    }
}
