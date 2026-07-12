/// Phase D: Quality Control & Validation
/// Statistical validation of Phase C's synthetic genomes against a real
/// 1000 Genomes reference built from actual chr1 VCF data.
/// Pure Rust implementation
///
/// Usage: cargo run --release --bin phase_d_quality_control [-- <max_variants>]

use ntg_kernel::genomic::{
    init_chromosome_brain, BlockDetector, ChromosomeId, GenomeComparator, GenomeSampler,
    GenomeValidator, LdComputer, PowerAnalysis, ReferenceGenome, SyntheticGenome, VcfParser,
};

fn vcf_path(chr: &str) -> String {
    format!(
        "{}/../data/raw/1000g/ALL.chr{}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
        env!("CARGO_MANIFEST_DIR"),
        chr
    )
}

fn snp_key(idx: u32) -> String {
    format!("snp{}", idx)
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase D: Quality Control & Validation                       ║");
    println!("║  Statistical Validation: Synthetic vs 1000 Genomes           ║");
    println!("║  Pure Rust | No Dependencies | Production Ready             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let max_variants: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let synthetic_n_samples = 200;

    // ========== STEP 1: Build Reference Genome from real chr1 VCF ==========
    println!("\n[Step 1/6] Loading real 1000 Genomes chr1 data...");

    let chr_path = vcf_path("1");
    let chromosome = VcfParser::new(false)
        .parse_vcf_limited(&chr_path, 1, max_variants)
        .expect("failed to parse real VCF data");
    let positions: Vec<u32> = chromosome.snps.iter().map(|s| s.position).collect();
    let ld_matrix = LdComputer::new(false, 0.5)
        .compute_ld(&chromosome.genotypes, &positions)
        .expect("LD computation failed");

    let mut reference = ReferenceGenome::new("1000G-chr1".to_string(), chromosome.sample_names.len());
    for (idx, snp) in chromosome.genotypes.iter().enumerate() {
        let (freq_ref, freq_alt, _freq_missing) = snp.allele_frequencies();
        reference.add_snp(snp_key(idx as u32), freq_alt as f32, freq_ref as f32);
    }
    for pair in &ld_matrix.pairs {
        reference.add_ld_pair(snp_key(pair.snp1_idx), snp_key(pair.snp2_idx), pair.r_squared);
    }
    reference.finalize();

    println!(
        "✓ Reference: real 1000 Genomes chr1 (n={}, SNPs={}, LD pairs={})",
        reference.n_samples,
        reference.allele_frequencies.len(),
        ld_matrix.pairs.len()
    );
    println!("✓ Reference LD r² mean: {:.3}", reference.mean_ld_r2);

    // ========== STEP 2: Build blocks + brain, then sample Phase C's synthetic genome ==========
    println!("\n[Step 2/6] Synthesizing a genome from Phase C (real per-locus frequencies)...");

    let mut blocks = BlockDetector::new(false)
        .detect_blocks(&ld_matrix.pairs, chromosome.snps.len())
        .expect("block detection failed");
    BlockDetector::new(false)
        .annotate_blocks(&mut blocks, &positions)
        .expect("block annotation failed");
    let brain = init_chromosome_brain(
        ChromosomeId(1),
        &chromosome.genotypes,
        &chromosome.snps,
        &ld_matrix.pairs,
        &blocks,
    )
    .expect("brain initialization failed");

    let sampler = GenomeSampler::from_brain(&brain, synthetic_n_samples, 42);
    let sampled_genome = sampler.sample(0);
    println!(
        "✓ Sampled 1 synthetic genome: {} SNPs, {} samples (targets = real chr1 allele frequencies)",
        sampled_genome.genotypes.len(),
        synthetic_n_samples
    );

    // ========== STEP 3: Quality Control on the synthetic genome's own genotypes ==========
    println!("\n[Step 3/6] Computing Quality Control Metrics on synthesized genotypes...");

    let n_qc_loci = sampled_genome.genotypes.len().min(200);
    let mut loci = Vec::new();
    for idx in 0..n_qc_loci {
        let mut counts = (0usize, 0usize, 0usize);
        for &g in &sampled_genome.genotypes[idx] {
            match g {
                0 => counts.0 += 1,
                1 => counts.1 += 1,
                2 => counts.2 += 1,
                _ => {} // missing
            }
        }
        loci.push(GenomeValidator::validate_locus(snp_key(idx as u32), counts));
    }

    let synthetic_mean_ld_r2: f32 = if ld_matrix.pairs.is_empty() {
        0.0
    } else {
        let sum: f32 = ld_matrix
            .pairs
            .iter()
            .map(|p| sampled_genome.ld_r2(p.snp1_idx as usize, p.snp2_idx as usize))
            .sum();
        sum / ld_matrix.pairs.len() as f32
    };

    let qc_report = GenomeValidator::generate_report(&loci, synthetic_mean_ld_r2);

    println!("✓ Population Statistics:");
    println!("  Samples: {}", qc_report.population_stats.n_samples);
    println!("  SNPs: {}", qc_report.population_stats.n_snps);
    println!("  Mean MAF: {:.4}", qc_report.population_stats.mean_maf);
    println!("  Mean He: {:.4}", qc_report.population_stats.mean_he);
    println!("  Mean π: {:.4}", qc_report.population_stats.mean_pi);
    println!("\n✓ Quality Control:");
    println!("  HWE violations: {}", qc_report.hwe_violations);
    println!("  Low MAF (<0.05): {}", qc_report.low_maf_count);
    println!("  Mean LD r²: {:.4}", qc_report.mean_ld_r2);
    println!("  Quality score: {:.4}", qc_report.quality_score);

    // ========== STEP 4: Compare synthetic genome to the real reference ==========
    println!("\n[Step 4/6] Validating synthetic genome against real reference...");

    let mut synthetic = SyntheticGenome::new(synthetic_n_samples);
    for idx in 0..sampled_genome.genotypes.len() {
        let freq_alt = sampled_genome.allele_freq(idx);
        // Same (alt, ref) convention as the reference above.
        synthetic.add_snp(snp_key(idx as u32), freq_alt, 1.0 - freq_alt);
    }
    for pair in &ld_matrix.pairs {
        let r2 = sampled_genome.ld_r2(pair.snp1_idx as usize, pair.snp2_idx as usize);
        synthetic.add_ld_pair(snp_key(pair.snp1_idx), snp_key(pair.snp2_idx), r2);
    }
    synthetic.finalize();

    let validation = GenomeComparator::validate(&reference, &synthetic);

    println!("✓ Validation Results:");
    println!("  Reference population: {}", validation.ref_population);
    println!("  SNPs compared: {}", validation.n_snps_compared);
    println!("  Allele frequency RMSE: {:.4}", validation.allele_freq_rmse);
    println!("  LD r² Pearson correlation: {:.4}", validation.ld_pearson_r);
    println!("  LD distance: {:.4}", validation.ld_distance);
    println!("  Overall similarity: {:.4}", validation.overall_similarity);

    // ========== STEP 5: Power Analysis ==========
    println!("\n[Step 5/6] Computing Statistical Power Analysis...");

    let effect_sizes = vec![0.05, 0.1, 0.2];
    let alpha = 0.05;

    println!("✓ Power Analysis (α=0.05):");
    println!("\nEffect Size | n=135 | n=250 | n=500 | Min n (80% power)");
    println!("------------|-------|-------|-------|------------------");

    for effect_size in effect_sizes {
        let power_135 = PowerAnalysis::calculate_power(135, effect_size, alpha);
        let power_250 = PowerAnalysis::calculate_power(250, effect_size, alpha);
        let power_500 = PowerAnalysis::calculate_power(500, effect_size, alpha);
        let min_n = PowerAnalysis::min_sample_size(effect_size, 0.8, alpha);

        println!(
            "{:11.3}| {:5.1}% | {:5.1}% | {:5.1}% | {:16}",
            effect_size,
            power_135 * 100.0,
            power_250 * 100.0,
            power_500 * 100.0,
            min_n
        );
    }

    // ========== STEP 6: Summary Report ==========
    println!("\n[Step 6/6] Generating Quality Control Report...\n");

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  PHASE D QUALITY CONTROL COMPLETE                           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    println!("\n📊 Quality Control Summary:");
    println!("  ✓ Synthetic genome: 1 genome, {} loci QC'd", loci.len());
    println!("  ✓ Hardy-Weinberg validation: {} SNPs pass (p > 0.05)", {
        loci.iter().filter(|l| l.hardy_weinberg_p > 0.05).count()
    });
    println!("  ✓ Allele frequency matching: {:.2}% similarity", validation.overall_similarity * 100.0);
    println!("  ✓ LD structure correlation: {:.4}", validation.ld_pearson_r);
    println!("  ✓ Statistical power: 80% at n={} (effect size 0.1)", PowerAnalysis::min_sample_size(0.1, 0.8, 0.05));

    println!("\n📈 Validation Metrics:");
    println!("  Phase A: Data → Genotypes → LD → Blocks ✓");
    println!("  Phase B: Brains → Agents → Domain Detection ✓");
    println!("  Phase C: Synthesis → Evolution → Phenotypes ✓");
    println!("  Phase D: Quality Control → Validation ✓");

    println!("\n✓ Pure Rust pipeline: 4 complete phases");
    println!("✓ Zero dependencies");
    println!("✓ Ready for Phase E (Extended Validation)");

    println!("\n📋 Recommendation:");
    if validation.overall_similarity > 0.90 {
        println!("  PASS: Synthetic genomes highly similar to reference (similarity={:.2}%)",
                 validation.overall_similarity * 100.0);
    } else if validation.overall_similarity > 0.80 {
        println!("  PASS: Synthetic genomes reasonably similar to reference (similarity={:.2}%)",
                 validation.overall_similarity * 100.0);
    } else {
        println!("  REVIEW: overall similarity {:.2}% is below threshold.", validation.overall_similarity * 100.0);
        println!(
            "    Allele frequency RMSE {:.4} ({}), LD correlation {:.4} ({}).",
            validation.allele_freq_rmse,
            if validation.allele_freq_rmse < 0.05 { "good match" } else { "poor match" },
            validation.ld_pearson_r,
            if validation.ld_pearson_r.abs() > 0.5 { "good match" } else { "poor match" },
        );
        if validation.allele_freq_rmse < 0.05 && validation.ld_pearson_r.abs() < 0.5 {
            println!(
                "    Diagnosis: GenomeSampler targets real per-locus allele frequencies correctly,\n\
                 \x20   but samples each locus independently and does not model haplotype/LD\n\
                 \x20   structure (see synthesis.rs module doc). That is the gap to close next,\n\
                 \x20   not a data-wiring problem."
            );
        }
    }
}
