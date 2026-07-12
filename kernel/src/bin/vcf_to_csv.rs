/// Ultra-fast VCF to CSV converter
/// Uses Rust for 50-100x speedup vs Python

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use flate2::read::GzDecoder;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: vcf_to_csv <input.vcf.gz> <output.csv>");
        std::process::exit(1);
    }

    let vcf_path = &args[1];
    let csv_path = &args[2];

    println!("═══════════════════════════════════════════════════");
    println!("VCF → CSV Converter (Rust)");
    println!("═══════════════════════════════════════════════════");
    println!("");
    println!("Input:  {}", vcf_path);
    println!("Output: {}", csv_path);
    println!("");

    let start = std::time::Instant::now();

    // Open gzipped VCF
    let vcf_file = File::open(vcf_path).expect("Cannot open VCF file");
    let gz = GzDecoder::new(vcf_file);
    let reader = BufReader::with_capacity(1024 * 1024, gz);

    let mut csv_file = File::create(csv_path).expect("Cannot create CSV file");
    let mut samples: Vec<String> = Vec::new();
    let mut variant_count = 0u64;

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.expect("Read error");

        // Parse header
        if line.starts_with("#CHROM") {
            let parts: Vec<&str> = line.split('\t').collect();
            samples = parts[9..].iter().map(|s| s.to_string()).collect();

            // Write CSV header
            writeln!(csv_file, "snp_id,position,{}", samples.join(","))
                .expect("Write error");

            println!("✓ Found {} samples", samples.len());
            continue;
        }

        // Skip other headers
        if line.starts_with("#") {
            continue;
        }

        // Parse variant
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 10 {
            continue;
        }

        let chrom = parts[0];
        let pos = parts[1];
        let var_id = if parts[2] != "." {
            parts[2].to_string()
        } else {
            format!("chr{}_{}", chrom, pos)
        };

        // Extract genotypes
        let mut genotypes = Vec::with_capacity(samples.len());
        for i in 9..std::cmp::min(9 + samples.len(), parts.len()) {
            let gt_field = parts[i];

            let gt = if let Some(colon_pos) = gt_field.find(':') {
                &gt_field[..colon_pos]
            } else {
                gt_field
            };

            let genotype_val = if gt == "./." || gt == "." {
                '3'
            } else {
                let gt_normalized = gt.replace('|', "/");
                let alleles: Vec<&str> = gt_normalized.split('/').collect();
                let count = alleles.iter().filter(|a| *a == &"1").count();
                (count as u8 + b'0') as char
            };

            genotypes.push(genotype_val);
        }

        // Pad if needed
        while genotypes.len() < samples.len() {
            genotypes.push('3');
        }

        // Write CSV row
        write!(csv_file, "{},{}", var_id, pos).expect("Write error");
        for g in &genotypes[..samples.len()] {
            write!(csv_file, ",{}", g).expect("Write error");
        }
        writeln!(csv_file).expect("Write error");

        variant_count += 1;
        if variant_count % 100000 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = variant_count as f64 / elapsed;
            println!("  ✓ {} variants ({:.0}/sec)...", variant_count, rate);
        }
    }

    let elapsed = start.elapsed();
    println!("");
    println!("═══════════════════════════════════════════════════");
    println!("✓ Conversion Complete!");
    println!("═══════════════════════════════════════════════════");
    println!("");
    println!("Variants: {}", variant_count);
    println!("Samples: {}", samples.len());
    println!("Duration: {:.1}s", elapsed.as_secs_f64());
    println!("Rate: {:.0} variants/sec", variant_count as f64 / elapsed.as_secs_f64());
    println!("");
}
