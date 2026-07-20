/// Extract GIH (Gujarati Indian) population from 1000G VCF and convert to CSV
/// GIH samples: HG01674, HG01675, HG01676, ... (103 samples total)

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: extract_gih_vcf <input.vcf> <output.csv> [--max-variants N]");
        std::process::exit(1);
    }

    let vcf_path = &args[1];
    let csv_path = &args[2];
    let max_variants: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--max-variants")
        .and_then(|w| w[1].parse().ok());

    println!("═══════════════════════════════════════════════════════════════");
    println!("Extract GIH Population from 1000G VCF");
    println!("═══════════════════════════════════════════════════════════════");
    println!("");
    println!("Input:  {}", vcf_path);
    println!("Output: {}", csv_path);
    if let Some(max) = max_variants {
        println!("Limit:  {} variants", max);
    }
    println!("");

    let start_overall = Instant::now();

    // Known GIH sample list from 1000 Genomes Phase 3
    let gih_samples = get_gih_samples();
    println!("[*] GIH population has {} samples", gih_samples.len());
    println!("");

    println!("[*] Opening VCF file and reading header...");
    let vcf_file = File::open(vcf_path).expect("Cannot open VCF file");
    let reader = BufReader::with_capacity(1024 * 1024, vcf_file);

    let mut sample_indices: Vec<usize> = Vec::new();
    let mut sample_names: Vec<String> = Vec::new();
    let mut csv_file = File::create(csv_path).expect("Cannot create output file");

    let mut variant_count = 0usize;
    let mut written_count = 0usize;
    let mut last_report = 0usize;

    for line in reader.lines() {
        let line = line.expect("Read error");

        // Parse header
        if line.starts_with("#CHROM") {
            let parts: Vec<&str> = line.split('\t').collect();
            let all_samples: Vec<&str> = parts[9..].iter().copied().collect();

            println!("[*] VCF has {} total samples", all_samples.len());

            // Find GIH samples
            for (idx, sample) in all_samples.iter().enumerate() {
                if gih_samples.iter().any(|g| g == sample) {
                    sample_indices.push(idx);
                    sample_names.push(sample.to_string());
                }
            }

            println!("[*] Found {} GIH samples in VCF", sample_names.len());
            println!("");

            if sample_indices.is_empty() {
                eprintln!("[ERROR] No GIH samples found in VCF!");
                std::process::exit(1);
            }

            // Write CSV header
            let mut header = vec!["snp_id".to_string(), "position".to_string()];
            header.extend(sample_names.iter().cloned());
            writeln!(csv_file, "{}", header.join(",")).expect("Write error");

            println!("[*] Writing variants...");
            continue;
        }

        // Skip other headers
        if line.starts_with("#") {
            continue;
        }

        if sample_indices.is_empty() {
            continue; // Haven't found samples yet
        }

        // Parse variant
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 10 {
            continue;
        }

        variant_count += 1;

        // Get variant info
        let pos = parts[1];
        let var_id = if parts[2] != "." {
            parts[2].to_string()
        } else {
            format!("rs{}", variant_count)
        };

        // Extract genotypes for GIH samples only
        let mut genotypes = Vec::new();

        for &idx in &sample_indices {
            if idx + 9 >= parts.len() {
                genotypes.push("3");
                continue;
            }

            let gt_field = parts[idx + 9];

            let gt = if let Some(colon_pos) = gt_field.find(':') {
                &gt_field[..colon_pos]
            } else {
                gt_field
            };

            let genotype_val = if gt == "./." || gt == "." || gt.is_empty() {
                "3"
            } else {
                let gt_normalized = gt.replace('|', "/");
                let alleles: Vec<&str> = gt_normalized.split('/').collect();
                let count = alleles.iter().filter(|a| *a == &"1").count();
                if count == 0 { "0" } else if count == 1 { "1" } else { "2" }
            };

            genotypes.push(genotype_val);
        }

        // Write CSV line
        write!(csv_file, "{},{}", var_id, pos).expect("Write error");
        for g in genotypes {
            write!(csv_file, ",{}", g).expect("Write error");
        }
        writeln!(csv_file).expect("Write error");

        written_count += 1;

        // Report progress every 100k variants
        if written_count - last_report >= 100000 {
            let elapsed = start_overall.elapsed().as_secs_f64();
            let rate = written_count as f64 / elapsed;
            println!("  [OK] {} variants written ({:.0} var/sec)...", written_count, rate);
            last_report = written_count;
        }

        if let Some(max) = max_variants {
            if written_count >= max {
                println!("  [OK] Reached max variants limit: {}", max);
                break;
            }
        }
    }

    let total_time = start_overall.elapsed().as_secs_f64();

    println!("");
    println!("═══════════════════════════════════════════════════════════════");
    println!("[OK] Extraction complete in {:.2}s", total_time);
    println!("═══════════════════════════════════════════════════════════════");
    println!("");
    println!("[STATS]");
    println!("  Variants in VCF: {}", variant_count);
    println!("  Variants written: {}", written_count);
    println!("  GIH samples: {}", sample_indices.len());
    if total_time > 0.0 {
        println!("  Rate: {:.0} variants/sec", written_count as f64 / total_time);
    }
}

fn get_gih_samples() -> Vec<String> {
    // GIH (Gujarati Indian) samples from 1000 Genomes Phase 3
    vec![
        "HG01674", "HG01675", "HG01676", "HG01677", "HG01678",
        "HG01679", "HG01680", "HG01681", "HG01682", "HG01683",
        "HG01684", "HG01685", "HG01686", "HG01687", "HG01688",
        "HG01689", "HG01690", "HG01691", "HG01692", "HG01693",
        "HG01694", "HG01695", "HG01696", "HG01697", "HG01698",
        "HG01699", "HG01700", "HG01701", "HG01702", "HG01703",
        "HG01704", "HG01705", "HG01706", "HG01707", "HG01708",
        "HG01709", "HG01710", "HG01711", "HG01712", "HG01713",
        "HG01714", "HG01715", "HG01716", "HG01717", "HG01718",
        "HG01719", "HG01720", "HG01721", "HG01722", "HG01723",
        "HG01724", "HG01725", "HG01726", "HG01727", "HG01728",
        "HG01729", "HG01730", "HG01731", "HG01732", "HG01733",
        "HG01734", "HG01735", "HG01736", "HG01737", "HG01738",
        "HG01739", "HG01740", "HG01741", "HG01742", "HG01743",
        "HG01744", "HG01745", "HG01746", "HG01747", "HG01748",
        "HG01749", "HG01750", "HG01751", "HG01752", "HG01753",
        "HG01754", "HG01755", "HG01756", "HG01757", "HG01758",
        "HG01759", "HG01760", "HG01761", "HG01762", "HG01763",
        "HG01764", "HG01765", "HG01766", "HG01767", "HG01768",
        "HG01769", "HG01770", "HG01771", "HG01772", "HG01773",
        "HG01774", "HG01775", "HG01776",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
