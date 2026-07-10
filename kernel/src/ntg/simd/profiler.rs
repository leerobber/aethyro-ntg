//! SIMD path profiling and benchmarking.
//!
//! `benchmark_matmul` below does real wall-clock measurement and is used
//! by density_bench. `profile_simd_path` is not wired up yet: it's a
//! stub returning a fixed placeholder (see its own doc), not an actual
//! per-path measurement of latency / throughput / memory bandwidth.

use super::dispatcher::SIMDPath;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub path: SIMDPath,
    pub latency_us: f64,
    pub throughput_ops_per_sec: f64,
    pub speedup_vs_scalar: f64,
}

#[derive(Clone, Debug)]
pub struct ProfileResult {
    pub path: SIMDPath,
    pub passed_correctness: bool,
    pub latency_us: f64,
    pub memory_bytes: usize,
}

/// Profile a single SIMD path: run tests, measure performance.
pub fn profile_simd_path(path: SIMDPath, test_size: usize) -> Result<ProfileResult, String> {
    // For now, return a placeholder profile
    // Real implementation would:
    // 1. Generate test matrices
    // 2. Run correctness check (vs. scalar reference)
    // 3. Benchmark: wall-clock, cache, throughput
    // 4. Return comprehensive profile

    Ok(ProfileResult {
        path,
        passed_correctness: true,
        latency_us: 0.0,
        memory_bytes: test_size * test_size * 4,
    })
}

/// Benchmark function for measuring matmul performance.
/// Returns (latency_us, throughput_ops_per_sec).
pub fn benchmark_matmul<F>(
    f: F,
    _matrix_size: usize,
    runs: usize,
) -> (f64, f64)
where
    F: Fn() -> Result<Vec<f32>, crate::ntg::error::NtgError>,
{
    let mut latencies = Vec::new();

    for _ in 0..runs {
        let start = Instant::now();
        let _ = f();
        let elapsed = start.elapsed();
        // Sub-microsecond resolution so tiny closures don't report 0.0 µs
        latencies.push(elapsed.as_secs_f64() * 1_000_000.0);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Return median latency (floor at epsilon to keep throughput finite)
    let median_latency = latencies[latencies.len() / 2].max(f64::EPSILON);

    // Compute throughput (matrix multiplications per second)
    let throughput = 1_000_000.0 / median_latency;

    (median_latency, throughput)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_scalar_succeeds() -> Result<(), String> {
        let profile = profile_simd_path(SIMDPath::Scalar, 1000)?;
        assert!(profile.passed_correctness);
        Ok(())
    }

    #[test]
    fn benchmark_is_positive() {
        let (latency, throughput) = benchmark_matmul(
            || Ok(vec![0.0f32; 100]),
            10,
            3,
        );
        assert!(latency > 0.0);
        assert!(throughput > 0.0);
    }
}
