/// Compute LD (Linkage Disequilibrium) Matrix from 1000 Genomes
/// Shows how genetic variants correlate across the population

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: compute_ld <input.csv> [--summary] [--sample N]");
        std::process::exit(1);
    }

    let csv_path = &args[1];
    let summary_only = args.contains(&"--summary".to_string());
    let sample_n: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--sample")
        .and_then(|w| w[1].parse().ok());

    println!("════════════════════════════════════════════════════════════════");
    println!("COMPUTING: Linkage Disequilibrium (LD) Matrix");
    println!("════════════════════════════════════════════════════════════════");
    println!("");
    println!("Input: {}", csv_path);
    println!("");

    let start_overall = Instant::now();

    // Load CSV
    println!("📖 Loading genotypes...");
    let start = Instant::now();

    let file = File::open(csv_path).expect("Cannot open CSV file");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Parse header
    let header_line = lines.next().expect("No header").expect("Read error");
    let header_parts: Vec<&str> = header_line.split(',').collect();
    let num_samples = header_parts.len() - 2;

    println!("✓ Found {} samples", num_samples);

    // Load genotypes
    let mut genotypes: Vec<Vec<u8>> = Vec::new();
    let mut variant_ids: Vec<String> = Vec::new();
    let mut positions: Vec<u32> = Vec::new();

    for line in lines {
        let line = line.expect("Read error");
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 3 {
            continue;
        }

        variant_ids.push(parts[0].to_string());
        positions.push(parts[1].parse().unwrap_or(0));

        let mut geno = Vec::with_capacity(num_samples);
        for i in 2..std::cmp::min(2 + num_samples, parts.len()) {
            let val: u8 = parts[i].parse().unwrap_or(3);
            geno.push(val);
        }

        genotypes.push(geno);

        if sample_n.is_some() && genotypes.len() >= sample_n.unwrap() {
            break;
        }
    }

    let num_variants = genotypes.len();
    let load_time = start.elapsed().as_secs_f64();

    println!("✓ Loaded {} variants in {:.1}s", num_variants, load_time);
    println!("  Memory: ~{:.1} MB", (num_variants * num_samples) as f64 / 1_000_000.0);
    println!("");

    if summary_only {
        print_summary_only(&genotypes, &variant_ids, &positions, num_samples);
        return;
    }

    // Compute statistics
    println!("📊 Computing statistics...");
    let start = Instant::now();

    let mut means = vec![0.0; num_variants];
    let mut variances = vec![0.0; num_variants];

    for (v, geno) in genotypes.iter().enumerate() {
        let mut sum = 0.0;
        let mut sum2 = 0.0;
        let mut valid = 0.0;

        for &g in geno {
            if g != 3 {
                // Skip missing
                sum += g as f64;
                sum2 += (g as f64) * (g as f64);
                valid += 1.0;
            }
        }

        if valid > 0.0 {
            means[v] = sum / valid;
            variances[v] = (sum2 / valid) - (means[v] * means[v]);
        }
    }

    let stats_time = start.elapsed().as_secs_f64();
    println!("✓ Statistics computed in {:.1}s", stats_time);
    println!("");

    // Compute LD matrix
    println!("🔗 Computing LD matrix ({} SNPs)...", num_variants);
    println!("  This will compute {} pairwise correlations", num_variants * (num_variants - 1) / 2);
    println!("");

    let start = Instant::now();
    let mut ld_count = 0;
    let mut high_ld_pairs: Vec<(usize, usize, f64)> = Vec::new();

    for i in 0..num_variants {
        for j in (i + 1)..num_variants {
            let mut dot = 0.0;
            let mut valid = 0.0;

            for k in 0..num_samples {
                let gi = genotypes[i][k];
                let gj = genotypes[j][k];

                if gi != 3 && gj != 3 {
                    dot += (gi as f64) * (gj as f64);
                    valid += 1.0;
                }
            }

            if valid > 0.0 && variances[i] > 1e-9 && variances[j] > 1e-9 {
                let r = ((dot / valid) - (means[i] * means[j])) / (variances[i].sqrt() * variances[j].sqrt());
                let r_clamped = r.max(-1.0).min(1.0);
                let r2 = r_clamped * r_clamped;

                // Track high LD pairs (r² > 0.5)
                if r2 > 0.5 {
                    high_ld_pairs.push((i, j, r2));
                }

                ld_count += 1;

                if ld_count % 1000000 == 0 {
                    let elapsed = start.elapsed().as_secs_f64();
                    let rate = ld_count as f64 / elapsed;
                    println!("  ✓ {} pairs computed ({:.0} pairs/sec)...", ld_count, rate);
                }
            }
        }
    }

    let ld_time = start.elapsed().as_secs_f64();
    let total_time = start_overall.elapsed().as_secs_f64();

    println!("");
    println!("════════════════════════════════════════════════════════════════");
    println!("✅ LD MATRIX COMPUTATION COMPLETE!");
    println!("════════════════════════════════════════════════════════════════");
    println!("");

    println!("📈 LD Statistics:");
    println!("  Total SNP pairs: {}", num_variants * (num_variants - 1) / 2);
    println!("  Pairs computed: {}", ld_count);
    println!("  High LD pairs (r² > 0.5): {}", high_ld_pairs.len());
    println!("");

    println!("⏱️  Performance:");
    println!("  Load time: {:.1}s", load_time);
    println!("  Statistics: {:.1}s", stats_time);
    println!("  LD computation: {:.1}s", ld_time);
    println!("  Total time: {:.1}s", total_time);
    println!("  Rate: {:.0} pairs/sec", ld_count as f64 / ld_time);
    println!("");

    // Print top LD pairs
    if !high_ld_pairs.is_empty() {
        println!("🔗 Top 20 Linked Variant Pairs (highest r²):");
        println!("");

        high_ld_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        println!("{:<20} {:<20} {:<12} {:<20}", "SNP1", "SNP2", "r²", "Distance (bp)");
        println!("{}", "─".repeat(80));

        for (i, (idx1, idx2, r2)) in high_ld_pairs.iter().take(20).enumerate() {
            let dist = (positions[*idx2] as i64 - positions[*idx1] as i64).abs();
            println!(
                "{:<20} {:<20} {:<12.4} {:<20}",
                &variant_ids[*idx1][0..std::cmp::min(20, variant_ids[*idx1].len())],
                &variant_ids[*idx2][0..std::cmp::min(20, variant_ids[*idx2].len())],
                r2,
                format!("{} bp", dist)
            );
        }

        println!("");
    }

    // LD decay analysis
    println!("📉 LD Decay Analysis:");
    println!("");

    let mut ld_by_distance: std::collections::HashMap<u32, Vec<f64>> = std::collections::HashMap::new();

    for (idx1, idx2, r2) in &high_ld_pairs {
        let dist = ((positions[*idx2] as i64 - positions[*idx1] as i64).abs() / 10000) as u32 * 10000;
        ld_by_distance.entry(dist).or_insert_with(Vec::new).push(*r2);
    }

    let mut distances: Vec<u32> = ld_by_distance.keys().copied().collect();
    distances.sort();

    println!("{:<20} {:<15} {:<20}", "Distance (bp)", "Pairs", "Avg r²");
    println!("{}", "─".repeat(55));

    for dist in distances.iter().take(20) {
        if let Some(values) = ld_by_distance.get(dist) {
            let avg_r2 = values.iter().sum::<f64>() / values.len() as f64;
            println!(
                "{:<20} {:<15} {:<20.4}",
                format!("{} bp", dist),
                values.len(),
                avg_r2
            );
        }
    }

    println!("");
    println!("════════════════════════════════════════════════════════════════");
    println!("Analysis complete! LD patterns extracted from real 1000G data.");
    println!("════════════════════════════════════════════════════════════════");
}

