/// Phase E: Extended Validation
/// Genome-wide validation across all 22 chromosomes, multi-population
/// reference comparison (EUR/AFR/ASN), locus-specific statistical power,
/// recombination-rate matching, and haplotype block structure comparison.
/// Builds on Phase D (quality_control, validation) and Phase A
/// (haplotype_blocks, ld_compute). Pure Rust, no dependencies.

use std::collections::{HashMap, HashSet};

use crate::genomic::chromosome_brain::ChromosomeId;
use crate::genomic::haplotype_blocks::{
    compute_block_statistics, BlockDetector, BlockStatistics, HaplotypeBlock,
};
use crate::genomic::ld_compute::LdPair;
use crate::genomic::quality_control::{LocusStats, QCMetrics};
use crate::genomic::validation::{
    GenomeComparator, PowerAnalysis, ReferenceGenome, SyntheticGenome, ValidationResults,
};

/// 1000 Genomes-style super-population ancestry groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Population {
    Eur,
    Afr,
    Asn,
}

impl Population {
    pub fn label(&self) -> &'static str {
        match self {
            Population::Eur => "EUR",
            Population::Afr => "AFR",
            Population::Asn => "ASN",
        }
    }

    pub fn all() -> [Population; 3] {
        [Population::Eur, Population::Afr, Population::Asn]
    }
}

/// A reference panel spanning multiple ancestry populations over the same SNP set.
#[derive(Clone, Debug, Default)]
pub struct MultiPopulationReference {
    pub panels: HashMap<Population, ReferenceGenome>,
}

impl MultiPopulationReference {
    pub fn new() -> Self {
        MultiPopulationReference {
            panels: HashMap::new(),
        }
    }

    pub fn add_population(&mut self, pop: Population, reference: ReferenceGenome) {
        self.panels.insert(pop, reference);
    }

    pub fn n_populations(&self) -> usize {
        self.panels.len()
    }

    /// Validate one synthetic genome against every loaded population panel.
    pub fn validate_against_all(
        &self,
        synthetic: &SyntheticGenome,
    ) -> HashMap<Population, ValidationResults> {
        self.panels
            .iter()
            .map(|(pop, reference)| (*pop, GenomeComparator::validate(reference, synthetic)))
            .collect()
    }

