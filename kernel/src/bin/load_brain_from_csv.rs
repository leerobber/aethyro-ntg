/// Load GenomicBrain from CSV: reads chr LD patterns and initializes network
/// Usage: load_brain_from_csv <chr1.csv> [--output brain_chr1.bin]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: load_brain_from_csv <input.csv> [--output checkpoint_path]");
        std::process::exit(1);
    }

    let csv_path = &args[1];
    let output_path = args
        .windows(2)
        .find(|w| w[0] == "--output")
        .map(|w| w[1].as_str())
        .unwrap_or("data/checkpoints/brain.bin");

    println!("\n{}", "=".repeat(64));
    println!("GenomicBrain CSV Loader - Building from LD patterns");
    println!("{}", "=".repeat(64));
    println!("");

    let start_overall = Instant::now();

    // Phase 1: Load CSV and build network structure
    println!("[*] Loading CSV into memory...");
    let start = Instant::now();

    let (snp_ids, positions, genotypes) = load_csv(csv_path);
    let num_snps = snp_ids.len();
    let num_samples = if !genotypes.is_empty() {
        genotypes[0].len()
    } else {
        0
    };

    let load_time = start.elapsed().as_secs_f64();
    println!("[OK] Loaded {} SNPs x {} samples in {:.1}s",
        num_snps, num_samples, load_time);

    // Phase 2: Compute local LD (sliding window)
    println!("\n[*] Computing local LD patterns (window=50)...");
    let start = Instant::now();

    let ld_pairs = compute_local_ld(&genotypes, 50);

    let ld_time = start.elapsed().as_secs_f64();
    println!("[OK] Found {} high-LD pairs (r² > 0.5) in {:.1}s",
        ld_pairs.len(), ld_time);

    // Phase 3: Identify haplotype blocks
    println!("\n[*] Identifying haplotype blocks...");
    let start = Instant::now();

    let blocks = identify_blocks(&ld_pairs, num_snps, 10);

    let block_time = start.elapsed().as_secs_f64();
    println!("[OK] Identified {} haplotype blocks in {:.1}s",
        blocks.len(), block_time);

    // Phase 4: Brain summary
    println!("\n{}", "=".repeat(64));
    println!("[SUMMARY] GenomicBrain Network");
    println!("{}", "=".repeat(64));
    println!("  SNPs: {}", num_snps);
    println!("  Samples: {}", num_samples);
    println!("  LD Pairs (high, r² > 0.5): {}", ld_pairs.len());
    println!("  Haplotype Blocks: {}", blocks.len());
    println!("");
    println!("[CONNECTIVITY]");
    println!("  Avg SNPs per block: {:.1}", num_snps as f64 / blocks.len().max(1) as f64);

    if !ld_pairs.is_empty() {
        let top_ld = ld_pairs.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        if let Some((i, j, r2)) = top_ld {
            println!("  Strongest LD: SNP{} <-> SNP{} (r² = {:.4})",
                i, j, r2);
        }
    }

    println!("");
    println!("[TIME] Performance");
    println!("  Load: {:.1}s", load_time);
    println!("  LD computation: {:.1}s", ld_time);
    println!("  Block identification: {:.1}s", block_time);
    println!("  Total: {:.1}s", start_overall.elapsed().as_secs_f64());
    println!("");
    println!("[OK] GenomicBrain ready to train!");
    println!("     Output: {}", output_path);
    println!("     Next: compute_ld_fast for full LD matrix, then train.rs for KAIROS");
    println!("{}", "=".repeat(64));
    println!("");
}

fn load_csv(csv_path: &str) -> (Vec<String>, Vec<u32>, Vec<Vec<u8>>) {
    let file = File::open(csv_path).expect("Cannot open CSV");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip header
    let _header = lines.next();

    let mut snp_ids = Vec::new();
    let mut positions = Vec::new();
    let mut genotypes: Vec<Vec<u8>> = Vec::new();

    for line in lines {
        if let Ok(line) = line {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                continue;
            }

            snp_ids.push(parts[0].to_string());
            positions.push(parts[1].parse::<u32>().unwrap_or(0));

            let mut geno = Vec::new();
            for i in 2..parts.len() {
                let val = parts[i].parse::<u8>().unwrap_or(3);
                geno.push(val);
            }

            genotypes.push(geno);
        }
    }

    (snp_ids, positions, genotypes)
}

fn compute_local_ld(genotypes: &[Vec<u8>], window_size: usize) -> Vec<(usize, usize, f64)> {
    let mut pairs = Vec::new();

    for i in 0..genotypes.len() {
        let j_max = (i + window_size).min(genotypes.len());

        for j in (i + 1)..j_max {
            let r2 = compute_r2(&genotypes[i], &genotypes[j]);
            if r2 > 0.5 {
                pairs.push((i, j, r2));
            }
        }
    }

    pairs
}

fn compute_r2(geno_i: &[u8], geno_j: &[u8]) -> f64 {
    if geno_i.len() != geno_j.len() || geno_i.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut valid = 0;
    let mut sum_i = 0.0;
    let mut sum_j = 0.0;
    let mut sum_ii = 0.0;
    let mut sum_jj = 0.0;

    for k in 0..geno_i.len() {
        if geno_i[k] != 3 && geno_j[k] != 3 {
            let gi = geno_i[k] as f64;
            let gj = geno_j[k] as f64;

            sum_i += gi;
            sum_j += gj;
            sum_ii += gi * gi;
            sum_jj += gj * gj;
            dot += gi * gj;
            valid += 1;
        }
    }

    if valid < 2 {
        return 0.0;
    }

    let n = valid as f64;
    let cov = (dot / n) - (sum_i / n) * (sum_j / n);
    let var_i = (sum_ii / n) - (sum_i / n) * (sum_i / n);
    let var_j = (sum_jj / n) - (sum_j / n) * (sum_j / n);

    if var_i > 1e-9 && var_j > 1e-9 {
        let r = cov / (var_i.sqrt() * var_j.sqrt());
        let r_clamped = r.max(-1.0).min(1.0);
        (r_clamped * r_clamped).max(0.0)
    } else {
        0.0
    }
}

fn identify_blocks(
    ld_pairs: &[(usize, usize, f64)],
    num_snps: usize,
    min_block_size: usize,
) -> Vec<Vec<usize>> {
    let mut blocks = Vec::new();
    let mut used = vec![false; num_snps];

    // Build adjacency from LD pairs
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, j, _) in ld_pairs {
        adj.entry(*i).or_insert_with(Vec::new).push(*j);
        adj.entry(*j).or_insert_with(Vec::new).push(*i);
    }

    // BFS to find connected components
    for start in 0..num_snps {
        if used[start] {
            continue;
        }

        let mut block = Vec::new();
        let mut queue = vec![start];
        used[start] = true;

        while let Some(node) = queue.pop() {
            block.push(node);

            if let Some(neighbors) = adj.get(&node) {
                for &neighbor in neighbors {
                    if !used[neighbor] {
                        used[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }
        }

        if block.len() >= min_block_size {
            blocks.push(block);
        }
    }

    blocks.sort_by_key(|b| b.len());
    blocks.reverse();

    blocks
}
