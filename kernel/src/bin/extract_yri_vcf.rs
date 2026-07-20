/// Extract YRI (Yoruba in Ibadan, Nigeria) population from 1000G VCF and convert to CSV
/// YRI samples: NA18501-NA18522 (parent-child trios) + NA19000-NA19103 (unrelated) = 108 samples total

use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: extract_yri_vcf <input.vcf.gz> <output.csv> [--max-variants N]");
        std::process::exit(1);
    }

    let vcf_path = &args[1];
    let csv_path = &args[2];
    let max_variants: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--max-variants")
        .and_then(|w| w[1].parse().ok());

    println!("═══════════════════════════════════════════════════════════════");
    println!("Extract YRI Population from 1000G VCF");
    println!("═══════════════════════════════════════════════════════════════");
    println!("");
    println!("Population: Yoruba in Ibadan (Nigeria) - West African");
    println!("Input:  {}", vcf_path);
    println!("Output: {}", csv_path);
    println!("");

    let start = std::time::Instant::now();

    // Known YRI sample list from 1000 Genomes phase 3
    // These are the 108 YRI samples (parent-child trios + unrelated)
    let yri_samples = get_yri_samples();

    println!("[*] Loading VCF file and filtering for YRI samples...");
    println!("    Total YRI samples: {}", yri_samples.len());
    println!("");

    // Use system unzip if available, otherwise fail
    #[cfg(windows)]
    let output = std::process::Command::new("powershell")
        .args(&["-Command", &format!("Get-Content '{}' -ReadCount 0 | ForEach-Object {{ $_ }}", vcf_path)])
        .output();

    #[cfg(not(windows))]
    let output = std::process::Command::new("gunzip")
        .args(&["-c", vcf_path])
        .output();

    let vcf_content = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => {
            // Fallback: try reading directly if not gzipped
            std::fs::read_to_string(vcf_path).expect("Cannot open VCF file")
        }
    };

    let lines: Vec<&str> = vcf_content.lines().collect();
    process_vcf_lines(&lines, &yri_samples, csv_path, max_variants);

    let elapsed = start.elapsed().as_secs_f64();
    println!("");
    println!("═══════════════════════════════════════════════════════════════");
    println!("[OK] Extraction complete in {:.2}s", elapsed);
    println!("═══════════════════════════════════════════════════════════════");
}

fn process_vcf_lines(
    lines: &[&str],
    yri_samples: &[String],
    csv_path: &str,
    max_variants: Option<usize>,
) {
    let mut sample_indices: Vec<usize> = Vec::new();
    let mut sample_names: Vec<String> = Vec::new();
    let mut csv_file = File::create(csv_path).expect("Cannot create output file");

    let mut variant_count = 0u64;
    let mut written_count = 0u64;

    for line in lines {
        // Parse header
        if line.starts_with("#CHROM") {
            let parts: Vec<&str> = line.split('\t').collect();
            let all_samples: Vec<&str> = parts[9..].iter().map(|s| *s).collect();

            println!("[*] VCF has {} total samples", all_samples.len());

            // Find YRI samples
            for (idx, sample) in all_samples.iter().enumerate() {
                if yri_samples.contains(&sample.to_string()) {
                    sample_indices.push(idx);
                    sample_names.push(sample.to_string());
                }
            }

            println!("[*] Found {} YRI samples in VCF", sample_names.len());
            println!("");

            if sample_indices.is_empty() {
                eprintln!("[ERROR] No YRI samples found in VCF!");
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
        let chrom = parts[0];
        let pos = parts[1];
        let var_id = if parts[2] != "." {
            parts[2].to_string()
        } else {
            format!("chr{}_{}", chrom, pos)
        };

        // Extract genotypes for YRI samples only
        let mut genotypes = Vec::new();

        for &idx in &sample_indices {
            if idx + 9 >= parts.len() {
                genotypes.push("3".to_string());
                continue;
            }

            let gt_field = parts[idx + 9];

            let gt = if let Some(colon_pos) = gt_field.find(':') {
                &gt_field[..colon_pos]
            } else {
                gt_field
            };

            let genotype_val = if gt == "./." || gt == "." || gt.is_empty() {
                "3".to_string()
            } else {
                let gt_normalized = gt.replace('|', "/");
                let alleles: Vec<&str> = gt_normalized.split('/').collect();
                let count = alleles.iter().filter(|a| *a == &"1").count();
                count.to_string()
            };

            genotypes.push(genotype_val);
        }

        // Write CSV line
        let mut row = vec![var_id, pos.to_string()];
        row.extend(genotypes);
        writeln!(csv_file, "{}", row.join(",")).expect("Write error");

        written_count += 1;

        if written_count % 100000 == 0 {
            println!("  [OK] {} variants written...", written_count);
        }

        if let Some(max) = max_variants {
            if written_count >= max as u64 {
                println!("  [OK] Reached max variants limit: {}", max);
                break;
            }
        }
    }

    println!("");
    println!("[STATS]");
    println!("  Total variants in VCF: {}", variant_count);
    println!("  Variants written: {}", written_count);
    println!("  YRI samples: {}", sample_indices.len());
}

fn get_yri_samples() -> Vec<String> {
    // YRI (Yoruba in Ibadan, Nigeria) samples from 1000 Genomes Phase 3
    // Parent-child trios (22 samples)
    let mut samples = vec![
        "NA18501".to_string(), "NA18502".to_string(), "NA18503".to_string(), "NA18504".to_string(), "NA18505".to_string(),
        "NA18506".to_string(), "NA18507".to_string(), "NA18508".to_string(), "NA18509".to_string(), "NA18510".to_string(),
        "NA18511".to_string(), "NA18512".to_string(), "NA18513".to_string(), "NA18514".to_string(), "NA18515".to_string(),
        "NA18516".to_string(), "NA18517".to_string(), "NA18518".to_string(), "NA18519".to_string(), "NA18520".to_string(),
        "NA18521".to_string(), "NA18522".to_string(),
    ];

    // Unrelated individuals (104 samples)
    for i in 19000..=19103 {
        samples.push(format!("NA{}", i));
    }

    samples
}
