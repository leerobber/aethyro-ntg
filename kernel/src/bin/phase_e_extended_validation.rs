/// Phase E: Extended Validation
/// Genome-wide validation across the real chromosomes available in
/// data/raw (chr1, chr2, chr3, chr22 -- not the full 22-autosome set),
/// multi-population reference comparison, and locus-specific statistical
/// power computed from real per-locus allele frequencies.
/// Pure Rust implementation
///
/// Usage: cargo run --release --bin phase_e_extended_validation [-- <max_variants>]
///
/// Real-data honesty notes:
/// - Genome-wide validation and locus power are built from real VCF data
///   via build_real_chromosome (VCF -> LD -> brain -> Phase C synthesis
///   via haplotype-block resampling, preserving real within-block LD),
///   the same pipeline Phase D uses.
/// - Only 4 of the 22 human autosomes have VCFs in data/raw, so
///   `is_complete_autosome_set` will correctly report false.
/// - Multi-population (EUR/AFR/ASN) comparison remains a labeled SYNTHETIC
///   PROXY: data/raw has no per-sample population panel file, only pooled
///   "ALL" 1000G genotypes, so there is no real ancestry label to split
///   samples by. Faking a population split would be worse than not having
///   one -- this is flagged in every place the proxy numbers are printed.
/// - Recombination-rate matching also remains a synthetic proxy: no real
///   genetic map file is present in data/raw either.

use ntg_kernel::genomic::{
    build_real_chromosome, snp_key, ChromosomeValidation, ExtendedValidationReport,
    GenomeComparator, GenomeValidator, GenomeWideValidation, HaplotypeBlockComparator,
    LocusPowerAnalyzer, MultiPopulationReference, Population, RecombinationComparator,
    RecombinationMap, ReferenceGenome, SyntheticGenome,
};

const REAL_CHROMOSOMES: [u8; 4] = [1, 2, 3, 22];
const PROXY_SNPS_PER_CHROMOSOME: usize = 10;

fn vcf_path(chr: u8) -> String {
    format!(
        "{}/../data/raw/1000g/ALL.chr{}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
        env!("CARGO_MANIFEST_DIR"),
        chr
    )
}

// ---- SYNTHETIC PROXY helpers (multi-population + recombination only) ----
// No real population panel or genetic map file exists in data/raw; see
// the module doc above. Everything else in this binary uses real data.

fn proxy_freq(chr: u8, snp_idx: usize, drift: f32) -> f32 {
    let base = ((chr as f32 * 7.0 + snp_idx as f32 * 3.0).sin() + 1.0) / 2.0;
    (0.05 + base * 0.9 + drift).max(0.02).min(0.98)
}

fn proxy_reference(chr: u8, n_samples: usize, population_drift: f32) -> ReferenceGenome {
    let mut reference = ReferenceGenome::new(format!("proxy-chr{}", chr), n_samples);
    for i in 0..PROXY_SNPS_PER_CHROMOSOME {
        let freq_a = proxy_freq(chr, i, population_drift);
        reference.add_snp(format!("chr{}_rs{}", chr, i), freq_a, 1.0 - freq_a);
    }
    for i in 0..PROXY_SNPS_PER_CHROMOSOME - 1 {
        let r2 = (0.1 + 0.4 * (i as f32 / PROXY_SNPS_PER_CHROMOSOME as f32)).min(0.9);
        reference.add_ld_pair(format!("chr{}_rs{}", chr, i), format!("chr{}_rs{}", chr, i + 1), r2);
    }
    reference.finalize();
    reference
}

fn proxy_synthetic(chr: u8, n_samples: usize) -> SyntheticGenome {
    let mut synthetic = SyntheticGenome::new(n_samples);
    for i in 0..PROXY_SNPS_PER_CHROMOSOME {
        let freq_a = proxy_freq(chr, i, 0.01);
        synthetic.add_snp(format!("chr{}_rs{}", chr, i), freq_a, 1.0 - freq_a);
    }
    for i in 0..PROXY_SNPS_PER_CHROMOSOME - 1 {
        let r2 = (0.1 + 0.4 * (i as f32 / PROXY_SNPS_PER_CHROMOSOME as f32)).min(0.9);
        synthetic.add_ld_pair(
            format!("chr{}_rs{}", chr, i),
            format!("chr{}_rs{}", chr, i + 1),
            (r2 + 0.02).min(1.0),
        );
    }
    synthetic.finalize();
    synthetic
}

fn proxy_recomb_rate(chr: u8, interval_idx: usize, drift: f32) -> f32 {
    let base = ((chr as f32 * 5.0 + interval_idx as f32 * 11.0).cos() + 1.0) / 2.0;
    (0.2 + base * 2.0 + drift).max(0.05)
}

