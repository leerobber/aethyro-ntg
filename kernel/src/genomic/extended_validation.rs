/// Phase E: Extended Validation
/// Genome-wide validation across all 22 chromosomes, multi-population
/// reference comparison (EUR/AFR/ASN), and locus-specific statistical power.
/// Builds on Phase D (quality_control, validation). Pure Rust, no dependencies.

use std::collections::HashMap;

use crate::genomic::chromosome_brain::ChromosomeId;
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

/// Validation record for a single chromosome.
#[derive(Clone, Debug)]
pub struct ChromosomeValidation {
    pub chr: ChromosomeId,
    pub n_loci: usize,
    pub qc: QCMetrics,
    pub validation: ValidationResults,
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
             Complete autosome set (1-22): {}",
            self.n_chromosomes(),
            self.total_loci(),
            self.mean_similarity(),
            self.mean_quality_score(),
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
             Best-matching population: {} (similarity {:.4})\n\
             Population pairs compared (Fst): {}\n\
             Locus power: mean {:.4} across {} loci, {} underpowered",
            self.n_chromosomes,
            self.is_complete_autosome_set,
            self.total_loci,
            self.mean_similarity,
            self.mean_quality_score,
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

    #[test]
    fn test_genome_wide_validation_aggregation() {
        let mut gw = GenomeWideValidation::new();

        for chr in 1u8..=22 {
            let locus = GenomeValidator::validate_locus("rs1".to_string(), (85, 40, 10));
            let qc = GenomeValidator::generate_report(&[locus], 0.4);
            let reference = sample_reference(0.0);
            let synthetic = sample_synthetic();
            let validation = GenomeComparator::validate(&reference, &synthetic);

            gw.add_chromosome(ChromosomeValidation {
                chr: ChromosomeId(chr),
                n_loci: 1,
                qc,
                validation,
            });
        }

        assert_eq!(gw.n_chromosomes(), 22);
        assert!(gw.is_complete_autosome_set());
        assert_eq!(gw.total_loci(), 22);
        assert!(gw.mean_similarity() > 0.9);
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
