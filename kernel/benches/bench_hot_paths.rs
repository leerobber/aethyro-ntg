use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ntg_kernel::genomic::{BitstreamGenotypes, LdComputer};
use ntg_kernel::{Graph, NodeKind};

fn generate_genotypes(n_samples: usize, n_snps: usize) -> Vec<BitstreamGenotypes> {
    let mut snps = Vec::new();
    for i in 0..n_snps {
        let mut g = BitstreamGenotypes::new(n_samples);
        for s in 0..n_samples {
            let gt = ((s as u32).wrapping_mul(7).wrapping_add(i as u32)) % 3;
            g.set(s, gt as u8);
        }
        snps.push(g);
    }
    snps
}

fn bench_ld_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ld_computation");

    for n_snps in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}snps_100samples", n_snps)),
            n_snps,
            |b, &n_snps| {
                let snps = black_box(generate_genotypes(100, n_snps));
                let positions: Vec<u32> = (0..n_snps as u32).map(|i| i * 1000).collect();
                b.iter(|| {
                    let computer = LdComputer::new(false, 0.0);
                    computer.compute_ld(&snps, &positions)
                });
            },
        );
    }

    group.finish();
}

fn bench_graph_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_operations");

    group.bench_function("add_node_100times", |b| {
        b.iter(|| {
            let mut graph = black_box(Graph::new());
            for i in 0..100 {
                graph.add_node(NodeKind::Content, format!("node_{}", i));
            }
        });
    });

    group.bench_function("add_edge_1000times", |b| {
        let mut graph = Graph::new();
        for i in 0..100 {
            graph.add_node(NodeKind::Content, format!("node_{}", i));
        }
        let graph = black_box(graph);

        b.iter(|| {
            let mut g = graph.clone();
            for i in 0..100 {
                for j in (i + 1)..100.min(i + 10) {
                    g.add_edge(i, j).ok();
                }
            }
        });
    });

    group.finish();
}

fn bench_bitstream_genotypes(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitstream_genotypes");

    group.bench_function("set_get_1million", |b| {
        let mut g = black_box(BitstreamGenotypes::new(10000));
        b.iter(|| {
            for i in 0..10000 {
                g.set(i, (i % 3) as u8);
            }
            for i in 0..10000 {
                let _ = black_box(g.get(i));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ld_computation,
    bench_graph_operations,
    bench_bitstream_genotypes
);
criterion_main!(benches);
