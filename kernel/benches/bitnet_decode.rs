//! Per-token cost of BitNet b1.58 2B-4T's linear layers, at the model's real shapes.
//!
//! Each transformer layer has seven BitLinear matmuls (hidden 2560, FFN 6912, 20 query / 5
//! KV heads of width 128), 30 layers, 210 matrices, about 2.08 B weights:
//!
//!   q 2560->2560, k 2560->640, v 2560->640, o 2560->2560,
//!   gate 2560->6912, up 2560->6912, down 6912->2560
//!
//! This times only those matmuls (the part the kernel replaces), with random ternary
//! weights (dense kernels' speed does not depend on weight values). Real weights go through
//! `scripts/bitnet_eval.py`. To keep weights streaming from memory rather than cache, 6
//! distinct transformer layers are allocated and cycled; per-token time is scaled to 30
//! layers. Compared: f32 row-dot (the best f32 here), bf16 row-dot (`vdpbf16ps`, the
//! practical baseline for a model shipped in bf16), dense int8 VNNI (control), and the
//! ternary kernel, single-threaded and on a 4-thread rayon pool.
//!
//! Not included: attention, norms, embeddings, lm_head (a 128256x2560 bf16 matrix read per
//! token, which is not ternary), activation quantisation, Python overhead.
//!
//! Run:   cargo bench --bench bitnet_decode
//! Quick: cargo bench --bench bitnet_decode -- --check   (tiny odd shapes, equality only)

use ntg_kernel::ntg::storage::{Int8Activations, PackedTernaryMatrix, TernaryI8Kernel};
use rayon::prelude::*;
use std::hint::black_box;
use std::time::Instant;

#[path = "common/baselines.rs"]
mod baselines;
#[cfg(target_arch = "x86_64")]
use baselines::{bf16dot, dense, f32dot};
use baselines::Rng;

const MODEL_LAYERS: usize = 30;
/// (name, in_features, out_features)
const SHAPES: [(&str, usize, usize); 7] = [
    ("q", 2560, 2560),
    ("k", 2560, 640),
    ("v", 2560, 640),
    ("o", 2560, 2560),
    ("gate", 2560, 6912),
    ("up", 2560, 6912),
    ("down", 6912, 2560),
];

struct Lin {
    rows: usize,
    cols: usize,
    f32_w: Vec<f32>,
    bf16_w: Vec<u16>, // rows x padded(cols), zero padded
    dense: Vec<u8>,
    packed: PackedTernaryMatrix,
}

fn make(rows: usize, cols: usize, rng: &mut Rng) -> Lin {
    let w: Vec<i8> = (0..rows * cols).map(|_| rng.ternary()).collect();
    let k_padded = cols.div_ceil(256) * 256;
    let mut dense = vec![1u8; rows * k_padded];
    for r in 0..rows {
        for c in 0..cols {
            dense[r * k_padded + c] = (w[r * cols + c] + 1) as u8;
        }
    }
    let cols_p = bf16dot::padded(cols);
    let mut bf16_w = vec![0u16; rows * cols_p];
    for r in 0..rows {
        for c in 0..cols {
            bf16_w[r * cols_p + c] = bf16dot::to_bf16(w[r * cols + c] as f32);
        }
    }
    Lin {
        rows,
        cols,
        bf16_w,
        f32_w: w.iter().map(|&v| v as f32).collect(),
        dense,
        packed: PackedTernaryMatrix::from_i8(&w, rows, cols).expect("ternary by construction"),
    }
}

fn f32_run(l: &Lin, x: &[f32], batch: usize, out: &mut [f32], pool: Option<&rayon::ThreadPool>) {
    #[cfg(target_arch = "x86_64")]
    {
        match pool {
            Some(p) if batch == 1 => {
                let chunk = l.rows.div_ceil(p.current_num_threads()).next_multiple_of(8);
                p.install(|| {
                    out.par_chunks_mut(chunk).enumerate().for_each(|(i, o)| {
                        // SAFETY: AVX-512F checked by the caller; `o` is rows i*chunk.. of the output.
                        unsafe { f32dot::gemm(&l.f32_w[i * chunk * l.cols..], o.len(), l.cols, x, 1, o) }
                    });
                });
            }
            // SAFETY: AVX-512F checked by the caller; shapes come from `make`.
            _ => unsafe { f32dot::gemm(&l.f32_w, l.rows, l.cols, x, batch, out) },
        }
    }
}

