//! Graph forward-pass overhead vs static LeafSignal fold.
//!
//! Run: cargo run --release --bin graph_overhead_bench
//!
//! Measures median wall-clock of:
//! 1. Graph::forward_pass (topo + signal combine)
//! 2. Static fold over the same signals in a Vec (no graph)

use ntg_kernel::ntg::docparse;
use ntg_kernel::ntg::graph::Graph;
use ntg_kernel::ntg::leafsignal::LeafSignal;
use std::hint::black_box;
use std::time::Instant;

const WARMUP: usize = 50;
const ITERS: usize = 500;

fn median_ns(samples: &mut [u128]) -> f64 {
    samples.sort_unstable();
    samples[samples.len() / 2] as f64
}

fn time_ns<F: FnMut()>(mut f: F) -> f64 {
    for _ in 0..WARMUP {
        f();
    }
    let mut samples = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos());
    }
    median_ns(&mut samples)
}

fn main() {
    let mut g = Graph::new();
    // Real-ish structure from a small doc
    let doc = r#"# Title
## Section
- item one
- item two
```rust
fn x() {}
```
## Other
- more
"#;
    docparse::parse_into(&mut g, "bench", doc);

    let signals: Vec<LeafSignal> = g
        .all_node_ids()
        .iter()
        .map(|&id| g.node(id).unwrap().signal)
        .collect();

    let graph_ns = time_ns(|| {
        let _ = black_box(g.forward_pass().unwrap());
    });
    let static_ns = time_ns(|| {
        let mut total = LeafSignal::default();
        for s in &signals {
            total = total.combine(s);
        }
        black_box(total);
    });

    let graph_us = graph_ns / 1000.0;
    let static_us = static_ns / 1000.0;
    let overhead = if static_us > 0.0 {
        graph_us / static_us
    } else {
        f64::INFINITY
    };

    println!("# graph_overhead_bench");
    println!("nodes={}", g.node_count());
    println!("graph_forward_pass_us={:.4}", graph_us);
    println!("static_signal_fold_us={:.4}", static_us);
    println!("overhead_ratio={:.3}x (graph / static)", overhead);
    println!();
    println!("Interpretation: ratio > 1 means graph scheduling costs more than a flat fold of the same signals. This measures structure overhead, not ternary TOBL.");
}
