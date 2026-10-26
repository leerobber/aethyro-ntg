//! Ternary vs f32 head-to-head (Phase 6 kernel gate, redone fairly).
//!
//! The earlier `phase6_benchmark` ("143.6x") compared the bit-sliced kernel
//! against a *triple-loop scalar f32* GEMM -- the weakest possible opponent --
//! and fed the two sides different values (continuous f32 vs quantised). This
//! benchmark fixes both problems:
//!
//!   * Part A (throughput): C = A * B where A and B hold exact ternary values
//!     {-1,0,+1}. Every method computes the *identical* integer result, which
//!     is checked bit-for-bit, so speed is compared at equal output.
//!     Opponents: the old naive f32 loop (to reproduce the old regime), a
//!     cache-friendly f32 loop, and `matrixmultiply::sgemm` (SIMD, blocked, FMA
//!     -- what `ndarray` uses). All methods are single-threaded.
//!   * Part B (fidelity): what ternary quantisation costs. Gaussian weights are
//!     quantised with the kernel's own `encode` (absmean threshold) and the
//!     output is compared to the f32 result. Post-training quantisation of
//!     random weights is a pessimistic case; quantisation-aware training
//!     (BitNet) is not measured here.
//!
//! Not measured: end-to-end model quality, multi-threading, GPU, and the
//! aethyro.com comparison.
//!
//! Run:   cargo bench --bench ternary_vs_f32
//! Quick: cargo bench --bench ternary_vs_f32 -- --check     (small, correctness only)

use matrixmultiply::sgemm;
use ntg_kernel::{encode, BitSlicedTernary};
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
    fn unit(&mut self) -> f32 {
        ((self.next() >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }
    fn gauss(&mut self) -> f32 {
        let (u1, u2) = (self.unit(), self.unit());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
    }
    fn ternary(&mut self) -> i8 {
        (self.next() % 3) as i8 - 1
    }
}

fn median_ms(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn time_ms<F: FnMut()>(warmup: usize, reps: usize, mut f: F) -> f64 {
    for _ in 0..warmup {
        f();
    }
    let mut s = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        f();
        s.push(t.elapsed().as_secs_f64() * 1e3);
    }
    median_ms(s)
}

/// The old baseline: i-j-k with a strided walk down B's columns.
fn gemm_f32_naive(a: &[f32], b: &[f32], c: &mut [f32], m: usize, k: usize, n: usize) {
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0.0f32;
            for p in 0..k {
                acc += a[i * k + p] * b[p * n + j];
            }
            c[i * n + j] = acc;
        }
    }
}

/// i-k-j order: unit-stride inner loop the compiler can vectorise.
fn gemm_f32_loop(a: &[f32], b: &[f32], c: &mut [f32], m: usize, k: usize, n: usize) {
    c.fill(0.0);
    for i in 0..m {
        for p in 0..k {
            let av = a[i * k + p];
            let crow = &mut c[i * n..(i + 1) * n];
            let brow = &b[p * n..(p + 1) * n];
            for j in 0..n {
                crow[j] += av * brow[j];
            }
        }
    }
}

fn gemm_f32_sgemm(a: &[f32], b: &[f32], c: &mut [f32], m: usize, k: usize, n: usize) {
    // SAFETY: slices are m*k, k*n and m*n long and row-major with the strides given.
    unsafe {
        sgemm(
            m, k, n, 1.0, a.as_ptr(), k as isize, 1, b.as_ptr(), n as isize, 1, 0.0,
            c.as_mut_ptr(), n as isize, 1,
        );
    }
}

fn pack_cols(b: &[i8], k: usize, n: usize) -> Vec<BitSlicedTernary> {
    (0..n)
        .map(|j| {
            let col: Vec<i8> = (0..k).map(|p| b[p * n + j]).collect();
            BitSlicedTernary::from_slice(&col)
        })
        .collect()
}

fn gemm_ternary(a_rows: &[BitSlicedTernary], b_cols: &[BitSlicedTernary], c: &mut [i32]) {
    let n = b_cols.len();
    for (i, ar) in a_rows.iter().enumerate() {
        for (j, bc) in b_cols.iter().enumerate() {
            c[i * n + j] = BitSlicedTernary::dot_product_parallel(ar, bc) as i32;
        }
    }
}

