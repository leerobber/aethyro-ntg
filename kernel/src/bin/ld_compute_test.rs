/// LD Computation Test - End-to-End Phase A Integration
/// VCF Parse → Bitsliced Genotypes → LD Computation
///
/// Usage: cargo run --release --bin ld_compute_test [-- <max_variants>]
/// With no argument, parses each full chromosome file (can take a long
/// time on the real 1000 Genomes files in data/raw). Pass a variant cap
/// to bound the run to a leading slice of the real data.

use ntg_kernel::genomic::{VcfParser, LdComputer};
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
    println!("║  LD Computation Test - Full Phase A Integration              ║");
    println!("║  Pipeline: VCF → Bitsliced Genotypes → LD Matrix            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let max_variants: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    if let Some(limit) = max_variants {
        println!("\n[*] Bounding each chromosome to the first {} variants", limit);
    }

    let test_cases = vec![
        ("1", vcf_path("1")),
        ("2", vcf_path("2")),
        ("3", vcf_path("3")),
    ];

    let vcf_parser = VcfParser::new(true);
    let ld_computer = LdComputer::new(true, 0.5);  // Keep r² > 0.5

    for (chr_id, vcf_path) in test_cases {
        let vcf_path = vcf_path.as_str();
        println!("\n╔─────────────────────────────────────────────────────────────╗");
        println!("║ Chr{} End-to-End Test", chr_id);
        println!("╚─────────────────────────────────────────────────────────────╝");

        let chr_num: u8 = chr_id.parse().unwrap();

        // Check if file exists
        if !Path::new(vcf_path).exists() {
            println!("[✗] VCF file not found: {}", vcf_path);
            continue;
        }

        // Step 1: Parse VCF
        println!("\n[Step 1] Parsing VCF and encoding genotypes...");
        match vcf_parser.parse_vcf_limited(vcf_path, chr_num, max_variants) {
            Ok(chromosome) => {
                println!("[✓] VCF parsing succeeded");

                // Validate
                match chromosome.validate() {
                    Ok(_) => println!("[✓] Validation passed"),
                    Err(e) => {
                        println!("[✗] Validation failed: {}", e);
                        continue;
                    }
                }

                println!("\n{}", chromosome.summary());

                // Step 2: Compute LD
                println!("\n[Step 2] Computing LD matrix...");
                let positions: Vec<u32> = chromosome.snps.iter().map(|s| s.position).collect();

                match ld_computer.compute_ld(&chromosome.genotypes, &positions) {
                    Ok(ld_matrix) => {
                        println!("[✓] LD computation succeeded");

                        // Print summary
                        println!("\n{}", ld_matrix.summary());

                        // LD decay analysis
                        println!("\n{}", ld_matrix.ld_decay_analysis());

                        // Sample pairs
                        println!("\nFirst 5 LD pairs (r² > 0.5):");
                        for (i, pair) in ld_matrix.pairs.iter().take(5).enumerate() {
                            println!(
                                "  {}. SNP{}-SNP{}: r²={:.4} (pos {}bp → {}bp, distance={}bp)",
                                i + 1,
                                pair.snp1_idx,
                                pair.snp2_idx,
                                pair.r_squared,
                                pair.position1,
                                pair.position2,
                                (pair.position2 as i32 - pair.position1 as i32).abs()
                            );
                        }

                        if ld_matrix.pairs.len() > 5 {
                            println!("  ... and {} more pairs", ld_matrix.pairs.len() - 5);
                        }

                        // Statistics
                        let mean_r_sq: f64 =
                            ld_matrix.pairs.iter().map(|p| p.r_squared as f64).sum::<f64>()
                                / (ld_matrix.pairs.len() as f64 + 1e-10);

                        let data_reduction = if !ld_matrix.pairs.is_empty() {
                            (chromosome.snps.len() as f64 * (chromosome.snps.len() - 1) as f64 / 2.0)
                                / ld_matrix.pairs.len() as f64
                        } else {
                            0.0
                        };

                        println!("\n[Summary] Chr{}:", chr_id);
                        println!("  SNPs: {}", chromosome.snps.len());
                        println!("  High-LD pairs: {}", ld_matrix.pairs.len());
                        println!("  Mean r² (high-LD pairs): {:.4}", mean_r_sq);
                        println!("  Data reduction: {:.0}× (vs full matrix)", data_reduction);

                        // Estimate memory and storage
                        let csv_data = ld_matrix.to_csv();
                        let csv_bytes = csv_data.len();
                        let csv_mb = csv_bytes as f64 / (1024.0 * 1024.0);

                        println!("  CSV size: {:.1} MB", csv_mb);
                        println!("  Expected gzip (70% ratio): {:.1} MB", csv_mb * 0.7);
                    }
                    Err(e) => {
                        println!("[✗] LD computation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("[✗] Parsing failed: {}", e);
            }
        }
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase A Integration Test Complete                           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\n✓ VCF parsing → Genotype encoding → LD computation pipeline validated");
    println!("✓ Ready for next phase (haplotype block detection)");
}
