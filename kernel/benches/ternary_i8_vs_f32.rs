//! Ternary-weight x int8-activation GEMM vs f32 sgemm, at model-like shapes.
//!
//! The earlier `ternary_vs_f32` bench (ternary x ternary through a popcount) lost to
//! SIMD f32. This one measures the kernel that is meant to win: 2-bit weights
//! unpacked in-register and multiplied with int8 activations (`vpdpbusd` /
//! `vpmaddubsw`), see `ntg::storage::ternary_i8_gemm`.
//!
//! What it controls for
//!   * Equal output: all methods compute the identical integers (exact in f32 because
//!     every partial sum is an integer below 2^24 at these sizes) and are asserted equal
//!     before timing.
//!   * A fair opponent, best of two f32 implementations per cell: `matrixmultiply::sgemm`
//!     (SIMD, blocked, FMA; it repacks the weight matrix on every call, which is wasteful
//!     for small batches) and a direct AVX-512 FMA row-dot kernel that streams the weights
//!     once per group of 4 batch rows (the right shape for matrix-vector work).
//!   * A dense int8 control (1 byte/weight, same VNNI loop, no unpacking), so "int8 is
//!     fast" is separated from "2-bit packing pays off".
//!   * Weights that do not fit in cache: each shape cycles through `L` distinct layers,
//!     so a forward pass streams the model's weights as real inference does.
//!   * Single thread for every method. Real inference is multi-threaded; ratios in the
//!     compute-bound rows should carry over, memory-bound rows will saturate sooner.
//!
//! Not measured: end-to-end model latency, GPU, accuracy (see the trained-model test).
//!
//! Run:   cargo bench --bench ternary_i8_vs_f32
//! Quick: cargo bench --bench ternary_i8_vs_f32 -- --check   (tiny shapes, equality only)

use matrixmultiply::sgemm;
use ntg_kernel::ntg::storage::{Int8Activations, PackedTernaryMatrix, TernaryI8Kernel};
use std::hint::black_box;
use std::time::Instant;

#[path = "common/baselines.rs"]
mod baselines;
#[cfg(target_arch = "x86_64")]
use baselines::{dense, f32dot};
use baselines::Rng;

/// One layer in every format under test.
struct Layer {
    f32_w: Vec<f32>,        // rows x cols
    dense: Vec<u8>,         // rows x k_padded, code = w + 1
    packed: PackedTernaryMatrix,
}

fn make_layer(rows: usize, cols: usize, rng: &mut Rng) -> Layer {
    let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
    let k_padded = cols.div_ceil(256) * 256;
    let mut dense = vec![1u8; rows * k_padded];
    for r in 0..rows {
        for c in 0..cols {
            dense[r * k_padded + c] = (w[r * cols + c] + 1) as u8;
        }
    }
    Layer {
        f32_w: w.iter().map(|&v| v as f32).collect(),
        dense,
        packed: PackedTernaryMatrix::from_i8(&w, rows, cols).expect("ternary by construction"),
    }
}