fn bf16_run(l: &Lin, xb: &[u16], batch: usize, out: &mut [f32], pool: Option<&rayon::ThreadPool>) {
    #[cfg(target_arch = "x86_64")]
    {
        let cp = bf16dot::padded(l.cols);
        match pool {
            Some(p) if batch == 1 => {
                let chunk = l.rows.div_ceil(p.current_num_threads()).next_multiple_of(8);
                p.install(|| {
                    out.par_chunks_mut(chunk).enumerate().for_each(|(i, o)| {
                        // SAFETY: AVX512-BF16 checked by the caller; chunked rows of one output vector.
                        unsafe { bf16dot::gemm(&l.bf16_w[i * chunk * cp..], o.len(), cp, xb, 1, o) }
                    });
                });
            }
            // SAFETY: as above; shapes come from `make`.
            _ => unsafe { bf16dot::gemm(&l.bf16_w, l.rows, cp, xb, batch, out) },
        }
    }
}

fn dense_run(l: &Lin, acts: &Int8Activations, out: &mut [i32], pool: Option<&rayon::ThreadPool>) {
    #[cfg(target_arch = "x86_64")]
    {
        let kp = acts.k_padded();
        match pool {
            Some(p) if acts.batch() == 1 => {
                let chunk = l.rows.div_ceil(p.current_num_threads()).next_multiple_of(8);
                p.install(|| {
                    out.par_chunks_mut(chunk).enumerate().for_each(|(i, o)| {
                        // SAFETY: AVX-512 VNNI checked by the caller; chunked rows of one output vector.
                        unsafe { dense::gemm(&l.dense[i * chunk * kp..], o.len(), kp, acts.padded(), acts.row_sums(), 1, o) }
                    });
                });
            }
            // SAFETY: as above.
            _ => unsafe { dense::gemm(&l.dense, l.rows, kp, acts.padded(), acts.row_sums(), acts.batch(), out) },
        }
    }
}

fn ternary_run(l: &Lin, acts: &Int8Activations, out: &mut [i32], pool: Option<&rayon::ThreadPool>) {
    let k = TernaryI8Kernel::best_available();
    match pool {
        Some(p) => l.packed.gemm_i8_parallel(acts, out, k, p).unwrap(),
        None => l.packed.gemm_i8(acts, out, k).unwrap(),
    }
}

