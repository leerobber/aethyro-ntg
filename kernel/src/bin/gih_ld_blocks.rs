/// Compute LD blocks for GIH population using sliding window approach
/// This is much more efficient than all-pairs computation

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gih_ld_blocks <input.csv> [--window-size 100000] [--min-r2 0.2]");
        std::process::exit(1);
    }

    let csv_path = &args[1];

    let window_size: usize = args
        .windows(2)
        .find(|w| w[0] == "--window-size")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(100_000); // 100kb default window

    let min_r2: f64 = args
        .windows(2)
        .find(|w| w[0] == "--min-r2")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0.2);

    println!("═══════════════════════════════════════════════════════════════");
    println!("GIH LD Block Detection (Sliding Window)");
    println!("═══════════════════════════════════════════════════════════════");
    println!("");
    println!("Input: {}", csv_path);
    println!("Window size: {} bp", window_size);
    println!("Min r²: {}", min_r2);
    println!("");

    let start_overall = Instant::now();

    // Load CSV data
    println!("[*] Loading genotypes...");
    let (variants, genotypes, num_samples) = load_csv(csv_path);

    println!("[OK] Loaded {} variants, {} samples", variants.len(), num_samples);
    println!("");

    // Compute blocks using sliding window
    println!("[*] Computing LD blocks in sliding windows...");
    let blocks = detect_blocks(&variants, &genotypes, num_samples, window_size, min_r2);

    let total_time = start_overall.elapsed().as_secs_f64();

    println!("");
    println!("═══════════════════════════════════════════════════════════════");
    println!("[OK] Block Detection Complete!");
    println!("═══════════════════════════════════════════════════════════════");
    println!("");

    println!("[RESULTS]");
    println!("  Total variants: {}", variants.len());
    println!("  Haplotype blocks detected: {}", blocks.len());
    println!("  Computation time: {:.2}s", total_time);
    println!("");

    // Calculate block statistics
    let mut block_sizes = Vec::new();
    let mut block_spans = Vec::new();

    for block in &blocks {
        block_sizes.push(block.snps);
        block_spans.push(block.span_bp);
    }

    block_sizes.sort();
    block_spans.sort();

    if !block_sizes.is_empty() {
        let mean_size = block_sizes.iter().sum::<usize>() as f64 / block_sizes.len() as f64;
        let median_size = block_sizes[block_sizes.len() / 2];
        let min_size = *block_sizes.first().unwrap();
        let max_size = *block_sizes.last().unwrap();

        let mean_span = block_spans.iter().sum::<usize>() as f64 / block_spans.len() as f64;

        println!("[BLOCK STATISTICS]");
        println!("  Mean block size: {:.1} SNPs", mean_size);
        println!("  Median block size: {} SNPs", median_size);
        println!("  Block size range: {} - {} SNPs", min_size, max_size);
        println!("  Mean block span: {:.0} bp", mean_span);
        println!("");
    }

    println!("[TOP 20 BLOCKS]");
    println!("{:<6} {:<12} {:<12} {:<15} {:<15}", "Block", "SNPs", "Start", "End", "Span (bp)");
    println!("{}", "-".repeat(70));

    for (i, block) in blocks.iter().take(20).enumerate() {
        println!(
            "{:<6} {:<12} {:<12} {:<15} {:<15}",
            i + 1,
            block.snps,
            block.start_pos,
            block.end_pos,
            block.span_bp
        );
    }

    if blocks.len() > 20 {
        println!("  ... and {} more blocks", blocks.len() - 20);
    }
}

#[derive(Clone, Debug)]
struct Variant {
    id: String,
    position: usize,
}

#[derive(Clone)]
struct Block {
    snps: usize,
    start_pos: usize,
    end_pos: usize,
    span_bp: usize,
}

