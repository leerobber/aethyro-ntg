//! Prefill (large-batch) cost of BitNet b1.58 2B-4T's linear layers at the model's real shapes.
//!
//! One transformer layer has seven BitLinear matrices (q, k, v, o, gate, up, down; 69.3 M weights);
//! the model has 30 layers. A prefill of `B` tokens is a `B`-row batch through each. This times one
//! layer's seven products at several batch sizes and reports the implied whole-model prefill rate
//! (`B / (30 x layer time)`, linears only: attention, norms, the output matrix and Python are not
//! included), plus the ceiling for this CPU: the rate of a register-only VNNI loop.
//!
//! Compared: the row-layout ternary kernel (the decode kernel), the panel-layout prefill kernel
//! (`PackedTernaryMatrix::prepare_prefill`), and a dense int8 VNNI control (`baselines::dense`,
//! one thread, four-batch-row blocking). The ceiling above is one core running nothing else.
//!
//! Run:   cargo bench --bench bitnet_prefill
//! Quick: cargo bench --bench bitnet_prefill -- --check   (tiny odd shapes, equality only)

use ntg_kernel::ntg::storage::{Int8Activations, PackedTernaryMatrix, SpinPool, TernaryI8Kernel};
use std::hint::black_box;
use std::time::Instant;

#[path = "common/baselines.rs"]
mod baselines;
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
    Lin { rows, cols, dense, packed: PackedTernaryMatrix::from_i8(&w, rows, cols).expect("ternary by construction") }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// MAC/s of a register-only VNNI loop: eight independent accumulators, no memory traffic.