/// int8 activations as zero-padded bf16 rows (`batch x padded(cols)`); exact for |x| <= 256.
fn bf16_acts(xs: &[i8], batch: usize, cols: usize) -> Vec<u16> {
    #[cfg(target_arch = "x86_64")]
    {
        let cp = bf16dot::padded(cols);
        let mut out = vec![0u16; batch * cp];
        for b in 0..batch {
            for c in 0..cols {
                out[b * cp + c] = bf16dot::to_bf16(xs[b * cols + c] as f32);
            }
        }
        out
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (xs, batch, cols);
        Vec::new()
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Median wall time (ms) of one pass over every matrix in `model`.
fn time_pass<F: FnMut(&Lin)>(model: &[Lin], reps: usize, mut f: F) -> f64 {
    model.iter().for_each(&mut f);
    let mut samples = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        model.iter().for_each(&mut f);
        samples.push(t.elapsed().as_secs_f64() * 1e3);
    }
    median(samples)
}

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    let (div, alloc_layers) = if check_only { (8, 1) } else { (1, 6) };
    let mut rng = Rng(0xB17_0158);
    let model: Vec<Lin> = (0..alloc_layers)
        .flat_map(|_| SHAPES.iter().map(|&(_, i, o)| (i / div, o / div)).collect::<Vec<_>>())
        .map(|(i, o)| make(o, i, &mut rng))
        .collect();

    let pool1 = None;
    let pool4 = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let have_vnni = TernaryI8Kernel::Avx512Vnni.is_available();
    #[cfg(target_arch = "x86_64")]
    let have_f32 = f32dot::available();
    #[cfg(not(target_arch = "x86_64"))]
    let have_f32 = false;
    #[cfg(target_arch = "x86_64")]
    let have_bf16 = bf16dot::available();
    #[cfg(not(target_arch = "x86_64"))]
    let have_bf16 = false;

    let scale = MODEL_LAYERS as f64 / alloc_layers as f64;
    let params: usize = SHAPES.iter().map(|&(_, i, o)| i * o).sum::<usize>() * MODEL_LAYERS;
    let layer_f32: usize = model.iter().map(|l| l.f32_w.len() * 4).sum();
    let layer_packed: usize = model.iter().map(|l| l.packed.packed_bytes()).sum();

    println!("# bitnet_decode{}", if check_only { " (--check)" } else { "" });
    if !check_only {
        println!(
            "BitNet b1.58 2B-4T shapes: 7 matrices x {MODEL_LAYERS} layers = {:.2} B weights; allocated {alloc_layers} layers \
             (f32 {:.0} MB, ternary {:.0} MB) and scaled x{scale:.0} to {MODEL_LAYERS} layers",
            params as f64 / 1e9,
            layer_f32 as f64 / 1e6,
            layer_packed as f64 / 1e6,
        );
        println!("per-token weight traffic: f32 {:.2} GB, int8 {:.2} GB, ternary {:.0} MB",
            params as f64 * 4.0 / 1e9, params as f64 / 1e9, params as f64 / 4.0 / 1e6);
        println!();
    }

    for &batch in if check_only { &[1usize, 3][..] } else { &[1usize, 8][..] } {
        let mut xrng = Rng(0xFEED ^ batch as u64);
        let max_in = model.iter().map(|l| l.cols).max().unwrap();
        let x_i8: Vec<i8> = (0..batch * max_in).map(|_| xrng.act()).collect();

        // ---- correctness gate on every matrix: ternary == f32 row-dot == dense int8, 1T == 4T.
        for l in &model {
            let xs: Vec<i8> = (0..batch).flat_map(|b| x_i8[b * max_in..b * max_in + l.cols].to_vec()).collect();
            let acts = Int8Activations::from_i8(&xs, batch, l.cols).unwrap();
            let xf: Vec<f32> = xs.iter().map(|&v| v as f32).collect();
            let mut t1 = vec![0i32; batch * l.rows];
            let mut t4 = vec![0i32; batch * l.rows];
            ternary_run(l, &acts, &mut t1, pool1);
            l.packed.gemm_i8_parallel(&acts, &mut t4, TernaryI8Kernel::best_available(), &pool4).unwrap();
            assert_eq!(t1, t4, "parallel ternary differs ({}x{} B={batch})", l.rows, l.cols);
            if have_f32 {
                let mut f = vec![0f32; batch * l.rows];
                f32_run(l, &xf, batch, &mut f, pool1);
                assert!(f.iter().zip(&t1).all(|(&a, &b)| a == b as f32), "f32 differs ({}x{} B={batch})", l.rows, l.cols);
                if batch == 1 {
                    let mut f4 = vec![0f32; l.rows];
                    f32_run(l, &xf, 1, &mut f4, Some(&pool4));
                    assert_eq!(f, f4, "parallel f32 differs ({}x{})", l.rows, l.cols);
                }
            }
            if have_vnni {
                let mut d = vec![0i32; batch * l.rows];
                dense_run(l, &acts, &mut d, pool1);
                assert_eq!(d, t1, "dense int8 differs ({}x{} B={batch})", l.rows, l.cols);
            }
            if have_bf16 {
                let xb = bf16_acts(&xs, batch, l.cols);
                let mut f = vec![0f32; batch * l.rows];
                bf16_run(l, &xb, batch, &mut f, pool1);
                assert!(f.iter().zip(&t1).all(|(&a, &b)| a == b as f32), "bf16 differs ({}x{} B={batch})", l.rows, l.cols);
                if batch == 1 {
                    let mut f4 = vec![0f32; l.rows];
                    bf16_run(l, &xb, 1, &mut f4, Some(&pool4));
                    assert_eq!(f, f4, "parallel bf16 differs ({}x{})", l.rows, l.cols);
                }
            }
        }
        if check_only {
            continue;
        }

        // ---- timing: activations are prepared once; only the matmuls are inside the loop.
        let reps = 5;
        let inputs: Vec<(Int8Activations, Vec<f32>, Vec<u16>)> = model
            .iter()
            .map(|l| {
                let xs: Vec<i8> = (0..batch).flat_map(|b| x_i8[b * max_in..b * max_in + l.cols].to_vec()).collect();
                let xf = xs.iter().map(|&v| v as f32).collect();
                let xb = bf16_acts(&xs, batch, l.cols);
                (Int8Activations::from_i8(&xs, batch, l.cols).unwrap(), xf, xb)
            })
            .collect();
        let idx = |l: &Lin| model.iter().position(|m| std::ptr::eq(m, l)).unwrap();

        println!("### batch {batch}  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)");
        println!();
        println!("| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |");
        println!("|--:|---|--:|--:|--:|--:|--:|--:|");
        for (label, threads, pool) in [("1", 1, None), ("4", 4, Some(&pool4))] {
            if batch > 1 && threads > 1 {
                continue; // multi-threaded rows are decode (batch 1) only
            }
            let mut out_f = vec![0f32; batch * 6912];
            let mut out_i = vec![0i32; batch * 6912];
            let f32_ms = have_f32.then(|| {
                time_pass(&model, reps, |l| {
                    let (_, xf, _) = &inputs[idx(l)];
                    f32_run(l, black_box(xf), batch, &mut out_f[..batch * l.rows], pool);
                    black_box(&out_f);
                }) * scale
            });
            let bf16_ms = have_bf16.then(|| {
                time_pass(&model, reps, |l| {
                    let (_, _, xb) = &inputs[idx(l)];
                    bf16_run(l, black_box(xb), batch, &mut out_f[..batch * l.rows], pool);
                    black_box(&out_f);
                }) * scale
            });
            let dense_ms = have_vnni.then(|| {
                time_pass(&model, reps, |l| {
                    let (a, _, _) = &inputs[idx(l)];
                    dense_run(l, black_box(a), &mut out_i[..batch * l.rows], pool);
                    black_box(&out_i);
                }) * scale
            });
            let tern_ms = time_pass(&model, reps, |l| {
                let (a, _, _) = &inputs[idx(l)];
                ternary_run(l, black_box(a), &mut out_i[..batch * l.rows], pool);
                black_box(&out_i);
            }) * scale;

            let gbs = |bytes: f64, ms: f64| bytes / (ms / 1e3) / 1e9;
            let row = |name: &str, ms: f64, bytes: f64, vs: [Option<f64>; 3]| {
                let c = |v: Option<f64>| v.map_or("-".into(), |v| format!("{v:.1}x"));
                println!(
                    "| {label} | {name} | {ms:.1} | {:.1} | {:.1} | {} | {} | {} |",
                    1000.0 / ms, gbs(bytes, ms), c(vs[0]), c(vs[1]), c(vs[2])
                );
            };
            let p = params as f64;
            if let Some(ms) = f32_ms {
                row("f32 row-dot", ms, p * 4.0, [None, None, None]);
            }
            if let Some(ms) = bf16_ms {
                row("bf16 row-dot (vdpbf16ps)", ms, p * 2.0, [f32_ms.map(|f| f / ms), None, None]);
            }
            if let Some(ms) = dense_ms {
                row("int8 dense VNNI", ms, p, [f32_ms.map(|f| f / ms), bf16_ms.map(|b| b / ms), None]);
            }
            row(
                "ternary 2-bit (this kernel)",
                tern_ms,
                p / 4.0,
                [f32_ms.map(|f| f / tern_ms), bf16_ms.map(|b| b / tern_ms), dense_ms.map(|d| d / tern_ms)],
            );
            let _ = threads;
        }
        println!();
    }
    if check_only {
        println!("bitnet_decode --check: OK");
    }
}