fn proxy_recombination_map(chr: u8, drift: f32, snp_order: &[String]) -> RecombinationMap {
    let mut map = RecombinationMap::new(format!("chr{}", chr));
    for i in 0..snp_order.len().saturating_sub(1) {
        let rate = proxy_recomb_rate(chr, i, drift);
        map.add_interval(snp_order[i].clone(), snp_order[i + 1].clone(), rate);
    }
    map
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase E: Extended Validation                                ║");
    println!("║  Real Chromosomes (1,2,3,22) | Multi-Pop* | Locus Power      ║");
    println!("║  Pure Rust | No Dependencies | *Multi-pop is a labeled proxy ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let max_variants: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    // 2000, not 200: with the LD-computation fix (see ld_compute.rs), LD
    // correlation is real but noisy for rare variants at small synthetic
    // sample sizes -- confirmed empirically in Phase D that n=2000
    // resolves the noise (r² correlation 0.05 -> 0.59 at the same cap).
    let synthetic_n_samples = 2000;

    // ========== STEP 1: Multi-Population Reference Panels (SYNTHETIC PROXY) ==========
    println!("\n[Step 1/4] Building Multi-Population Reference Panels (EUR/AFR/ASN)...");
    println!("  ⚠ PROXY DATA: data/raw has no population-panel file (no per-sample");
    println!("    ancestry labels), only pooled \"ALL\" 1000G genotypes. This step");
    println!("    uses deterministic synthetic panels to exercise the Fst/best-match");
    println!("    machinery, NOT real EUR/AFR/ASN allele frequencies.");

    let mut multi_pop = MultiPopulationReference::new();
    multi_pop.add_population(Population::Eur, proxy_reference(1, 503, 0.0));
    multi_pop.add_population(Population::Afr, proxy_reference(1, 661, 0.18));
    multi_pop.add_population(Population::Asn, proxy_reference(1, 504, 0.10));

    println!("✓ Populations loaded: {}", multi_pop.n_populations());
    for (pop_a, pop_b, fst) in multi_pop.pairwise_fst() {
        println!("  Fst({}, {}) = {:.4}", pop_a.label(), pop_b.label(), fst);
    }

    let representative_synthetic = proxy_synthetic(1, synthetic_n_samples);
    let (best_pop, best_results) = multi_pop
        .best_match(&representative_synthetic)
        .expect("at least one population panel loaded");
    println!(
        "✓ Best-matching proxy population: {} (similarity {:.4})",
        best_pop.label(),
        best_results.overall_similarity
    );

    // ========== STEP 2: Genome-Wide Validation across real chromosomes ==========
    println!(
        "\n[Step 2/4] Validating Phase C's Synthetic Genomes Against Real Chromosomes {:?}...",
        REAL_CHROMOSOMES
    );
    println!("  (data/raw only has VCFs for these 4 of the 22 human autosomes)");

    let mut genome_wide = GenomeWideValidation::new();
    let mut chr1_real_loci = Vec::new();
    let mut chr1_real_n_samples = 0usize;

    for &chr in &REAL_CHROMOSOMES {
        println!("\n  Building chr{} from real VCF data...", chr);
        let data = build_real_chromosome(&vcf_path(chr), chr, max_variants, synthetic_n_samples, 42, true)
            .unwrap_or_else(|e| panic!("failed to build real chr{}: {}", chr, e));

        println!(
            "  ✓ chr{}: {} real SNPs, {} real samples, {} LD pairs, {} synthetic samples",
            chr,
            data.snp_order.len(),
            data.n_real_samples,
            data.ld_pairs.len(),
            synthetic_n_samples
        );

        // QC from the sampled genome's own real genotype counts.
        let n_qc_loci = data.sampled_genome.genotypes.len().min(200);
        let mut loci = Vec::new();
        for idx in 0..n_qc_loci {
            let mut counts = (0usize, 0usize, 0usize);
            for &g in &data.sampled_genome.genotypes[idx] {
                match g {
                    0 => counts.0 += 1,
                    1 => counts.1 += 1,
                    2 => counts.2 += 1,
                    _ => {}
                }
            }
            loci.push(GenomeValidator::validate_locus(snp_key(idx as u32), counts));
        }

        if chr == 1 {
            chr1_real_loci = loci.clone();
            chr1_real_n_samples = data.n_real_samples;
        }

        let synthetic_mean_ld_r2: f32 = if data.ld_pairs.is_empty() {
            0.0
        } else {
            let sum: f32 = data
                .ld_pairs
                .iter()
                .map(|p| data.sampled_genome.ld_r2(p.snp1_idx as usize, p.snp2_idx as usize))
                .sum();
            sum / data.ld_pairs.len() as f32
        };
        let qc = GenomeValidator::generate_report(&loci, synthetic_mean_ld_r2);

        let validation = GenomeComparator::validate(&data.reference, &data.synthetic);

        // Recombination stays a labeled proxy (no real genetic map available).
        let reference_recomb = proxy_recombination_map(chr, 0.0, &data.snp_order);
        let synthetic_recomb = proxy_recombination_map(chr, 0.05, &data.snp_order);
        let recombination = RecombinationComparator::compare(&reference_recomb, &synthetic_recomb);

        let haplotype_blocks =
            HaplotypeBlockComparator::compare(&data.reference, &data.synthetic, &data.snp_order);

        genome_wide.add_chromosome(ChromosomeValidation {
            chr: data.chr,
            n_loci: data.snp_order.len(),
            qc,
            validation,
            recombination,
            haplotype_blocks,
        });
    }

    println!("\n{}", genome_wide.summary_string());
    if let Some(worst) = genome_wide.worst_chromosome() {
        println!("  Lowest similarity: chr{} ({:.4})", worst.chr.0, worst.validation.overall_similarity);
    }
    if let Some(best) = genome_wide.best_chromosome() {
        println!("  Highest similarity: chr{} ({:.4})", best.chr.0, best.validation.overall_similarity);
    }

    // ========== STEP 3: Locus-Specific Power (real chr1 allele frequencies) ==========
    println!("\n[Step 3/4] Computing Locus-Specific Statistical Power (real chr1 loci)...");

    let beta = 0.15;
    let alpha = 0.05;
    let power_target = 0.8;
    let power_profile =
        LocusPowerAnalyzer::power_profile(&chr1_real_loci, beta, chr1_real_n_samples, alpha, power_target);

    println!(
        "✓ Locus power (β={:.2}, n={} real chr1 samples, α={:.2}):",
        beta, chr1_real_n_samples, alpha
    );
    println!("SNP        | MAF    | Std. Effect | Power  | Min n (80% power)");
    println!("-----------|--------|-------------|--------|------------------");
    for locus in power_profile.iter().take(10) {
        println!(
            "{:11}| {:6.3} | {:11.4} | {:5.1}% | {:16}",
            locus.snp_id, locus.maf, locus.standardized_effect, locus.power * 100.0, locus.min_n_for_target_power
        );
    }
    if power_profile.len() > 10 {
        println!("... and {} more loci", power_profile.len() - 10);
    }

    let underpowered = LocusPowerAnalyzer::underpowered_loci(&power_profile, power_target);
    println!(
        "✓ Mean locus power: {:.4} | Underpowered loci (<{:.0}%): {} of {}",
        LocusPowerAnalyzer::mean_power(&power_profile),
        power_target * 100.0,
        underpowered.len(),
        power_profile.len()
    );

    // ========== STEP 4: Extended Validation Report ==========
    println!("\n[Step 4/4] Generating Phase E Report...\n");

    let report: ExtendedValidationReport = ExtendedValidationReport::build(
        &genome_wide,
        &multi_pop,
        &representative_synthetic,
        &power_profile,
        power_target,
    );

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  PHASE E EXTENDED VALIDATION COMPLETE                        ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\n{}", report.summary_string());
    println!(
        "(chromosome set: real chr{{1,2,3,22}} only -- {} of 22 autosomes; multi-population is a labeled proxy)",
        REAL_CHROMOSOMES.len()
    );

    println!("\n📋 Recommendation:");
    if report.is_complete_autosome_set && report.mean_similarity > 0.90 {
        println!("  PASS: All 22 autosomes validated, mean similarity {:.2}%", report.mean_similarity * 100.0);
    } else if report.mean_similarity > 0.80 {
        println!(
            "  PASS: Genome-wide similarity acceptable ({:.2}%), but chromosome set incomplete ({} of 22 real)",
            report.mean_similarity * 100.0,
            REAL_CHROMOSOMES.len()
        );
    } else {
        println!(
            "  REVIEW: Genome-wide similarity below threshold ({:.2}%) on real chr{{1,2,3,22}}",
            report.mean_similarity * 100.0
        );
        println!(
            "    (GenomeSampler now resamples real haplotype fragments per LD block; if LD\n\
             \x20   correlation is still weak, check whether synthetic_n_samples is large enough\n\
             \x20   -- rare-variant pairwise LD needs more synthetic draws to estimate reliably,\n\
             \x20   not more haplotype pools. Phase D confirmed n=2000 resolves this at chr1 scale.)"
        );
    }
}