struct Row {
    size: usize,
    naive: f64,
    looped: f64,
    sgemm: f64,
    tern: f64,
    tern_with_pack: f64,
    f32_bytes: usize,
    tern_bytes: usize,
}

fn part_a(size: usize, reps: usize, check_only: bool) -> Row {
    let (m, k, n) = (size, size, size);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ size as u64);
    let a_t: Vec<i8> = (0..m * k).map(|_| rng.ternary()).collect();
    let b_t: Vec<i8> = (0..k * n).map(|_| rng.ternary()).collect();
    let a: Vec<f32> = a_t.iter().map(|&v| v as f32).collect();
    let b: Vec<f32> = b_t.iter().map(|&v| v as f32).collect();

    let a_rows: Vec<BitSlicedTernary> = (0..m)
        .map(|i| BitSlicedTernary::from_slice(&a_t[i * k..(i + 1) * k]))
        .collect();
    let b_cols = pack_cols(&b_t, k, n);

    // Correctness gate: every method must produce the same exact integers.
    let mut c_naive = vec![0.0f32; m * n];
    let mut c_loop = vec![0.0f32; m * n];
    let mut c_sgemm = vec![0.0f32; m * n];
    let mut c_tern = vec![0i32; m * n];
    gemm_f32_naive(&a, &b, &mut c_naive, m, k, n);
    gemm_f32_loop(&a, &b, &mut c_loop, m, k, n);
    gemm_f32_sgemm(&a, &b, &mut c_sgemm, m, k, n);
    gemm_ternary(&a_rows, &b_cols, &mut c_tern);
    for idx in 0..m * n {
        let t = c_tern[idx];
        assert!(
            c_naive[idx] == t as f32 && c_loop[idx] == t as f32 && c_sgemm[idx] == t as f32,
            "output mismatch at {idx} (size {size}): ternary {t}, naive {}, loop {}, sgemm {}",
            c_naive[idx], c_loop[idx], c_sgemm[idx]
        );
    }

    if check_only {
        return Row { size, naive: 0.0, looped: 0.0, sgemm: 0.0, tern: 0.0, tern_with_pack: 0.0,
            f32_bytes: 0, tern_bytes: 0 };
    }

    let naive_reps = if size >= 1024 { 1 } else { reps };
    let naive = time_ms(0, naive_reps, || {
        gemm_f32_naive(black_box(&a), black_box(&b), &mut c_naive, m, k, n);
        black_box(&c_naive);
    });
    let looped = time_ms(1, reps, || {
        gemm_f32_loop(black_box(&a), black_box(&b), &mut c_loop, m, k, n);
        black_box(&c_loop);
    });
    let sgemm_ms = time_ms(1, reps, || {
        gemm_f32_sgemm(black_box(&a), black_box(&b), &mut c_sgemm, m, k, n);
        black_box(&c_sgemm);
    });
    let tern = time_ms(1, reps, || {
        gemm_ternary(black_box(&a_rows), black_box(&b_cols), &mut c_tern);
        black_box(&c_tern);
    });
    // Activations arrive as dense values, so B must be packed on every call.
    let tern_with_pack = time_ms(1, reps, || {
        let cols = pack_cols(black_box(&b_t), k, n);
        gemm_ternary(&a_rows, &cols, &mut c_tern);
        black_box(&c_tern);
    });

    let words = k.div_ceil(64);
    Row {
        size, naive, looped, sgemm: sgemm_ms, tern, tern_with_pack,
        f32_bytes: (m * k + k * n) * 4,
        tern_bytes: (m + n) * 2 * words * 8,
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let (mut dot, mut na, mut nb) = (0f64, 0f64, 0f64);
    for (x, y) in a.iter().zip(b) {
        dot += (*x as f64) * (*y as f64);
        na += (*x as f64).powi(2);
        nb += (*y as f64).powi(2);
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn rel_rmse(reference: &[f32], approx: &[f32]) -> f64 {
    let (mut err, mut norm) = (0f64, 0f64);
    for (r, a) in reference.iter().zip(approx) {
        err += ((*r - *a) as f64).powi(2);
        norm += (*r as f64).powi(2);
    }
    (err / norm).sqrt()
}

/// Least-squares scale so that scale * q approximates w.
fn ls_scale(w: &[f32], q: &[i8]) -> f32 {
    let num: f32 = w.iter().zip(q).map(|(w, &q)| w * q as f32).sum();
    let den: f32 = q.iter().map(|&q| (q as f32) * (q as f32)).sum();
    if den == 0.0 { 0.0 } else { num / den }
}

/// Returns (cos W_t*x_f32, relRMSE, cos W_t*x_t, relRMSE) averaged over trials.
fn part_b(rows: usize, cols: usize, trials: usize) -> (f64, f64, f64, f64) {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let (mut c1, mut e1, mut c2, mut e2) = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..trials {
        let w: Vec<f32> = (0..rows * cols).map(|_| rng.gauss()).collect();
        let x: Vec<f32> = (0..cols).map(|_| rng.gauss()).collect();
        let wq = encode(&w);
        let ws = ls_scale(&w, &wq);
        let xq = encode(&x);
        let xs = ls_scale(&x, &xq);

        let mut y_ref = vec![0.0f32; rows];
        let mut y_wt = vec![0.0f32; rows];
        let mut y_wt_xt = vec![0.0f32; rows];
        for r in 0..rows {
            let (mut a, mut b, mut c) = (0f32, 0f32, 0f32);
            for p in 0..cols {
                a += w[r * cols + p] * x[p];
                b += wq[r * cols + p] as f32 * x[p];
                c += wq[r * cols + p] as f32 * xq[p] as f32;
            }
            y_ref[r] = a;
            y_wt[r] = ws * b;
            y_wt_xt[r] = ws * xs * c;
        }
        c1 += cosine(&y_ref, &y_wt);
        e1 += rel_rmse(&y_ref, &y_wt);
        c2 += cosine(&y_ref, &y_wt_xt);
        e2 += rel_rmse(&y_ref, &y_wt_xt);
    }
    let t = trials as f64;
    (c1 / t, e1 / t, c2 / t, e2 / t)
}

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    let sizes: &[usize] = if check_only { &[64, 130] } else { &[128, 256, 512, 1024] };
    let reps = 7;

    println!("# ternary_vs_f32{}", if check_only { " (--check)" } else { "" });
    println!("single-threaded; median of {reps} runs; all methods verified to give identical exact outputs");
    println!("build: debug_assertions={}  popcnt_enabled={}  avx2_enabled={}",
        cfg!(debug_assertions),
        cfg!(target_feature = "popcnt"),
        cfg!(target_feature = "avx2"));
    println!();

    let rows: Vec<Row> = sizes.iter().map(|&s| part_a(s, reps, check_only)).collect();

    if !check_only {
        println!("## Part A: C = A*B, A and B exactly ternary (square N)");
        println!();
        println!("| N | f32 naive ms | f32 loop ms | f32 sgemm ms | ternary ms | ternary+pack ms | vs naive | vs loop | vs sgemm | vs sgemm (+pack) | bytes f32 | bytes ternary |");
        println!("|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|");
        for r in &rows {
            println!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1}x | {:.1}x | {:.2}x | {:.2}x | {} | {} |",
                r.size, r.naive, r.looped, r.sgemm, r.tern, r.tern_with_pack,
                r.naive / r.tern, r.looped / r.tern, r.sgemm / r.tern, r.sgemm / r.tern_with_pack,
                r.f32_bytes, r.tern_bytes
            );
        }
        println!();
        println!("Speedup columns are f32-time / ternary-time: above 1.0x means ternary is faster, below 1.0x means ternary is slower.");
        println!();
    }

    let (rows_n, cols_n, trials) = if check_only { (32, 128, 3) } else { (256, 1024, 20) };
    let (c1, e1, c2, e2) = part_b(rows_n, cols_n, trials);
    println!("## Part B: fidelity of ternary quantisation (Gaussian W {rows_n}x{cols_n}, x {cols_n}; mean of {trials} draws)");
    println!();
    println!("| computation | cosine vs f32 | relative RMSE |");
    println!("|---|--:|--:|");
    println!("| ternary W, f32 x (BitNet-style) | {c1:.4} | {e1:.4} |");
    println!("| ternary W, ternary x (what this kernel computes) | {c2:.4} | {e2:.4} |");

    if check_only {
        assert!(c1 > 0.7 && c2 > 0.5, "fidelity sanity bounds violated: {c1} {c2}");
        println!("\nternary_vs_f32 --check: OK");
    }
}