fn load_csv(path: &str) -> (Vec<Variant>, Vec<Vec<u8>>, usize) {
    let file = File::open(path).expect("Cannot open CSV");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip header
    let _header = lines.next();

    let mut variants = Vec::new();
    let mut genotypes = Vec::new();
    let mut num_samples = 0;

    for line in lines {
        let line = line.expect("Read error");
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 3 {
            continue;
        }

        if num_samples == 0 {
            num_samples = parts.len() - 2;
        }

        let var_id = parts[0].to_string();
        let position: usize = parts[1].parse().unwrap_or(0);

        variants.push(Variant {
            id: var_id,
            position,
        });

        let mut geno = Vec::with_capacity(num_samples);
        for i in 2..std::cmp::min(2 + num_samples, parts.len()) {
            let g: u8 = parts[i].parse().unwrap_or(3);
            geno.push(g);
        }

        genotypes.push(geno);
    }

    (variants, genotypes, num_samples)
}

fn detect_blocks(
    variants: &[Variant],
    genotypes: &[Vec<u8>],
    num_samples: usize,
    window_size: usize,
    min_r2: f64,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < variants.len() {
        let window_end_pos = variants[i].position + window_size;

        // Find all variants in this window
        let mut window_end_idx = i;
        for j in (i + 1)..variants.len() {
            if variants[j].position <= window_end_pos {
                window_end_idx = j;
            } else {
                break;
            }
        }

        // Compute LD within this window
        let mut high_ld_count = 0;
        for j in i..window_end_idx {
            for k in (j + 1)..=window_end_idx {
                let r2 = compute_r2(&genotypes[j], &genotypes[k]);
                if r2 >= min_r2 {
                    high_ld_count += 1;
                }
            }
        }

        // If there's strong LD in the window, create a block
        let window_snp_count = window_end_idx - i + 1;
        let expected_pairs = (window_snp_count * (window_snp_count - 1)) / 2;
        let ld_fraction = if expected_pairs > 0 {
            high_ld_count as f64 / expected_pairs as f64
        } else {
            0.0
        };

        if ld_fraction > 0.3 || window_snp_count > 1 {
            blocks.push(Block {
                snps: window_snp_count,
                start_pos: variants[i].position,
                end_pos: variants[window_end_idx].position,
                span_bp: variants[window_end_idx].position - variants[i].position,
            });
        }

        // Move to next unprocessed variant
        i = window_end_idx + 1;
    }

    blocks
}

fn compute_r2(g1: &[u8], g2: &[u8]) -> f64 {
    if g1.len() != g2.len() {
        return 0.0;
    }

    let n = g1.len();
    let mut sum_g1 = 0.0;
    let mut sum_g2 = 0.0;
    let mut sum_g1_2 = 0.0;
    let mut sum_g2_2 = 0.0;
    let mut sum_prod = 0.0;
    let mut valid_count = 0;

    for i in 0..n {
        if g1[i] != 3 && g2[i] != 3 {
            let v1 = g1[i] as f64;
            let v2 = g2[i] as f64;
            sum_g1 += v1;
            sum_g2 += v2;
            sum_g1_2 += v1 * v1;
            sum_g2_2 += v2 * v2;
            sum_prod += v1 * v2;
            valid_count += 1;
        }
    }

    if valid_count < 2 {
        return 0.0;
    }

    let valid = valid_count as f64;
    let mean_g1 = sum_g1 / valid;
    let mean_g2 = sum_g2 / valid;

    let var_g1 = (sum_g1_2 / valid) - (mean_g1 * mean_g1);
    let var_g2 = (sum_g2_2 / valid) - (mean_g2 * mean_g2);
    let cov = (sum_prod / valid) - (mean_g1 * mean_g2);

    if var_g1 < 1e-10 || var_g2 < 1e-10 {
        return 0.0;
    }

    let r = cov / (var_g1.sqrt() * var_g2.sqrt());
    let r_clamped = r.max(-1.0).min(1.0);
    let r2 = r_clamped * r_clamped;

    r2
}
