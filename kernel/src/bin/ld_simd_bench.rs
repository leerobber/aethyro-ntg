//! Honest before/after benchmark for the word-parallel LD r² path.
//!
//! Loads a real 1000-Genomes chromosome's genotypes once, then computes
//! genotypic r² for every SNP pair in the same sliding window two ways:
//!   * `scalar`  — the original per-sample `get()` loop (inlined here as
//!     the baseline; it now lives only as a test oracle in
//!     `ld_compute`).
//!   * `bitparallel` — `BitstreamGenotypes::pearson_r2_bitparallel`, the
//!     word-level popcount path now on the production hot path.
//!
//! Reports wall-clock for each over the identical pair set and asserts the
//! two produce the same r² (so the speedup is not bought with a wrong
//! answer). Numbers go in docs/EXPERIMENTS.md.
//!
//! Usage:
//!   cargo run --release --bin ld_simd_bench -- <vcf.gz> <chr> [max_variants] [window]

use ntg_kernel::genomic::bitsliced_genotypes::BitstreamGenotypes;
use ntg_kernel::genomic::vcf_stream::VcfParser;
use std::time::Instant;

/// Original scalar per-sample genotypic r² — the pre-change baseline.
fn r2_scalar(g1: &BitstreamGenotypes, g2: &BitstreamGenotypes) -> Option<f32> {
    if g1.len() != g2.len() {
        return None;
    }
    let n = g1.len();
    let (mut sx, mut sy, mut sxy, mut sx2, mut sy2, mut valid) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let a = g1.get(i);
        let b = g2.get(i);
        if a == 3 || b == 3 {
            continue;
        }
        let (x, y) = (a as f64, b as f64);
        sx += x;
        sy += y;
        sxy += x * y;
        sx2 += x * x;
        sy2 += y * y;
        valid += 1.0;
    }
    if valid < 10.0 {
        return None;
    }
    let num = valid * sxy - sx * sy;
    let den = ((valid * sx2 - sx * sx) * (valid * sy2 - sy * sy)).sqrt();
    if den <= 0.0 {
        return None;
    }
    let r = num / den;
    Some(((r * r) as f32).clamp(0.0, 1.0))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ld_simd_bench <vcf.gz> <chr> [max_variants=15000] [window=500]");
        std::process::exit(1);
    }
    let vcf = &args[1];
    let chr: u8 = args[2].parse().expect("chr must be a number");
    let max_variants: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15_000);
    let window: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(500);

    eprintln!("[*] Parsing {} variants from chr{} of {}", max_variants, chr, vcf);
    let parse_start = Instant::now();
    let chrom = VcfParser::new(false)
        .parse_vcf_limited(vcf, chr, Some(max_variants))
        .expect("VCF parse failed");
    let g = &chrom.genotypes;
    let n_snps = g.len();
    let n_samples = g.first().map(|s| s.len()).unwrap_or(0);
    eprintln!(
        "[*] Parsed {} SNPs x {} samples in {:.2}s",
        n_snps,
        n_samples,
        parse_start.elapsed().as_secs_f64()
    );

    // Build the exact sliding-window pair index set LdComputer uses.
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..n_snps {
        let end = (i + window).min(n_snps);
        for j in (i + 1)..end {
            pairs.push((i, j));
        }
    }
    eprintln!("[*] {} SNP pairs (window={})", pairs.len(), window);

    // --- scalar baseline (timed alone) ---
    let t = Instant::now();
    let mut acc_scalar = 0.0f64;
    let mut kept_scalar = 0u64;
    for &(i, j) in &pairs {
        if let Some(r2) = r2_scalar(&g[i], &g[j]) {
            acc_scalar += r2 as f64;
            kept_scalar += 1;
        }
    }
    let scalar_secs = t.elapsed().as_secs_f64();

    // --- bit-parallel path (timed alone; no oracle work in this loop) ---
    let t = Instant::now();
    let mut acc_bp = 0.0f64;
    let mut kept_bp = 0u64;
    for &(i, j) in &pairs {
        if let Some(r2) = g[i].pearson_r2_bitparallel(&g[j], 10) {
            acc_bp += r2 as f64;
            kept_bp += 1;
        }
    }
    let bp_secs = t.elapsed().as_secs_f64();

    // --- correctness cross-check (not timed) ---
    let mut max_abs_diff = 0.0f32;
    for &(i, j) in &pairs {
        let bp = g[i].pearson_r2_bitparallel(&g[j], 10);
        let sc = r2_scalar(&g[i], &g[j]);
        match (bp, sc) {
            (Some(a), Some(b)) => max_abs_diff = max_abs_diff.max((a - b).abs()),
            (None, None) => {}
            (a, b) => panic!("Some/None mismatch at ({i},{j}): bp={a:?} scalar={b:?}"),
        }
    }

    assert_eq!(kept_scalar, kept_bp, "kept-pair count diverged");
    assert!(
        max_abs_diff < 1e-5,
        "r² diverged beyond f32 tolerance: {}",
        max_abs_diff
    );

    let speedup = scalar_secs / bp_secs;
    println!("──────────────────────────────────────────────");
    println!("chr{chr}  SNPs={n_snps}  samples={n_samples}  pairs={}", pairs.len());
    println!("kept pairs (r² computable): {kept_bp}");
    println!("max |Δr²| scalar-vs-bitparallel: {max_abs_diff:.2e}  (identical within f32)");
    println!("mean r² (both paths): scalar={:.6} bitparallel={:.6}",
        acc_scalar / kept_scalar.max(1) as f64, acc_bp / kept_bp.max(1) as f64);
    println!("scalar      : {scalar_secs:.4}s");
    println!("bitparallel : {bp_secs:.4}s");
    println!("speedup     : {speedup:.2}x");
    println!("──────────────────────────────────────────────");
}