#[cfg(target_arch = "x86_64")]
fn vnni_ceiling_gmacs() -> Option<f64> {
    if !TernaryI8Kernel::Avx512Vnni.is_available() {
        return None;
    }
    #[target_feature(enable = "avx512f,avx512bw,avx512vnni")]
    unsafe fn spin(iters: u64) -> i32 {
        use std::arch::x86_64::*;
        let a = _mm512_set1_epi8(1);
        let b = _mm512_set1_epi8(1);
        let mut acc = [_mm512_setzero_si512(); 8];
        for _ in 0..iters {
            for x in acc.iter_mut() {
                *x = _mm512_dpbusd_epi32(*x, a, b);
            }
        }
        let mut s = _mm512_setzero_si512();
        for x in acc {
            s = _mm512_add_epi32(s, x);
        }
        _mm512_reduce_add_epi32(s)
    }
    let iters = 20_000_000u64;
    let t = Instant::now();
    // SAFETY: the VNNI feature set was checked above.
    let r = unsafe { spin(iters) };
    let dt = t.elapsed().as_secs_f64();
    black_box(r);
    Some(iters as f64 * 8.0 * 64.0 / dt / 1e9)
}

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    let div = if check_only { 8 } else { 1 };
    let batches: &[usize] = if check_only { &[3, 9, 27] } else { &[8, 32, 128, 512] };
    let mut rng = Rng(0xB17_0159);
    let seed = rng.0;
    let layer: Vec<Lin> = SHAPES.iter().map(|&(_, i, o)| make(o / div, i / div, &mut rng)).collect();
    // The same weights again, with the panel copy built.
    let mut rng_p = Rng(seed);
    let mut layer_p: Vec<Lin> = SHAPES.iter().map(|&(_, i, o)| make(o / div, i / div, &mut rng_p)).collect();
    for l in &mut layer_p {
        l.packed.prepare_prefill();
    }
    let weights: usize = layer.iter().map(|l| l.rows * l.cols).sum();
    let kernel = TernaryI8Kernel::best_available();
    let pool4 = SpinPool::new(4).unwrap();

    println!("# bitnet_prefill{}", if check_only { " (--check)" } else { "" });
    println!("kernel: {}; one layer = {:.1} M weights; whole model = {MODEL_LAYERS} layers", kernel.name(), weights as f64 / 1e6);
    #[cfg(target_arch = "x86_64")]
    let ceiling = if check_only { None } else { vnni_ceiling_gmacs() };
    #[cfg(not(target_arch = "x86_64"))]
    let ceiling: Option<f64> = None;
    if let Some(c) = ceiling {
        println!("VNNI register-only ceiling, one core: {c:.0} GMAC/s");
    }
    println!();

    for &batch in batches {
        let max_in = layer.iter().map(|l| l.cols).max().unwrap();
        let x_i8: Vec<i8> = (0..batch * max_in).map(|_| rng.act()).collect();
        let acts: Vec<Int8Activations> = layer
            .iter()
            .map(|l| {
                let xs: Vec<i8> = (0..batch).flat_map(|b| x_i8[b * max_in..b * max_in + l.cols].to_vec()).collect();
                Int8Activations::from_i8(&xs, batch, l.cols).unwrap()
            })
            .collect();

        // Correctness: row layout == panel layout (1 and 4 threads) == dense int8 control on every matrix.
        for ((l, lp), a) in layer.iter().zip(&layer_p).zip(&acts) {
            let mut pn = vec![0i32; batch * l.rows];
            lp.packed.gemm_i8(a, &mut pn, kernel).unwrap();
            let mut pn4 = vec![0i32; batch * l.rows];
            lp.packed.gemm_i8_spin(a, &mut pn4, kernel, &pool4).unwrap();
            assert_eq!(pn, pn4, "panel 4-thread result differs ({}x{} B={batch})", l.rows, l.cols);
            let mut t1 = vec![0i32; batch * l.rows];
            let mut t4 = vec![0i32; batch * l.rows];
            l.packed.gemm_i8(a, &mut t1, kernel).unwrap();
            l.packed.gemm_i8_spin(a, &mut t4, kernel, &pool4).unwrap();
            assert_eq!(t1, t4, "4-thread result differs ({}x{} B={batch})", l.rows, l.cols);
            assert_eq!(t1, pn, "panel result differs from the row layout ({}x{} B={batch})", l.rows, l.cols);
            #[cfg(target_arch = "x86_64")]
            if baselines::dense::available() {
                let kp = l.cols.div_ceil(256) * 256;
                let mut d = vec![0i32; batch * l.rows];
                // SAFETY: VNNI availability checked; shapes come from `make` and `Int8Activations`.
                unsafe { baselines::dense::gemm(&l.dense, l.rows, kp, a.padded(), a.row_sums(), batch, &mut d) };
                assert_eq!(t1, d, "ternary differs from the dense int8 control ({}x{} B={batch})", l.rows, l.cols);
            }
        }
        if check_only {
            continue;
        }

        let reps = if batch >= 512 { 3 } else { 5 };
        let macs = weights as f64 * batch as f64;
        println!("### batch {batch}  (one layer's seven products; whole-model prefill = linears only)");
        println!();
        println!("| method | ms per layer | GMAC/s | % of 1-core ceiling | prefill tok/s (30 layers) |");
        println!("|---|--:|--:|--:|--:|");
        let mut out = vec![0i32; batch * 6912];
        let row = |name: &str, ms: f64, cores: usize| {
            let gmacs = macs / (ms / 1e3) / 1e9;
            let pct = ceiling.map_or("-".to_string(), |c| format!("{:.0}%", 100.0 * gmacs / (c * cores as f64)));
            println!("| {name} | {ms:.1} | {gmacs:.1} | {pct} | {:.0} |", batch as f64 / (MODEL_LAYERS as f64 * ms / 1e3));
        };
        let time = |f: &mut dyn FnMut()| {
            f();
            let mut s = Vec::new();
            for _ in 0..reps {
                let t = Instant::now();
                f();
                s.push(t.elapsed().as_secs_f64() * 1e3);
            }
            median(s)
        };
        #[cfg(target_arch = "x86_64")]
        if baselines::dense::available() {
            let ms = time(&mut || {
                for (l, a) in layer.iter().zip(&acts) {
                    let kp = l.cols.div_ceil(256) * 256;
                    // SAFETY: as above.
                    unsafe { baselines::dense::gemm(&l.dense, l.rows, kp, a.padded(), a.row_sums(), batch, &mut out[..batch * l.rows]) };
                    black_box(&out);
                }
            });
            row("int8 dense VNNI control, 1 thread", ms, 1);
        }
        let ms1 = time(&mut || {
            for (l, a) in layer.iter().zip(&acts) {
                l.packed.gemm_i8(a, &mut out[..batch * l.rows], kernel).unwrap();
                black_box(&out);
            }
        });
        row("ternary row layout, 1 thread", ms1, 1);
        let ms4 = time(&mut || {
            for (l, a) in layer.iter().zip(&acts) {
                l.packed.gemm_i8_spin(a, &mut out[..batch * l.rows], kernel, &pool4).unwrap();
                black_box(&out);
            }
        });
        row("ternary row layout, 4 threads", ms4, 4);
        let msp1 = time(&mut || {
            for (l, a) in layer_p.iter().zip(&acts) {
                l.packed.gemm_i8(a, &mut out[..batch * l.rows], kernel).unwrap();
                black_box(&out);
            }
        });
        row("ternary panel kernel, 1 thread", msp1, 1);
        let msp4 = time(&mut || {
            for (l, a) in layer_p.iter().zip(&acts) {
                l.packed.gemm_i8_spin(a, &mut out[..batch * l.rows], kernel, &pool4).unwrap();
                black_box(&out);
            }
        });
        row("ternary panel kernel, 4 threads", msp4, 4);
        println!();
    }
    if check_only {
        println!("bitnet_prefill --check: OK");
    }
}