/// y[b][n] = sum_k W[n][k] * x[b][k] with f32 sgemm (x given as f32 of the int8 values).
fn f32_forward(layer: &Layer, x_f32: &[f32], batch: usize, rows: usize, cols: usize, out: &mut [f32]) {
    // C (batch x rows) = X (batch x cols) * W^T (cols x rows)
    // SAFETY: slice lengths are batch*cols, rows*cols and batch*rows; strides match the layouts.
    unsafe {
        sgemm(
            batch, cols, rows, 1.0, x_f32.as_ptr(), cols as isize, 1,
            layer.f32_w.as_ptr(), 1, cols as isize, 0.0, out.as_mut_ptr(), rows as isize, 1,
        );
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Median wall time (µs) per layer for one full pass over `layers`.
fn time_pass<F: FnMut(usize)>(layers: usize, reps: usize, mut f: F) -> f64 {
    for l in 0..layers {
        f(l); // warm up (also touches every layer once)
    }
    let mut samples = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        for l in 0..layers {
            f(l);
        }
        samples.push(t.elapsed().as_secs_f64() * 1e6 / layers as f64);
    }
    median(samples)
}

struct Config {
    k: usize,
    n: usize,
    layers: usize,
    batches: &'static [usize],
}

fn run(cfg: &Config, check_only: bool) {
    let (k, n, layers) = (cfg.k, cfg.n, cfg.layers);
    let mut rng = Rng(0x1234_5678_9ABC_DEF1 ^ (k as u64) << 16 ^ n as u64);
    let model: Vec<Layer> = (0..layers).map(|_| make_layer(n, k, &mut rng)).collect();

    let f32_mb = (n * k * 4 * layers) as f64 / 1e6;
    let dense_mb = model[0].dense.len() as f64 * layers as f64 / 1e6;
    let packed_mb = model[0].packed.packed_bytes() as f64 * layers as f64 / 1e6;
    let kernels = TernaryI8Kernel::all_available();

    if !check_only {
        println!("### K={k}, N={n}, {layers} layers  (weights: f32 {f32_mb:.0} MB, int8 {dense_mb:.0} MB, ternary {packed_mb:.0} MB)");
        println!();
        println!("| batch | f32 sgemm µs | f32 row-dot µs | int8 dense µs | ternary µs | ternary+quant µs | vs best f32 | vs best f32 (+quant) | vs int8 dense | kernel |");
        println!("|--:|--:|--:|--:|--:|--:|--:|--:|--:|---|");
    }

    for &batch in cfg.batches {
        let x_i8: Vec<i8> = (0..batch * k).map(|_| rng.act()).collect();
        let x_f32: Vec<f32> = x_i8.iter().map(|&v| v as f32).collect();
        let acts = Int8Activations::from_i8(&x_i8, batch, k).unwrap();

        // ---- correctness gate: every method gives the same integers on layer 0.
        let mut ref_f32 = vec![0f32; batch * n];
        f32_forward(&model[0], &x_f32, batch, n, k, &mut ref_f32);
        let want: Vec<i32> = ref_f32.iter().map(|&v| v as i32).collect();
        assert!(ref_f32.iter().zip(&want).all(|(&f, &i)| f == i as f32), "f32 result not integral");
        for &kernel in &kernels {
            let mut got = vec![0i32; batch * n];
            model[0].packed.gemm_i8(&acts, &mut got, kernel).unwrap();
            assert_eq!(got, want, "{} disagrees with f32 sgemm (K={k} N={n} B={batch})", kernel.name());
        }
        #[cfg(target_arch = "x86_64")]
        if f32dot::available() {
            let mut got = vec![0f32; batch * n];
            // SAFETY: availability checked; shapes built from the same dimensions.
            unsafe { f32dot::gemm(&model[0].f32_w, n, k, &x_f32, batch, &mut got) };
            assert_eq!(got, ref_f32, "f32 row-dot disagrees with sgemm (K={k} N={n} B={batch})");
        }
        #[cfg(target_arch = "x86_64")]
        if dense::available() {
            let mut got = vec![0i32; batch * n];
            // SAFETY: availability checked; shapes built from the same dimensions.
            unsafe {
                dense::gemm(&model[0].dense, n, acts.k_padded(), acts.padded(), acts.row_sums(), batch, &mut got)
            };
            assert_eq!(got, want, "dense int8 control disagrees (K={k} N={n} B={batch})");
        }
        if check_only {
            continue;
        }

        // ---- timing
        let reps = if batch >= 64 { 3 } else { 5 };
        let mut out_f = vec![0f32; batch * n];
        let t_f32 = time_pass(layers, reps, |l| {
            f32_forward(&model[l], black_box(&x_f32), batch, n, k, &mut out_f);
            black_box(&out_f);
        });

        #[cfg(target_arch = "x86_64")]
        let t_rowdot = if f32dot::available() {
            let mut out = vec![0f32; batch * n];
            Some(time_pass(layers, reps, |l| {
                // SAFETY: availability checked above.
                unsafe { f32dot::gemm(&model[l].f32_w, n, k, black_box(&x_f32), batch, &mut out) };
                black_box(&out);
            }))
        } else {
            None
        };
        #[cfg(not(target_arch = "x86_64"))]
        let t_rowdot: Option<f64> = None;
        let t_f32_best = t_rowdot.map_or(t_f32, |r| r.min(t_f32));

        #[cfg(target_arch = "x86_64")]
        let t_dense = if dense::available() {
            let mut out = vec![0i32; batch * n];
            Some(time_pass(layers, reps, |l| {
                // SAFETY: availability checked above.
                unsafe {
                    dense::gemm(&model[l].dense, n, acts.k_padded(), acts.padded(), acts.row_sums(), batch, &mut out)
                };
                black_box(&out);
            }))
        } else {
            None
        };
        #[cfg(not(target_arch = "x86_64"))]
        let t_dense: Option<f64> = None;

        let mut best: Option<(TernaryI8Kernel, f64)> = None;
        let mut out_i = vec![0i32; batch * n];
        for &kernel in &kernels {
            if kernel == TernaryI8Kernel::Scalar && (n * k * batch) > 50_000_000 {
                continue; // the scalar reference is only a correctness oracle at this size
            }
            let t = time_pass(layers, reps, |l| {
                model[l].packed.gemm_i8(black_box(&acts), &mut out_i, kernel).unwrap();
                black_box(&out_i);
            });
            if best.is_none_or(|(_, bt)| t < bt) {
                best = Some((kernel, t));
            }
        }
        let (best_kernel, t_tern) = best.expect("scalar always runs for small shapes");

        // Dynamic quantisation of f32 activations + dequantised output, per layer.
        let mut out_deq = vec![0f32; batch * n];
        let t_quant = time_pass(layers, reps, |l| {
            let q = Int8Activations::quantize_f32(black_box(&x_f32), batch, k).unwrap();
            model[l].packed.forward_f32(&q, &mut out_deq, best_kernel).unwrap();
            black_box(&out_deq);
        });

        let vs_dense = t_dense.map_or("n/a".to_string(), |d| format!("{:.2}x", d / t_tern));
        let dense_cell = t_dense.map_or("n/a".to_string(), |d| format!("{d:.0}"));
        let rowdot_cell = t_rowdot.map_or("n/a".to_string(), |r| format!("{r:.0}"));
        println!(
            "| {batch} | {t_f32:.0} | {rowdot_cell} | {dense_cell} | {t_tern:.0} | {t_quant:.0} | {:.2}x | {:.2}x | {vs_dense} | {} |",
            t_f32_best / t_tern,
            t_f32_best / t_quant,
            best_kernel.name()
        );
    }
    if !check_only {
        println!();
    }
}

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    println!("# ternary_i8_vs_f32{}", if check_only { " (--check)" } else { "" });
    println!(
        "single-threaded; kernels available: {}; exact-equality gate before timing",
        TernaryI8Kernel::all_available().iter().map(|k| k.name()).collect::<Vec<_>>().join(", ")
    );
    println!("speedup columns are f32-or-int8 time / ternary time (above 1.0x = ternary faster)");
    println!();

    let configs: Vec<Config> = if check_only {
        vec![Config { k: 300, n: 70, layers: 2, batches: &[1, 3, 5, 8] }]
    } else {
        vec![
            Config { k: 1024, n: 1024, layers: 64, batches: &[1, 4, 16, 64] },
            Config { k: 4096, n: 4096, layers: 16, batches: &[1, 4, 16, 64] },
        ]
    };
    for cfg in &configs {
        run(cfg, check_only);
    }
    if check_only {
        println!("ternary_i8_vs_f32 --check: OK");
    }
}
