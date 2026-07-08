//! Ad-hoc timing comparison between `matmul_scalar` and `matmul_fast`.
//!
//! Not a rigorous benchmark harness (no `criterion` dependency added
//! for this) -- run via `cargo run --release --example bench_matmul`
//! (also wired into CI, see .github/workflows/ci.yml) so there's a
//! real, reproducible number in the CI log rather than an assumed one.
//! See docs/EXPERIMENTS.md for the actual measured result.

use std::time::Instant;

use ntg_kernel::{encode, matmul_fast, matmul_scalar};

fn main() {
    let (m, k, n) = (64usize, 512usize, 64usize);
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 61) % 3) as f32 - 1.0
    };
    let a_f32: Vec<f32> = (0..m * k).map(|_| next()).collect();
    let b_f32: Vec<f32> = (0..k * n).map(|_| next()).collect();
    let a = encode(&a_f32);
    let b = encode(&b_f32);

    let iters = 200u32;

    let start = Instant::now();
    for _ in 0..iters {
        let _ = matmul_scalar(&a, &b, m, k, n).unwrap();
    }
    let scalar_elapsed = start.elapsed();

    let start = Instant::now();
    for _ in 0..iters {
        let _ = matmul_fast(&a, &b, m, k, n).unwrap();
    }
    let fast_elapsed = start.elapsed();

    println!("shape: {m}x{k} @ {k}x{n}, iters: {iters}");
    println!("matmul_scalar: {:?} total, {:?}/iter", scalar_elapsed, scalar_elapsed / iters);
    println!("matmul_fast:   {:?} total, {:?}/iter", fast_elapsed, fast_elapsed / iters);
    let ratio = scalar_elapsed.as_secs_f64() / fast_elapsed.as_secs_f64();
    println!("speedup ratio (scalar/fast): {ratio:.3}x");

    // Correctness, not just speed -- if these ever disagree, something
    // is wrong with matmul_fast, not just "slow."
    let scalar_out = matmul_scalar(&a, &b, m, k, n).unwrap();
    let fast_out = matmul_fast(&a, &b, m, k, n).unwrap();
    assert_eq!(scalar_out, fast_out, "matmul_fast must match matmul_scalar bit-for-bit");
    println!("correctness: outputs are bit-identical, as required");
}
