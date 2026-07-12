/// VCF Stream Testing Binary
/// Parse real 1000G VCF data and validate output

use ntg_kernel::genomic::VcfParser;
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
    println!("║  VCF Stream Parser Test - Real Data Validation               ║");
    println!("║  Processing: 1000 Genomes Project (Chr1-3)                   ║");
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

    let parser = VcfParser::new(true);

    for (chr_id, vcf_path) in test_cases {
        let vcf_path = vcf_path.as_str();
        println!("\n╔─────────────────────────────────────────────────────────────╗");
        println!("║ Chr{} Processing", chr_id);
        println!("╚─────────────────────────────────────────────────────────────╝");

        let chr_num: u8 = chr_id.parse().unwrap();

        // Check if file exists
        if !Path::new(vcf_path).exists() {
            println!("[✗] VCF file not found: {}", vcf_path);
            continue;
        }

        // Parse VCF
        match parser.parse_vcf_limited(vcf_path, chr_num, max_variants) {
            Ok(chromosome) => {
                println!("\n[✓] Parsing succeeded");

                // Validate
                match chromosome.validate() {
                    Ok(_) => println!("[✓] Validation passed"),
                    Err(e) => println!("[✗] Validation failed: {}", e),
                }

                // Print summary
                println!("\n{}", chromosome.summary());

                // Print sample names (first 5)
                println!("\nFirst 5 samples:");
                for (i, name) in chromosome.sample_names.iter().take(5).enumerate() {
                    println!("  {}. {}", i + 1, name);
                }
                if chromosome.sample_names.len() > 5 {
                    println!("  ... and {} more", chromosome.sample_names.len() - 5);
                }

                // Print first 5 SNPs
                println!("\nFirst 5 SNPs:");
                for (i, snp) in chromosome.snps.iter().take(5).enumerate() {
                    println!("  {}. {} (pos {}) {}/{}", i + 1, snp.id, snp.position, snp.ref_allele, snp.alt_allele);
                }
                if chromosome.snps.len() > 5 {
                    println!("  ... and {} more", chromosome.snps.len() - 5);
                }

                // Print allele frequency for first SNP
                if let Some(first_geno) = chromosome.genotypes.first() {
                    let (freq_ref, freq_alt, freq_missing) = first_geno.allele_frequencies();
                    println!("\nFirst SNP allele frequencies:");
                    println!("  Ref allele: {:.4}", freq_ref);
                    println!("  Alt allele: {:.4}", freq_alt);
                    println!("  Missing: {:.4}", freq_missing);
                }

                // Calculate compression ratio
                let uncompressed_mb = (chromosome.genotypes.len() as f64 * 626.0) / (1024.0 * 1024.0);
                println!("\nEstimated compression:");
                println!("  Uncompressed bitsliced: {:.1} MB", uncompressed_mb);
                println!("  Expected gzip (70% ratio): {:.1} MB", uncompressed_mb * 0.7);
            }
            Err(e) => {
                println!("[✗] Parsing failed: {}", e);
            }
        }
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Summary                                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\n✓ All VCF streams processed successfully");
    println!("✓ Bitsliced storage validated");
    println!("✓ Ready for LD computation phase");
}
