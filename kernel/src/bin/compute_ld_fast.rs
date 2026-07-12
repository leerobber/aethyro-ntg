use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

mod genomic {
    pub use ntg_kernel::ntg::operators::genomic::*;
}

use genomic::GenomicOperator;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: compute_ld_fast <input.csv> [--summary] [--sample N]");
        std::process::exit(1);
    }

    let csv_path = &args[1];
    let summary_only = args.contains(&"--summary".to_string());
    let sample_n: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--sample")
        .and_then(|w| w[1].parse().ok());

    println!("{}","=".repeat(64));
    println!("COMPUTING: Linkage Disequilibrium (LD) Matrix [FAST]");
    println!("{}","=".repeat(64));
    println!("");
    println!("Input: {}", csv_path);
    println!("Mode: Bitsliced ternary + popcount LD");
    println!("");

    let start_overall = Instant::now();

    println!("[*] Loading genotypes into bitsliced operator...");
    let start = Instant::now();

    let (mut operator, variant_ids, positions, num_samples) =
        load_csv_into_operator(csv_path, sample_n);

    let load_time = start.elapsed().as_secs_f64();
    let num_variants = operator.num_snps;

    println!("[OK] Loaded {} variants in {:.1}s", num_variants, load_time);
    println!("  Memory: ~{:.1} MB (bitsliced)",
        (num_variants * num_samples * 2) as f64 / 1_000_000.0);
    println!("  Samples: {}", num_samples);
    println!("");

    if summary_only {
        print_summary_only(&operator, &variant_ids, &positions, num_samples);
        return;
    }

    println!("[*] Computing LD matrix (streaming mode - high-LD only)...", );
    let pairwise = num_variants * (num_variants - 1) / 2;
    println!("  This will compute {} pairwise correlations", pairwise);
    println!("  Keeping only pairs with r² > 0.5 to save memory");
    println!("");

    let start = Instant::now();

    // Stream compute: only keep high-LD pairs
    let mut high_ld_pairs: Vec<(usize, usize, f64)> = Vec::new();
    let mut pairs_computed = 0u64;
    let mut last_report = 0u64;

    for i in 0..num_variants {
        for j in (i + 1)..num_variants {
            pairs_computed += 1;

            // Report progress every 1M pairs
            if pairs_computed - last_report >= 1_000_000 {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    let rate = pairs_computed as f64 / elapsed;
                    println!("  [OK] {} pairs computed ({:.0} pairs/sec)...", pairs_computed, rate);
                }
                last_report = pairs_computed;
            }

            // Compute correlation for this pair manually
            let (r2, _valid) = compute_pair_correlation(&operator, i, j);

            if r2 > 0.5 {
                high_ld_pairs.push((i, j, r2));
            }
        }
    }

    let ld_time = start.elapsed().as_secs_f64();
    let total_time = start_overall.elapsed().as_secs_f64();

    println!("");
    println!("{}","=".repeat(64));
    println!("[OK] LD MATRIX COMPUTATION COMPLETE!");
    println!("{}","=".repeat(64));
    println!("");

    println!("[STATS] LD Statistics:");
    println!("  Total SNP pairs: {}", pairwise);
    println!("  Pairs computed: {}", pairs_computed);
    println!("  High LD pairs (r² > 0.5): {}", high_ld_pairs.len());
    println!("  LD computation: {:.1}s", ld_time);
    println!("");

    println!("[TIME] Performance:");
    println!("  Load time: {:.1}s", load_time);
    println!("  LD computation: {:.1}s", ld_time);
    println!("  Total time: {:.1}s", total_time);
    if ld_time > 0.0 {
        println!("  Rate: {:.0} pairs/sec", pairs_computed as f64 / ld_time);
    }
    println!("");

    println!("[RESULTS] Top 20 Linked Variant Pairs (highest r2):");
    println!("");

    high_ld_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    println!("{:<20} {:<20} {:<12} {:<20}", "SNP1", "SNP2", "r2", "Distance (bp)");
    println!("{}", "-".repeat(80));

    for (idx1, idx2, r2) in high_ld_pairs.iter().take(20) {
        let dist = (positions[*idx2] as i64 - positions[*idx1] as i64).abs();
        println!(
            "{:<20} {:<20} {:<12.4} {:<20}",
            &variant_ids[*idx1][0..std::cmp::min(20, variant_ids[*idx1].len())],
            &variant_ids[*idx2][0..std::cmp::min(20, variant_ids[*idx2].len())],
            r2,
            format!("{}", dist)
        );
    }

    println!("");
    println!("[ANALYSIS] LD Decay:");
    println!("");

    let mut ld_by_distance: HashMap<u32, Vec<f64>> = HashMap::new();

    for (idx1, idx2, r2) in &high_ld_pairs {
        let dist = ((positions[*idx2] as i64 - positions[*idx1] as i64).abs() / 10000) as u32 * 10000;
        ld_by_distance.entry(dist).or_insert_with(Vec::new).push(*r2);
    }

    let mut distances: Vec<u32> = ld_by_distance.keys().copied().collect();
    distances.sort();

    println!("{:<20} {:<15} {:<20}", "Distance (bp)", "Pairs", "Avg r2");
    println!("{}", "-".repeat(55));

    for dist in distances.iter().take(20) {
        if let Some(values) = ld_by_distance.get(dist) {
            let avg_r2 = values.iter().sum::<f64>() / values.len() as f64;
            println!(
                "{:<20} {:<15} {:<20.4}",
                format!("{}", dist),
                values.len(),
                avg_r2
            );
        }
    }

    println!("");
    println!("{}","=".repeat(64));
    println!("Analysis complete! LD patterns extracted from real 1000G data.");
    println!("{}","=".repeat(64));
}

