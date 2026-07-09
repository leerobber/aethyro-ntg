//! Density micro-benchmark: scalar i8 · bit-sliced · sparse COO dots.
//!
//! Run:
//!   cargo run --release --bin density_bench
//!
//! Prints a markdown table + JSON line suitable for EXPERIMENTS.md.
//! Honest wall-clock (median of timed iterations). Not Criterion; no
//! extra deps — intentional for air-gapped / minimal CI hosts.

use ntg_kernel::ntg::storage::{BitSlicedTernary, SparseBitSlicedTernary};
use std::hint::black_box;
use std::time::Instant;

const N: usize = 262_144; // 256K ternary elements (4096 × 64)
const WARMUP: usize = 20;
const ITERS: usize = 200;

/// Deterministic PRNG (xorshift64) — no external rand crate.
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Build dense i8 vectors with approx `density` fraction of ±1 (rest 0).
fn make_i8_pair(n: usize, density: f32, seed: u64) -> (Vec<i8>, Vec<i8>) {
    let mut s = seed;
    let mut a = vec![0i8; n];
    let mut b = vec![0i8; n];
    let threshold = ((density as f64) * (u64::MAX as f64)) as u64;
    for i in 0..n {
        if xorshift(&mut s) < threshold {
            a[i] = if xorshift(&mut s) & 1 == 0 { 1 } else { -1 };
        }
        if xorshift(&mut s) < threshold {
            b[i] = if xorshift(&mut s) & 1 == 0 { 1 } else { -1 };
        }
    }
    (a, b)
}

fn scalar_dot(a: &[i8], b: &[i8]) -> i64 {
    let mut sum = 0i64;
    for i in 0..a.len() {
        sum += a[i] as i64 * b[i] as i64;
    }
    sum
}

fn median_ns(samples: &mut [u128]) -> f64 {
    samples.sort_unstable();
    samples[samples.len() / 2] as f64
}

fn time_ns<F: FnMut() -> i64>(mut f: F, warmup: usize, iters: usize) -> (f64, i64) {
    let mut last = 0i64;
    for _ in 0..warmup {
        last = black_box(f());
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        last = black_box(f());
        samples.push(t0.elapsed().as_nanos());
    }
    (median_ns(&mut samples), last)
}

struct Row {
    density: f32,
    scalar_us: f64,
    bit_sliced_us: f64,
    sparse_us: f64,
    sparse_blocks: usize,
    scalar_sum: i64,
    bit_sum: i64,
    sparse_sum: i64,
}

fn run_density(density: f32) -> Row {
    let (a_i8, b_i8) = make_i8_pair(N, density, 0xAE77_4000u64 ^ (density.to_bits() as u64));

    let mut a_bs = BitSlicedTernary::new(N);
    let mut b_bs = BitSlicedTernary::new(N);
    let mut a_sp = SparseBitSlicedTernary::new(N);
    let mut b_sp = SparseBitSlicedTernary::new(N);
    for i in 0..N {
        if a_i8[i] != 0 {
            a_bs.set(i, a_i8[i]);
            a_sp.set(i, a_i8[i]);
        }
        if b_i8[i] != 0 {
            b_bs.set(i, b_i8[i]);
            b_sp.set(i, b_i8[i]);
        }
    }
    a_bs.compute_density();
    a_sp.compute_density();

    let (scalar_ns, scalar_sum) = time_ns(|| scalar_dot(&a_i8, &b_i8), WARMUP, ITERS);
    let (bit_ns, bit_sum) = time_ns(
        || BitSlicedTernary::dot_product_parallel(&a_bs, &b_bs),
        WARMUP,
        ITERS,
    );
    let (sparse_ns, sparse_sum) = time_ns(
        || SparseBitSlicedTernary::dot_product_sparse(&a_sp, &b_sp),
        WARMUP,
        ITERS,
    );

    Row {
        density,
        scalar_us: scalar_ns / 1000.0,
        bit_sliced_us: bit_ns / 1000.0,
        sparse_us: sparse_ns / 1000.0,
        sparse_blocks: a_sp.blocks.len().max(b_sp.blocks.len()),
        scalar_sum,
        bit_sum,
        sparse_sum,
    }
}

fn main() {
    println!("# density_bench");
    println!("n={N} warmup={WARMUP} iters={ITERS} (median wall-clock)");
    println!();
    println!("| density | scalar µs | bit-sliced µs | sparse µs | speedup BS/S | speedup SP/S | blocks | sums match |");
    println!("|--------:|----------:|--------------:|----------:|-------------:|-------------:|-------:|:----------:|");

    let densities = [0.01f32, 0.10, 0.50];
    let mut rows = Vec::new();
    for &d in &densities {
        let row = run_density(d);
        let match_ok = row.scalar_sum == row.bit_sum && row.scalar_sum == row.sparse_sum;
        let sp_bs = row.scalar_us / row.bit_sliced_us.max(1e-9);
        let sp_sp = row.scalar_us / row.sparse_us.max(1e-9);
        println!(
            "| {:.0}% | {:.2} | {:.2} | {:.2} | {:.2}× | {:.2}× | {} | {} |",
            d * 100.0,
            row.scalar_us,
            row.bit_sliced_us,
            row.sparse_us,
            sp_bs,
            sp_sp,
            row.sparse_blocks,
            if match_ok { "yes" } else { "NO" }
        );
        rows.push((row, match_ok));
    }

    println!();
    println!("## JSON");
    print!("[");
    for (i, (r, ok)) in rows.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!(
            r#"{{"density":{},"n":{},"scalar_us":{:.4},"bit_sliced_us":{:.4},"sparse_us":{:.4},"sparse_blocks":{},"sums_match":{}}}"#,
            r.density,
            N,
            r.scalar_us,
            r.bit_sliced_us,
            r.sparse_us,
            r.sparse_blocks,
            ok
        );
    }
    println!("]");

    // Fail process if any correctness mismatch (bench is also a proof).
    if rows.iter().any(|(_, ok)| !*ok) {
        eprintln!("ERROR: dot-product sums diverged across paths");
        std::process::exit(1);
    }
}
