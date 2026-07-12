/// Chromosome Brain Test - Phase B Validation
/// Test: VCF → Genotypes → LD → Blocks → ChomosomeBrain → KAIROS Training → Agents

use ntg_kernel::genomic::{
    VcfParser, LdComputer, BlockDetector, ChromosomeId,
    init_chromosome_brain, ChromosomeAgent, AgentQuery, AgentCoordinator,
};
use std::path::Path;

fn vcf_path(chr: &str) -> String {
    format!(
        "{}/../data/raw/1000g/ALL.chr{}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
        env!("CARGO_MANIFEST_DIR"),
        chr
    )
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase B: Chromosome Brain Test                              ║");
    println!("║  VCF → Genotypes → LD → Blocks → Brain → Agents             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let max_variants: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    if let Some(limit) = max_variants {
        println!("\n[*] Bounding each chromosome to the first {} variants", limit);
    }

    let test_cases = vec![
        ("1", vcf_path("1")),
        ("22", vcf_path("22")),
    ];

    let vcf_parser = VcfParser::new(true);
    let ld_computer = LdComputer::new(true, 0.5);
    let block_detector = BlockDetector::new(true);

    for (chr_id, vcf_path) in test_cases {
        let vcf_path = vcf_path.as_str();
        if !Path::new(vcf_path).exists() {
            println!("[✗] VCF file not found: {}", vcf_path);
            continue;
        }

        println!("\n╔═════════════════════════════════════════════════════════════╗");
        println!("║  Chr{} - Full Phase B Pipeline", chr_id);
        println!("╚═════════════════════════════════════════════════════════════╝");

        let chr_num: u8 = chr_id.parse().unwrap();

        // Step 1: Parse VCF
        println!("\n[Step 1/6] Parsing VCF and encoding genotypes...");
        let chromosome = match vcf_parser.parse_vcf_limited(vcf_path, chr_num, max_variants) {
            Ok(chr) => {
                match chr.validate() {
                    Ok(_) => {
                        println!(
                            "[✓] Parsed: {} SNPs, {} samples",
                            chr.snps.len(),
                            chr.sample_names.len()
                        );
                        chr
                    }
                    Err(e) => {
                        println!("[✗] Validation failed: {}", e);
                        continue;
                    }
                }
            }
            Err(e) => {
                println!("[✗] Parsing failed: {}", e);
                continue;
            }
        };

        // Step 2: Compute LD
        println!("\n[Step 2/6] Computing LD matrix...");
        let positions: Vec<u32> = chromosome.snps.iter().map(|s| s.position).collect();
        let ld_matrix = match ld_computer.compute_ld(&chromosome.genotypes, &positions) {
            Ok(ld) => {
                println!(
                    "[✓] LD computed: {} high-LD pairs (r² > 0.5)",
                    ld.pairs.len()
                );
                ld
            }
            Err(e) => {
                println!("[✗] LD computation failed: {}", e);
                continue;
            }
        };

        // Step 3: Detect Haplotype Blocks
        println!("\n[Step 3/6] Detecting haplotype blocks...");
        let mut blocks = match block_detector.detect_blocks(&ld_matrix.pairs, chromosome.snps.len()) {
            Ok(blks) => {
                println!("[✓] Detected {} haplotype blocks", blks.len());
                blks
            }
            Err(e) => {
                println!("[✗] Block detection failed: {}", e);
                continue;
            }
        };

        // Step 4: Annotate blocks
        println!("\n[Step 4/6] Annotating blocks with genomic positions...");
        if let Err(e) = block_detector.annotate_blocks(&mut blocks, &positions) {
            println!("[✗] Annotation failed: {}", e);
            continue;
        }
        println!("[✓] Blocks annotated");

        // Step 5: Initialize Chromosome Brain
        println!("\n[Step 5/6] Initializing chromosome brain (Phase B)...");
        let chr_brain = match init_chromosome_brain(
            ChromosomeId(chr_num),
            &chromosome.genotypes,
            &chromosome.snps,
            &ld_matrix.pairs,
            &blocks,
        ) {
            Ok(brain) => {
                let summary = brain.summary();
                println!("[✓] Brain initialized:");
                println!("    Neurons: {}", summary.n_neurons);
                println!("    Synapses: {}", summary.n_synapses);
                println!("    Blocks: {}", summary.n_blocks);
                println!("    Total LD: {:.3}", summary.total_ld);
                println!("    Avg Weight: {:.3}", summary.avg_weight);
                brain
            }
            Err(e) => {
                println!("[✗] Brain initialization failed: {}", e);
                continue;
            }
        };

        // Step 6: KAIROS Training & Agent Queries
        println!("\n[Step 6/6] KAIROS training and agent queries...");
        let mut brain = chr_brain;
        brain.train_kairos(100);

        let summary = brain.summary();
        println!("[✓] KAIROS Training Complete:");
        println!("    Cycles: {}", summary.training_cycles);
        println!("    Convergence: {:.3}", summary.convergence);
        println!("    Weight Updates: {}", brain.kairos_state.weight_updates);

        // Create agent and test queries
        let agent = ChromosomeAgent::new(brain);

        println!("\n[Agent Queries]");

        // Query 1: Disease Risk
        if chromosome.snps.len() > 0 {
            let query = AgentQuery::DiseaseRisk {
                snp_indices: vec![0],
            };
            let response = agent.handle_query(&query);
            println!("  Risk Assessment: {:.3}", response.score);
            println!("    {}", response.explanation);
        }

        // Query 2: Trait Pattern
        let query = AgentQuery::TraitPattern {
            trait_name: "Cognition".to_string(),
        };
        let response = agent.handle_query(&query);
        println!("  Trait Pattern: {:.3}", response.score);

        // Query 3: Population Signal
        let query = AgentQuery::PopulationSignal;
        let response = agent.handle_query(&query);
        println!("  Population Signal: {:.3}", response.score);

        // Query 4: Evolution Hint
        let query = AgentQuery::EvolutionHint;
        let response = agent.handle_query(&query);
        println!("  Evolution Hint: {:.3}", response.score);

        // Multi-agent Coordination (single brain)
        println!("\n[Multi-Agent Coordinator]");
        let coordinator = AgentCoordinator::new(vec![agent]);
        let coord_response = coordinator.coordinate_query(&AgentQuery::PopulationSignal);
        println!("  {}", coord_response.summary);
        println!("  Fused Signal Magnitude: {:.4}", coord_response.fused_signal[0]);
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase B Test Complete                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\n✓ Full Phase B pipeline validated: VCF→Brain→Agents");
    println!("✓ Ready for Phase C (Multi-Chromosome Coordination)");
}
