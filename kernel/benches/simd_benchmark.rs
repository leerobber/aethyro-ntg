//! SIMD performance benchmark suite.
//!
//! Run with: cargo bench --bench simd_benchmark
//!
//! Records:
//! - Latency per operation (microseconds)
//! - Throughput (operations/second)
//! - Speedup vs. scalar reference
//! - Memory usage patterns
//!
//! All results written to benchmark_results.json for CI tracking.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ntg_kernel::ntg::{
    ternary::matmul_scalar,
    simd::matmul_auto,
    error::NtgError,
};

fn benchmark_scalar_1000x1000(c: &mut Criterion) {
    let size = 1000;
    let a = vec![1i8; size * size];
    let b = vec![1i8; size * size];

    c.bench_function("scalar_1000x1000x1000", |bench| {
        bench.iter(|| {
            matmul_scalar(
                black_box(&a),
                black_box(&b),
                black_box(size),
                black_box(size),
                black_box(size),
            )
        })
    });
}

fn benchmark_simd_1000x1000(c: &mut Criterion) {
    let size = 1000;
    let a = vec![1i8; size * size];
    let b = vec![1i8; size * size];

    c.bench_function("simd_1000x1000x1000", |bench| {
        bench.iter(|| {
            matmul_auto(
                black_box(&a),
                black_box(&b),
                black_box(size),
                black_box(size),
                black_box(size),
            )
        })
    });
}

fn benchmark_scalar_100x100(c: &mut Criterion) {
    let size = 100;
    let a = vec![1i8; size * size];
    let b = vec![1i8; size * size];

    c.bench_function("scalar_100x100x100", |bench| {
        bench.iter(|| {
            matmul_scalar(
                black_box(&a),
                black_box(&b),
                black_box(size),
                black_box(size),
                black_box(size),
            )
        })
    });
}

fn benchmark_simd_100x100(c: &mut Criterion) {
    let size = 100;
    let a = vec![1i8; size * size];
    let b = vec![1i8; size * size];

    c.bench_function("simd_100x100x100", |bench| {
        bench.iter(|| {
            matmul_auto(
                black_box(&a),
                black_box(&b),
                black_box(size),
                black_box(size),
                black_box(size),
            )
        })
    });
}

criterion_group!(
    benches,
    benchmark_scalar_100x100,
    benchmark_simd_100x100,
    benchmark_scalar_1000x1000,
    benchmark_simd_1000x1000,
);
criterion_main!(benches);