    /// The population panel the synthetic genome matches most closely.
    pub fn best_match(&self, synthetic: &SyntheticGenome) -> Option<(Population, ValidationResults)> {
        self.validate_against_all(synthetic)
            .into_iter()
            .max_by(|a, b| {
                a.1.overall_similarity
                    .partial_cmp(&b.1.overall_similarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Approximate Fst between two population panels over their shared SNPs
    /// (allele-frequency-only two-population estimator).
    pub fn fst(&self, a: Population, b: Population) -> Option<f32> {
        let ref_a = self.panels.get(&a)?;
        let ref_b = self.panels.get(&b)?;

        let mut sum_fst = 0.0;
        let mut count = 0;

        for (snp_id, (freq_a, _)) in &ref_a.allele_frequencies {
            if let Some((freq_b, _)) = ref_b.allele_frequencies.get(snp_id) {
                sum_fst += snp_fst(*freq_a, *freq_b);
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }

        Some(sum_fst / count as f32)
    }

    /// Fst for every pair of loaded populations.
    pub fn pairwise_fst(&self) -> Vec<(Population, Population, f32)> {
        let pops: Vec<Population> = self.panels.keys().copied().collect();
        let mut out = Vec::new();

        for i in 0..pops.len() {
            for j in (i + 1)..pops.len() {
                if let Some(fst) = self.fst(pops[i], pops[j]) {
                    out.push((pops[i], pops[j], fst));
                }
            }
        }

        out
    }
}

/// Single-locus Fst (Hudson-style estimator) from two population allele frequencies.
fn snp_fst(p_a: f32, p_b: f32) -> f32 {
    let p_bar = (p_a + p_b) / 2.0;
    let h_t = 2.0 * p_bar * (1.0 - p_bar);

    if h_t <= 0.0 {
        return 0.0;
    }

    let h_s = (2.0 * p_a * (1.0 - p_a) + 2.0 * p_b * (1.0 - p_b)) / 2.0;

    ((h_t - h_s) / h_t).max(0.0).min(1.0)
}

/// Recombination-rate profile between adjacent SNPs on one chromosome
/// (cM/Mb), keyed the same way as the LD matrix so it can piggyback on
/// existing reference/synthetic genome plumbing without touching Phase D's
/// core types.
#[derive(Clone, Debug, Default)]
pub struct RecombinationMap {
    pub chromosome: String,
    pub rates: HashMap<(String, String), f32>,
}

impl RecombinationMap {
    pub fn new(chromosome: String) -> Self {
        RecombinationMap {
            chromosome,
            rates: HashMap::new(),
        }
    }

    pub fn add_interval(&mut self, snp_a: String, snp_b: String, rate_cm_per_mb: f32) {
        self.rates.insert((snp_a.clone(), snp_b.clone()), rate_cm_per_mb);
        self.rates.insert((snp_b, snp_a), rate_cm_per_mb);
    }

    pub fn mean_rate(&self) -> f32 {
        if self.rates.is_empty() {
            return 0.0;
        }
        self.rates.values().sum::<f32>() / self.rates.len() as f32
    }
}

/// Result of comparing a reference and synthetic recombination-rate profile.
#[derive(Clone, Debug, Default)]
pub struct RecombinationComparison {
    pub n_intervals_compared: usize,
    pub rmse: f32,
    pub pearson_r: f32,
    pub similarity: f32,
}

/// Compares reference vs. synthetic recombination-rate profiles the same
/// way Phase D compares LD structures: RMSE + Pearson correlation over the
/// shared intervals, folded into a single 0..1 similarity score.
pub struct RecombinationComparator;

impl RecombinationComparator {
    pub fn compare(reference: &RecombinationMap, synthetic: &RecombinationMap) -> RecombinationComparison {
        let mut ref_values = Vec::new();
        let mut syn_values = Vec::new();
        let mut seen = HashSet::new();

        for ((a, b), ref_rate) in &reference.rates {
            let key = if a <= b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
            if !seen.insert(key) {
                continue;
            }
            if let Some(syn_rate) = synthetic.rates.get(&(a.clone(), b.clone())) {
                ref_values.push(*ref_rate);
                syn_values.push(*syn_rate);
            }
        }

        if ref_values.is_empty() {
            return RecombinationComparison::default();
        }

        let n = ref_values.len() as f32;
        let sum_sq: f32 = ref_values
            .iter()
            .zip(&syn_values)
            .map(|(r, s)| (r - s).powi(2))
            .sum();
        let rmse = (sum_sq / n).sqrt();

        let mean_ref = ref_values.iter().sum::<f32>() / n;
        let mean_syn = syn_values.iter().sum::<f32>() / n;

        let mut cov = 0.0;
        let mut var_ref = 0.0;
        let mut var_syn = 0.0;
        for (r, s) in ref_values.iter().zip(&syn_values) {
            let dr = r - mean_ref;
            let ds = s - mean_syn;
            cov += dr * ds;
            var_ref += dr * dr;
            var_syn += ds * ds;
        }

        let pearson_r = if var_ref > 0.0 && var_syn > 0.0 {
            cov / (var_ref.sqrt() * var_syn.sqrt())
        } else {
            0.0
        };

        // Typical human recombination rates span ~0-3 cM/Mb; an RMSE of that
        // scale counts as "no agreement" for the RMSE half of the score.
        let rmse_score = (1.0 - (rmse / 3.0).min(1.0)).max(0.0);
        let corr_score = pearson_r.max(0.0).min(1.0);
        let similarity = (0.5 * rmse_score + 0.5 * corr_score).max(0.0).min(1.0);

        RecombinationComparison {
            n_intervals_compared: ref_values.len(),
            rmse,
            pearson_r,
            similarity,
        }
    }
}

/// Result of comparing haplotype block structure detected independently
/// from the reference and synthetic LD matrices.
#[derive(Clone, Debug, Default)]
pub struct HaplotypeBlockComparison {
    pub reference_blocks: usize,
    pub synthetic_blocks: usize,
    pub reference_mean_block_size: f64,
    pub synthetic_mean_block_size: f64,
    pub reference_mean_r_squared: f64,
    pub synthetic_mean_r_squared: f64,
    pub similarity: f32,
}

/// Detects haplotype blocks (Phase A's BFS-on-LD-graph algorithm) in both
/// the reference and synthetic LD matrices over a shared SNP order, then
/// compares block count, size, and internal LD as a structural-fidelity
/// check beyond the pairwise LD correlation Phase D already computes.
pub struct HaplotypeBlockComparator;

impl HaplotypeBlockComparator {
    fn ld_pairs_for<'a>(
        ld_matrix: impl Iterator<Item = (&'a (String, String), &'a f32)>,
        index_of: &HashMap<&str, u32>,
    ) -> Vec<LdPair> {
        let mut seen = HashSet::new();
        let mut pairs = Vec::new();

        for ((a, b), r2) in ld_matrix {
            let (Some(&ia), Some(&ib)) = (index_of.get(a.as_str()), index_of.get(b.as_str())) else {
                continue;
            };
            let (lo, hi) = if ia < ib { (ia, ib) } else { (ib, ia) };
            if !seen.insert((lo, hi)) {
                continue;
            }
            pairs.push(LdPair {
                snp1_idx: lo,
                snp2_idx: hi,
                r_squared: *r2,
                position1: lo * 1000,
                position2: hi * 1000,
            });
        }

        pairs
    }

    fn block_stats(pairs: &[LdPair], n_snps: usize) -> BlockStatistics {
        if n_snps == 0 {
            return compute_block_statistics(&[]);
        }
        if pairs.is_empty() {
            // No LD edges at all: every SNP is its own singleton block
            // (mirrors what BlockDetector's BFS would produce, since it
            // guards against being called with zero pairs).
            let singletons: Vec<HaplotypeBlock> = (0..n_snps as u32)
                .map(|i| HaplotypeBlock {
                    id: i,
                    snp_indices: vec![i],
                    mean_r_squared: 0.0,
                    start_position: i * 1000,
                    end_position: i * 1000,
                    size: 1,
                })
                .collect();
            return compute_block_statistics(&singletons);
        }
        let blocks = BlockDetector::new(false)
            .detect_blocks(pairs, n_snps)
            .unwrap_or_default();
        compute_block_statistics(&blocks)
    }

    /// Compare block structure between a reference and synthetic genome,
    /// given the SNP id order both LD matrices were built over.
    pub fn compare(
        reference: &ReferenceGenome,
        synthetic: &SyntheticGenome,
        snp_order: &[String],
    ) -> HaplotypeBlockComparison {
        let index_of: HashMap<&str, u32> = snp_order
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i as u32))
            .collect();
        let n_snps = snp_order.len();

        let ref_pairs = Self::ld_pairs_for(reference.ld_matrix.iter(), &index_of);
        let syn_pairs = Self::ld_pairs_for(synthetic.ld_matrix.iter(), &index_of);

        let ref_stats = Self::block_stats(&ref_pairs, n_snps);
        let syn_stats = Self::block_stats(&syn_pairs, n_snps);

        let count_similarity = 1.0
            - (ref_stats.total_blocks as f32 - syn_stats.total_blocks as f32).abs()
                / (ref_stats.total_blocks.max(syn_stats.total_blocks).max(1) as f32);
        let size_similarity = 1.0
            - ((ref_stats.mean_block_size - syn_stats.mean_block_size).abs()
                / ref_stats.mean_block_size.max(syn_stats.mean_block_size).max(1.0)) as f32;
        let r2_similarity = 1.0 - (ref_stats.mean_r_squared - syn_stats.mean_r_squared).abs() as f32;

        let similarity = ((count_similarity + size_similarity + r2_similarity) / 3.0)
            .max(0.0)
            .min(1.0);

        HaplotypeBlockComparison {
            reference_blocks: ref_stats.total_blocks,
            synthetic_blocks: syn_stats.total_blocks,
            reference_mean_block_size: ref_stats.mean_block_size,
            synthetic_mean_block_size: syn_stats.mean_block_size,
            reference_mean_r_squared: ref_stats.mean_r_squared,
            synthetic_mean_r_squared: syn_stats.mean_r_squared,
            similarity,
        }
    }
}

/// Validation record for a single chromosome.
#[derive(Clone, Debug)]
pub struct ChromosomeValidation {
    pub chr: ChromosomeId,
    pub n_loci: usize,
    pub qc: QCMetrics,
    pub validation: ValidationResults,
    pub recombination: RecombinationComparison,
    pub haplotype_blocks: HaplotypeBlockComparison,
}

/// Aggregated validation across an arbitrary set of chromosomes
/// (nominally the full set of 22 human autosomes).
#[derive(Clone, Debug, Default)]
pub struct GenomeWideValidation {
    pub chromosomes: Vec<ChromosomeValidation>,
}

impl GenomeWideValidation {
    pub fn new() -> Self {
        GenomeWideValidation {
            chromosomes: Vec::new(),
        }
    }

    pub fn add_chromosome(&mut self, record: ChromosomeValidation) {
        self.chromosomes.push(record);
    }

    pub fn n_chromosomes(&self) -> usize {
        self.chromosomes.len()
    }

    pub fn total_loci(&self) -> usize {
        self.chromosomes.iter().map(|c| c.n_loci).sum()
    }

    pub fn mean_similarity(&self) -> f32 {
        if self.chromosomes.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.chromosomes.iter().map(|c| c.validation.overall_similarity).sum();
        sum / self.chromosomes.len() as f32
    }

    pub fn mean_quality_score(&self) -> f32 {
        if self.chromosomes.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.chromosomes.iter().map(|c| c.qc.quality_score).sum();
        sum / self.chromosomes.len() as f32
    }

    pub fn mean_recombination_similarity(&self) -> f32 {
        if self.chromosomes.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.chromosomes.iter().map(|c| c.recombination.similarity).sum();
        sum / self.chromosomes.len() as f32
    }

    pub fn mean_haplotype_block_similarity(&self) -> f32 {
        if self.chromosomes.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.chromosomes.iter().map(|c| c.haplotype_blocks.similarity).sum();
        sum / self.chromosomes.len() as f32
    }

    pub fn worst_chromosome(&self) -> Option<&ChromosomeValidation> {
        self.chromosomes.iter().min_by(|a, b| {
            a.validation
                .overall_similarity
                .partial_cmp(&b.validation.overall_similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn best_chromosome(&self) -> Option<&ChromosomeValidation> {
        self.chromosomes.iter().max_by(|a, b| {
            a.validation
                .overall_similarity
                .partial_cmp(&b.validation.overall_similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// True once every autosome 1..=22 has a validation record.
    pub fn is_complete_autosome_set(&self) -> bool {
        let mut seen: Vec<u8> = self.chromosomes.iter().map(|c| c.chr.0).collect();
        seen.sort_unstable();
        seen.dedup();
        seen == (1u8..=22).collect::<Vec<u8>>()
    }

    pub fn summary_string(&self) -> String {
        format!(
            "Genome-Wide Validation: {} chromosomes, {} loci\n\
             Mean similarity: {:.4}\n\
             Mean quality score: {:.4}\n\
             Mean recombination-rate similarity: {:.4}\n\
             Mean haplotype block similarity: {:.4}\n\
             Complete autosome set (1-22): {}",
            self.n_chromosomes(),
            self.total_loci(),
            self.mean_similarity(),
            self.mean_quality_score(),
            self.mean_recombination_similarity(),
            self.mean_haplotype_block_similarity(),
            self.is_complete_autosome_set(),
        )
    }
}

/// Statistical power computed for one locus given a causal effect size.
#[derive(Clone, Debug)]
pub struct LocusPower {
    pub snp_id: String,
    pub maf: f32,
    pub beta: f32,
    pub standardized_effect: f32,
    pub power: f32,
    pub min_n_for_target_power: usize,
}

/// Locus-specific power analysis: unlike Phase D's uniform effect-size power
/// curves, this scales the detectable effect by each locus's own MAF, since
/// variance explained by an additive SNP effect is proportional to
/// 2 * maf * (1 - maf).
pub struct LocusPowerAnalyzer;

impl LocusPowerAnalyzer {
    pub fn standardized_effect_size(beta: f32, maf: f32) -> f32 {
        let maf = maf.max(0.0).min(0.5);
        beta * (2.0 * maf * (1.0 - maf)).max(0.0).sqrt()
    }

    pub fn calculate_locus_power(
        locus: &LocusStats,
        beta: f32,
        n_samples: usize,
        alpha: f32,
        power_target: f32,
    ) -> LocusPower {
        let maf = locus.allele_freq_a.min(locus.allele_freq_b);
        let standardized_effect = Self::standardized_effect_size(beta, maf);
        let power = PowerAnalysis::calculate_power(n_samples, standardized_effect, alpha);
        let min_n_for_target_power =
            PowerAnalysis::min_sample_size(standardized_effect.max(1e-4), power_target, alpha);

        LocusPower {
            snp_id: locus.snp_id.clone(),
            maf,
            beta,
            standardized_effect,
            power,
            min_n_for_target_power,
        }
    }

    /// Power profile for every locus at a shared causal effect size and sample size.
    pub fn power_profile(
        loci: &[LocusStats],
        beta: f32,
        n_samples: usize,
        alpha: f32,
        power_target: f32,
    ) -> Vec<LocusPower> {
        loci.iter()
            .map(|l| Self::calculate_locus_power(l, beta, n_samples, alpha, power_target))
            .collect()
    }

    pub fn mean_power(profile: &[LocusPower]) -> f32 {
        if profile.is_empty() {
            return 0.0;
        }
        profile.iter().map(|p| p.power).sum::<f32>() / profile.len() as f32
    }

    /// Loci whose power falls below the given threshold at the tested sample size.
    pub fn underpowered_loci(profile: &[LocusPower], power_threshold: f32) -> Vec<&LocusPower> {
        profile.iter().filter(|p| p.power < power_threshold).collect()
    }
}

/// Combined Phase E report: genome-wide validation + multi-population
/// comparison + locus-specific power, in one summary.
#[derive(Clone, Debug)]
pub struct ExtendedValidationReport {
    pub n_chromosomes: usize,
    pub total_loci: usize,
    pub mean_similarity: f32,
    pub mean_quality_score: f32,
    pub mean_recombination_similarity: f32,
    pub mean_haplotype_block_similarity: f32,
    pub is_complete_autosome_set: bool,
    pub best_population: Option<Population>,
    pub best_population_similarity: f32,
    pub pairwise_fst: Vec<(Population, Population, f32)>,
    pub mean_locus_power: f32,
    pub underpowered_count: usize,
    pub n_loci_tested_for_power: usize,
}

impl ExtendedValidationReport {
    pub fn build(
        genome_wide: &GenomeWideValidation,
        multi_pop: &MultiPopulationReference,
        representative_synthetic: &SyntheticGenome,
        locus_power_profile: &[LocusPower],
        power_threshold: f32,
    ) -> Self {
        let (best_population, best_population_similarity) =
            match multi_pop.best_match(representative_synthetic) {
                Some((pop, results)) => (Some(pop), results.overall_similarity),
                None => (None, 0.0),
            };

        ExtendedValidationReport {
            n_chromosomes: genome_wide.n_chromosomes(),
            total_loci: genome_wide.total_loci(),
            mean_similarity: genome_wide.mean_similarity(),
            mean_quality_score: genome_wide.mean_quality_score(),
            mean_recombination_similarity: genome_wide.mean_recombination_similarity(),
            mean_haplotype_block_similarity: genome_wide.mean_haplotype_block_similarity(),
            is_complete_autosome_set: genome_wide.is_complete_autosome_set(),
            best_population,
            best_population_similarity,
            pairwise_fst: multi_pop.pairwise_fst(),
            mean_locus_power: LocusPowerAnalyzer::mean_power(locus_power_profile),
            underpowered_count: LocusPowerAnalyzer::underpowered_loci(locus_power_profile, power_threshold).len(),
            n_loci_tested_for_power: locus_power_profile.len(),
        }
    }

    pub fn summary_string(&self) -> String {
        format!(
            "Phase E Extended Validation Report\n\
             Chromosomes validated: {} (complete 1-22 set: {})\n\
             Total loci: {}\n\
             Mean genome-wide similarity: {:.4}\n\
             Mean genome-wide quality score: {:.4}\n\
             Mean recombination-rate similarity: {:.4}\n\
             Mean haplotype block similarity: {:.4}\n\
             Best-matching population: {} (similarity {:.4})\n\
             Population pairs compared (Fst): {}\n\
             Locus power: mean {:.4} across {} loci, {} underpowered",
            self.n_chromosomes,
            self.is_complete_autosome_set,
            self.total_loci,
            self.mean_similarity,
            self.mean_quality_score,
            self.mean_recombination_similarity,
            self.mean_haplotype_block_similarity,
            self.best_population.map(|p| p.label()).unwrap_or("none"),
            self.best_population_similarity,
            self.pairwise_fst.len(),
            self.mean_locus_power,
            self.n_loci_tested_for_power,
            self.underpowered_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::quality_control::GenomeValidator;

    fn sample_reference(pop_freq_shift: f32) -> ReferenceGenome {
        let mut r = ReferenceGenome::new("test".to_string(), 500);
        r.add_snp("rs1".to_string(), 0.5 + pop_freq_shift, 0.5 - pop_freq_shift);
        r.add_snp("rs2".to_string(), 0.3 + pop_freq_shift, 0.7 - pop_freq_shift);
        r.add_snp("rs3".to_string(), 0.6 + pop_freq_shift, 0.4 - pop_freq_shift);
        r.add_ld_pair("rs1".to_string(), "rs2".to_string(), 0.4);
        r.add_ld_pair("rs2".to_string(), "rs3".to_string(), 0.2);
        r.finalize();
        r
    }

    fn sample_synthetic() -> SyntheticGenome {
        let mut s = SyntheticGenome::new(100);
        s.add_snp("rs1".to_string(), 0.5, 0.5);
        s.add_snp("rs2".to_string(), 0.3, 0.7);
        s.add_snp("rs3".to_string(), 0.6, 0.4);
        s.add_ld_pair("rs1".to_string(), "rs2".to_string(), 0.4);
        s.add_ld_pair("rs2".to_string(), "rs3".to_string(), 0.2);
        s.finalize();
        s
    }

    #[test]
    fn test_multi_population_best_match() {
        let mut multi = MultiPopulationReference::new();
        multi.add_population(Population::Eur, sample_reference(0.0));
        multi.add_population(Population::Afr, sample_reference(0.25));

        let synthetic = sample_synthetic();
        let (best, results) = multi.best_match(&synthetic).unwrap();

        assert_eq!(best, Population::Eur);
        assert!(results.overall_similarity > 0.9);
    }

    #[test]
    fn test_pairwise_fst_identical_populations_is_zero() {
        let mut multi = MultiPopulationReference::new();
        multi.add_population(Population::Eur, sample_reference(0.0));
        multi.add_population(Population::Afr, sample_reference(0.0));

        let fst = multi.fst(Population::Eur, Population::Afr).unwrap();
        assert!(fst < 1e-6);
    }

    #[test]
    fn test_pairwise_fst_diverged_populations_is_positive() {
        let mut multi = MultiPopulationReference::new();
        multi.add_population(Population::Eur, sample_reference(0.0));
        multi.add_population(Population::Afr, sample_reference(0.3));

        let fst = multi.fst(Population::Eur, Population::Afr).unwrap();
        assert!(fst > 0.0);
    }

    fn sample_snp_order() -> Vec<String> {
        vec!["rs1".to_string(), "rs2".to_string(), "rs3".to_string()]
    }

    fn sample_recombination_map(rate_shift: f32) -> RecombinationMap {
        let mut m = RecombinationMap::new("test".to_string());
        m.add_interval("rs1".to_string(), "rs2".to_string(), 1.0 + rate_shift);
        m.add_interval("rs2".to_string(), "rs3".to_string(), 0.5 + rate_shift);
        m
    }

    #[test]
    fn test_genome_wide_validation_aggregation() {
        let mut gw = GenomeWideValidation::new();
        let snp_order = sample_snp_order();

        for chr in 1u8..=22 {
            let locus = GenomeValidator::validate_locus("rs1".to_string(), (85, 40, 10));
            let qc = GenomeValidator::generate_report(&[locus], 0.4);
            let reference = sample_reference(0.0);
            let synthetic = sample_synthetic();
            let validation = GenomeComparator::validate(&reference, &synthetic);
            let recombination = RecombinationComparator::compare(
                &sample_recombination_map(0.0),
                &sample_recombination_map(0.0),
            );
            let haplotype_blocks =
                HaplotypeBlockComparator::compare(&reference, &synthetic, &snp_order);

            gw.add_chromosome(ChromosomeValidation {
                chr: ChromosomeId(chr),
                n_loci: 1,
                qc,
                validation,
                recombination,
                haplotype_blocks,
            });
        }

        assert_eq!(gw.n_chromosomes(), 22);
        assert!(gw.is_complete_autosome_set());
        assert_eq!(gw.total_loci(), 22);
        assert!(gw.mean_similarity() > 0.9);
        assert!(gw.mean_recombination_similarity() > 0.9);
        assert!(gw.mean_haplotype_block_similarity() > 0.9);
    }

    #[test]
    fn test_recombination_comparator_identical_maps_score_high() {
        let a = sample_recombination_map(0.0);
        let b = sample_recombination_map(0.0);

        let result = RecombinationComparator::compare(&a, &b);

        assert_eq!(result.n_intervals_compared, 2);
        assert!(result.rmse < 1e-6);
        assert!(result.similarity > 0.95);
    }

    #[test]
    fn test_recombination_comparator_diverged_maps_score_lower() {
        let a = sample_recombination_map(0.0);
        let b = sample_recombination_map(2.5);

        let result = RecombinationComparator::compare(&a, &b);

        assert!(result.rmse > 1.0);
        assert!(result.similarity < 0.6);
    }

    #[test]
    fn test_haplotype_block_comparator_identical_ld_scores_high() {
        let reference = sample_reference(0.0);
        let synthetic = sample_synthetic();

        let result = HaplotypeBlockComparator::compare(&reference, &synthetic, &sample_snp_order());

        assert!(result.reference_blocks >= 1);
        assert_eq!(result.reference_blocks, result.synthetic_blocks);
        assert!(result.similarity > 0.9);
    }

    #[test]
    fn test_haplotype_block_comparator_no_ld_yields_one_block_per_snp() {
        let reference = ReferenceGenome::new("test".to_string(), 100);
        let synthetic = SyntheticGenome::new(100);

        let result = HaplotypeBlockComparator::compare(&reference, &synthetic, &sample_snp_order());

        assert_eq!(result.reference_blocks, 3);
        assert_eq!(result.synthetic_blocks, 3);
        assert!(result.similarity > 0.9);
    }

    #[test]
    fn test_locus_power_scales_with_maf() {
        let common = GenomeValidator::validate_locus("common".to_string(), (250, 500, 250));
        let rare = GenomeValidator::validate_locus("rare".to_string(), (2, 30, 468));

        let common_power = LocusPowerAnalyzer::calculate_locus_power(&common, 0.2, 500, 0.05, 0.8);
        let rare_power = LocusPowerAnalyzer::calculate_locus_power(&rare, 0.2, 500, 0.05, 0.8);

        assert!(common_power.standardized_effect > rare_power.standardized_effect);
        assert!(common_power.power >= rare_power.power);
    }

    #[test]
    fn test_underpowered_loci_filter() {
        let low_maf = GenomeValidator::validate_locus("rare".to_string(), (1, 10, 489));
        let profile = LocusPowerAnalyzer::power_profile(&[low_maf], 0.05, 50, 0.05, 0.8);

        let underpowered = LocusPowerAnalyzer::underpowered_loci(&profile, 0.8);
        assert_eq!(underpowered.len(), 1);
    }
}
