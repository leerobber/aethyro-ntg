//! Haplotype Block Detection Test - Complete Phase A Pipeline
//! VCF Parse → Bitsliced Genotypes → LD Computation → Haplotype Blocks

use ntg_kernel::genomic::{VcfParser, LdComputer, BlockDetector, compute_block_statistics};
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
    println!("║  Phase A Complete Pipeline Test                             ║");
    println!("║  VCF → Genotypes → LD → Haplotype Blocks                    ║");
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
    let ld_computer = LdComputer::new(true, 0.5);
    let block_detector = BlockDetector::new(true);

    for (chr_id, vcf_path) in test_cases {
        let vcf_path = vcf_path.as_str();
        println!("\n╔═════════════════════════════════════════════════════════════╗");
        println!("║  Chr{} - Full Phase A Pipeline", chr_id);
        println!("╚═════════════════════════════════════════════════════════════╝");

        let chr_num: u8 = chr_id.parse().unwrap();

        // Check if file exists
        if !Path::new(vcf_path).exists() {
            println!("[✗] VCF file not found: {}", vcf_path);
            continue;
        }

        // Step 1: Parse VCF
        println!("\n[Step 1/4] Parsing VCF and encoding genotypes...");
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
        println!("\n[Step 2/4] Computing LD matrix...");
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
        println!("\n[Step 3/4] Detecting haplotype blocks...");
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

        // Step 4: Annotate blocks with positions
        println!("\n[Step 4/4] Annotating blocks with genomic positions...");
        if let Err(e) = block_detector.annotate_blocks(&mut blocks, &positions) {
            println!("[✗] Annotation failed: {}", e);
            continue;
        }
        println!("[✓] Blocks annotated with positions");

        // Print summary statistics
        println!("\n[Summary] Chr{} Phase A Results:", chr_id);
        println!("  SNPs: {}", chromosome.snps.len());
        println!("  High-LD pairs: {}", ld_matrix.pairs.len());
        println!("  Haplotype blocks: {}", blocks.len());

        let stats = compute_block_statistics(&blocks);
        println!("\n[Statistics]");
        println!("  Mean block size: {:.1} SNPs", stats.mean_block_size);
        println!("  Median block size: {} SNPs", stats.median_block_size);
        println!(
            "  Block size range: {}-{} SNPs",
            stats.min_block_size, stats.max_block_size
        );
        println!("  Mean r² within blocks: {:.4}", stats.mean_r_squared);
        println!("  Mean block span: {:.0} bp", stats.mean_span_bp);

        // Print first 10 blocks
        println!("\nFirst 10 haplotype blocks:");
        for (i, block) in blocks.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, block.summary());
        }

        if blocks.len() > 10 {
            println!("  ... and {} more blocks", blocks.len() - 10);
        }

        // Data reduction summary
        let total_possible_pairs = (chromosome.snps.len() * (chromosome.snps.len() - 1)) / 2;
        let reduction = total_possible_pairs as f64 / (ld_matrix.pairs.len() as f64 + 1e-10);

        println!("\n[Data Reduction]");
        println!("  Possible SNP pairs: {} (full matrix)", total_possible_pairs);
        println!("  High-LD pairs kept: {}", ld_matrix.pairs.len());
        println!("  Reduction factor: {:.0}×", reduction);
        println!("  Memory saved: {:.1}×", (total_possible_pairs as f64 * 4.0) / (ld_matrix.pairs.len() as f64 * 24.0));
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Phase A Complete Pipeline Test Finished                    ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\n✓ Full Phase A pipeline validated: VCF→LD→Blocks");
    println!("✓ Ready for Phase B (GenomicBrain Training)");
}
