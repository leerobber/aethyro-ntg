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

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn ternary(&mut self) -> i8 {
        (self.next() % 3) as i8 - 1
    }
    fn act(&mut self) -> i8 {
        ((self.next() % 255) as i32 - 127) as i8
    }
}

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

#[cfg(target_arch = "x86_64")]
mod dense {
    use std::arch::x86_64::*;

    pub fn available() -> bool {
        is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512bw")
            && is_x86_feature_detected!("avx512vnni")
    }

    /// Dense int8 control: same loop as the ternary VNNI kernel but the weight codes are
    /// already one byte each, so there is nothing to unpack and 4x the weight traffic.
    ///
    /// # Safety
    /// CPU must support AVX-512 F/BW/VNNI; `w` is `rows x k_padded`, `x` is
    /// `batch x k_padded`, `out` is `batch x rows`.
    #[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
    pub unsafe fn gemm(w: &[u8], rows: usize, k_padded: usize, x: &[i8], sums: &[i32], batch: usize, out: &mut [i32]) {
        let mut b0 = 0;
        while b0 < batch {
            let nb = (batch - b0).min(4);
            for n in 0..rows {
                let wrow = w.as_ptr().add(n * k_padded);
                let mut acc = [[_mm512_setzero_si512(); 4]; 4];
                let mut k = 0;
                while k < k_padded {
                    let w0 = _mm512_loadu_si512(wrow.add(k) as *const _);
                    let w1 = _mm512_loadu_si512(wrow.add(k + 64) as *const _);
                    let w2 = _mm512_loadu_si512(wrow.add(k + 128) as *const _);
                    let w3 = _mm512_loadu_si512(wrow.add(k + 192) as *const _);
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = x.as_ptr().add((b0 + bi) * k_padded + k);
                        a[0] = _mm512_dpbusd_epi32(a[0], w0, _mm512_loadu_si512(xp as *const _));
                        a[1] = _mm512_dpbusd_epi32(a[1], w1, _mm512_loadu_si512(xp.add(64) as *const _));
                        a[2] = _mm512_dpbusd_epi32(a[2], w2, _mm512_loadu_si512(xp.add(128) as *const _));
                        a[3] = _mm512_dpbusd_epi32(a[3], w3, _mm512_loadu_si512(xp.add(192) as *const _));
                    }
                    k += 256;
                }
                for bi in 0..nb {
                    let a = &acc[bi];
                    let s = _mm512_add_epi32(_mm512_add_epi32(a[0], a[1]), _mm512_add_epi32(a[2], a[3]));
                    out[(b0 + bi) * rows + n] = _mm512_reduce_add_epi32(s) - sums[b0 + bi];
                }
            }
            b0 += nb;
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod f32dot {
    use std::arch::x86_64::*;

    pub fn available() -> bool {
        is_x86_feature_detected!("avx512f")
    }

    /// `out[b][n] = dot(w[n], x[b])`, streaming each weight row once per group of 4 batch
    /// rows with four independent FMA accumulators per batch row.
    ///
    /// # Safety
    /// CPU must support AVX-512F; `w` is `rows x cols`, `x` is `batch x cols`,
    /// `out` is `batch x rows`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn gemm(w: &[f32], rows: usize, cols: usize, x: &[f32], batch: usize, out: &mut [f32]) {
        let mut b0 = 0;
        while b0 < batch {
            let nb = (batch - b0).min(4);
            for n in 0..rows {
                let wrow = w.as_ptr().add(n * cols);
                let mut acc = [[_mm512_setzero_ps(); 4]; 4];
                let mut k = 0;
                while k + 64 <= cols {
                    let w0 = _mm512_loadu_ps(wrow.add(k));
                    let w1 = _mm512_loadu_ps(wrow.add(k + 16));
                    let w2 = _mm512_loadu_ps(wrow.add(k + 32));
                    let w3 = _mm512_loadu_ps(wrow.add(k + 48));
                    for (bi, a) in acc.iter_mut().enumerate().take(nb) {
                        let xp = x.as_ptr().add((b0 + bi) * cols + k);
                        a[0] = _mm512_fmadd_ps(w0, _mm512_loadu_ps(xp), a[0]);
                        a[1] = _mm512_fmadd_ps(w1, _mm512_loadu_ps(xp.add(16)), a[1]);
                        a[2] = _mm512_fmadd_ps(w2, _mm512_loadu_ps(xp.add(32)), a[2]);
                        a[3] = _mm512_fmadd_ps(w3, _mm512_loadu_ps(xp.add(48)), a[3]);
                    }
                    k += 64;
                }
                for bi in 0..nb {
                    let a = &acc[bi];
                    let s = _mm512_add_ps(_mm512_add_ps(a[0], a[1]), _mm512_add_ps(a[2], a[3]));
                    let mut total = _mm512_reduce_add_ps(s);
                    for kk in k..cols {
                        total += *wrow.add(kk) * x[(b0 + bi) * cols + kk];
                    }
                    out[(b0 + bi) * rows + n] = total;
                }
            }
            b0 += nb;
        }
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