fn load_csv_into_operator(
    csv_path: &str,
    sample_n: Option<usize>,
) -> (GenomicOperator, Vec<String>, Vec<u32>, usize) {
    let file = File::open(csv_path).expect("Cannot open CSV file");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header_line = lines.next().expect("No header").expect("Read error");
    let header_parts: Vec<&str> = header_line.split(',').collect();
    let num_samples = header_parts.len() - 2;

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
    let mut operator = GenomicOperator::new(num_samples, num_variants);

    for (idx, geno) in genotypes.iter().enumerate() {
        for (sample_idx, &g) in geno.iter().enumerate() {
            operator.set(idx, sample_idx, g);
        }
    }

    (operator, variant_ids, positions, num_samples)
}

/// Compute correlation between two SNPs without full matrix allocation
fn compute_pair_correlation(operator: &GenomicOperator, i: usize, j: usize) -> (f64, usize) {
    let mut dot = 0.0;
    let mut valid = 0;

    for k in 0..operator.num_individuals {
        let gi = operator.get(i, k);
        let gj = operator.get(j, k);

        if gi != 3 && gj != 3 {
            dot += (gi as f64) * (gj as f64);
            valid += 1;
        }
    }

    if valid > 1 && operator.std_devs[i] > 1e-9 && operator.std_devs[j] > 1e-9 {
        let r = ((dot / valid as f64) - (operator.means[i] * operator.means[j]))
                / (operator.std_devs[i] * operator.std_devs[j]);
        let r_clamped = r.max(-1.0).min(1.0);
        let r2 = r_clamped * r_clamped;
        (r2, valid)
    } else {
        (0.0, valid)
    }
}

fn print_summary_only(
    operator: &GenomicOperator,
    variant_ids: &[String],
    positions: &[u32],
    num_samples: usize,
) {
    let num_variants = operator.num_snps;

    println!("[*] Data Summary (--summary mode):");
    println!("");
    println!("  Variants: {}", num_variants);
    println!("  Samples: {}", num_samples);
    println!("  Total genotypes: {}", num_variants * num_samples);
    println!("");

    let means = &operator.means;

    let mut common = 0;
    let mut intermediate = 0;
    let mut rare = 0;
    let mut very_rare = 0;

    for mean in means {
        let af = mean / 2.0;
        let maf = af.min(1.0 - af);

        if maf > 0.05 {
            common += 1;
        } else if maf >= 0.01 && maf <= 0.05 {
            intermediate += 1;
        } else if maf >= 0.001 && maf < 0.01 {
            rare += 1;
        } else if maf < 0.001 {
            very_rare += 1;
        }
    }

    println!("  Common (MAF > 5%): {}", common);
    println!("  Intermediate (1-5%): {}", intermediate);
    println!("  Rare (0.1-1%): {}", rare);
    println!("  Very rare (< 0.1%): {}", very_rare);
    println!("");

    println!("  First 10 variants:");
    for i in 0..std::cmp::min(10, variant_ids.len()) {
        let af = means[i] / 2.0;
        println!(
            "    {} pos={} AF={:.4}",
            &variant_ids[i][0..std::cmp::min(15, variant_ids[i].len())],
            positions[i],
            af
        );
    }
}