fn print_summary_only(
    genotypes: &[Vec<u8>],
    variant_ids: &[String],
    positions: &[u32],
    num_samples: usize,
) {
    println!("📊 Data Summary (--summary mode):");
    println!("");
    println!("  Variants: {}", genotypes.len());
    println!("  Samples: {}", num_samples);
    println!("  Total genotypes: {}", genotypes.len() * num_samples);
    println!("");

    // Compute allele frequencies
    let mut allele_freq = vec![0.0; genotypes.len()];

    for (v, geno) in genotypes.iter().enumerate() {
        let mut sum = 0.0;
        let mut valid = 0.0;

        for &g in geno {
            if g != 3 {
                sum += g as f64;
                valid += 1.0;
            }
        }

        if valid > 0.0 {
            allele_freq[v] = sum / (valid * 2.0); // Divide by 2 since each person has 2 alleles
        }
    }

    let maf: Vec<f64> = allele_freq.iter().map(|&af| af.min(1.0 - af)).collect();

    let rare = maf.iter().filter(|&&m| m < 0.001).count();
    let very_rare = maf.iter().filter(|&&m| m < 0.01).count();
    let common = maf.iter().filter(|&&m| m > 0.05).count();

    println!("  Common (MAF > 5%): {}", common);
    println!("  Intermediate (1-5%): {}", maf.iter().filter(|&&m| m >= 0.01 && m <= 0.05).count());
    println!("  Rare (0.1-1%): {}", maf.iter().filter(|&&m| m >= 0.001 && m < 0.01).count());
    println!("  Very rare (< 0.1%): {}", very_rare);
    println!("");

    println!("  First 10 variants:");
    for i in 0..std::cmp::min(10, variant_ids.len()) {
        println!(
            "    {} pos={} AF={:.4}",
            &variant_ids[i][0..std::cmp::min(15, variant_ids[i].len())],
            positions[i],
            allele_freq[i]
        );
    }
}
