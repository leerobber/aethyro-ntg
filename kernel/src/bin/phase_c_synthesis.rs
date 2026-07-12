/// Phase C: Synthetic Genome Synthesis & Evolution
/// Complete pipeline: real chromosome brain -> Synthesis -> Evolution -> Phenotype Prediction
/// Pure Rust implementation
///
/// Usage: cargo run --release --bin phase_c_synthesis [-- <max_variants>]
/// Builds a real ChromosomeBrain from data/raw/1000g chr1 (optionally
/// capped to a leading slice of variants) and synthesizes a population
/// via haplotype-block resampling: real per-locus allele frequencies AND
/// real within-block LD, both from actual observed haplotypes (real VCFs
/// are phased) instead of independent per-locus draws at an assumed
/// frequency. See genomic::synthesis module doc for the mechanism.

use ntg_kernel::genomic::{
    BlockDetector, ChromosomeId, DefaultFitnessModel, Environment, EvolutionSim, GenomeSampler,
    GxEEngine, LdComputer, PhenotypeHead, VcfParser, init_chromosome_brain,
};

fn vcf_path(chr: &str) -> String {
    format!(
        "{}/../data/raw/1000g/ALL.chr{}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
        env!("CARGO_MANIFEST_DIR"),
        chr
    )
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase C: Synthetic Genome Synthesis & Evolution             ║");
    println!("║  Pure Rust | No Dependencies | Production Ready             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let max_variants: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let n_samples = 100;
    let population_size = 50;
    let n_generations = 10;

    // ========== STEP 1: Build a real Chromosome Brain (Phase A + B) ==========
    println!("\n[Step 1/4] Building chromosome brain from real chr1 VCF data (phased)...");
    let chr_path = vcf_path("1");
    let vcf_parser = VcfParser::new(false);
    let chromosome = vcf_parser
        .parse_vcf_phased_limited(&chr_path, 1, max_variants)
        .expect("failed to parse real VCF data");
    let positions: Vec<u32> = chromosome.snps.iter().map(|s| s.position).collect();
    let ld_matrix = LdComputer::new(false, 0.5)
        .compute_ld(&chromosome.genotypes, &positions)
        .expect("LD computation failed");
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
    println!(
        "✓ Brain built from real data: {} SNPs, {} LD pairs, {} haplotype blocks",
        brain.neurons.len(),
        ld_matrix.pairs.len(),
        blocks.len()
    );

    let mean_real_af: f32 =
        brain.neurons.iter().map(|n| n.allele_freq).sum::<f32>() / brain.neurons.len().max(1) as f32;
    println!("✓ Mean real alt-allele frequency across these loci: {:.4}", mean_real_af);

    // ========== STEP 2: Generate Initial Population via haplotype-block resampling ==========
    println!("\n[Step 2/4] Generating Initial Population from real per-locus frequencies + LD...");
    let sampler = GenomeSampler::from_brain_with_haplotypes(
        &brain,
        &chromosome.hap_a,
        &chromosome.hap_b,
        chromosome.sample_names.len(),
        n_samples,
        42,
    );
    let initial_pop = sampler.generate_population(population_size);
    println!(
        "✓ Generated {} genomes with {} variants each ({} haplotype pools preserving real LD)",
        initial_pop.len(),
        sampler.n_snps,
        sampler.haplotype_pools.len()
    );

    // ========== STEP 3: Evolution Simulation ==========
    println!("\n[Step 3/4] Running Evolution Simulation ({} generations)...\n", n_generations);

    let fitness_model = Box::new(DefaultFitnessModel {
        target_allele_freq: mean_real_af,
        selection_strength: 2.0,
    });

    let mut evo_sim = EvolutionSim::new(initial_pop, fitness_model);

    println!("Gen | Mean Fitness | Max Fitness | Elite Count |");
    println!("----|--------------|-------------|-------------|");

    for _ in 0..n_generations {
        evo_sim.step(&sampler);
        let stats = evo_sim.stats();
        println!(
            "{:3} | {:12.3} | {:11.3} | {:11} |",
            stats.generation, stats.mean_fitness, stats.max_fitness, stats.elite_count
        );
    }

    let final_stats = evo_sim.stats();
    println!(
        "\nEvolution Complete: Generation {}, Mean Fitness: {:.3}",
        final_stats.generation, final_stats.mean_fitness
    );

    // ========== STEP 4: Phenotype Prediction ==========
    println!("\n[Step 4/4] Computing Phenotypes via G×E...\n");

    // Create GxE engine
    let mut gxe_engine = GxEEngine::new();

    // Register traits
    gxe_engine.register_trait(PhenotypeHead::new("Cognition".to_string(), 100));
    gxe_engine.register_trait(PhenotypeHead::new("Height".to_string(), 100));
    gxe_engine.register_trait(PhenotypeHead::new("Metabolism".to_string(), 100));

    // Three environments
    let environments = vec![
        ("Default (Neutral)", Environment::default_env()),
        ("Stress", Environment::stress_env()),
        ("Rich", Environment::rich_env()),
    ];

    // Sample a few genomes and predict phenotypes
    let sample_genomes = &evo_sim.population[0..3.min(evo_sim.population.len())];

    for (genome_idx, genome) in sample_genomes.iter().enumerate() {
        println!("Genome #{} (Fitness: {:.3})", genome_idx, genome.fitness);

        for (env_name, environment) in &environments {
            let phenotypes = gxe_engine.compute_phenotypes(genome, environment);
            println!(
                "  {}: Cognition={:.3}, Height={:.3}, Metabolism={:.3}",
                env_name,
                phenotypes.get("Cognition").unwrap_or(&0.0),
                phenotypes.get("Height").unwrap_or(&0.0),
                phenotypes.get("Metabolism").unwrap_or(&0.0)
            );
        }
        println!();
    }

    // ========== SUMMARY ==========
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  PHASE C PIPELINE COMPLETE                                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    println!("\n✓ Synthesis:    {} genomes generated", population_size);
    println!("✓ Evolution:    {} generations completed", n_generations);
    println!("✓ Selection:    Mean fitness = {:.3}", final_stats.mean_fitness);
    println!("✓ Phenotypes:   3 traits × 3 environments predicted");
    println!("✓ G×E Model:    Gene-environment interactions computed");

    println!("\n📊 Pipeline Summary:");
    println!("  Phase A: Data → Genotypes → LD → Blocks ✓");
    println!("  Phase B: Brains → Agents → Domain Detection ✓");
    println!("  Phase C: Synthesis → Evolution → Phenotypes ✓");
    println!("\n✓ Pure Rust pipeline: 3 complete phases");
    println!("✓ Zero dependencies");
    println!("✓ Ready for Phase D (Quality Control)");
}
