//! SIMD path profiling and benchmarking.
//!
//! Measures real performance of each SIMD path:
//! - Wall-clock latency
//! - Throughput (ops/sec)
//! - Correctness vs. scalar reference (a path that fails this is never
//!   eligible for selection, regardless of speed)
//!
//! `profile_simd_path` previously returned a hardcoded placeholder
//! (`passed_correctness: true`, `latency_us: 0.0` for every path,
//! unconditionally) -- meaning the "self-tuning" dispatcher had no real
//! signal to tune on. It now actually runs each path and measures it.

use super::dispatcher::SIMDPath;
use super::super::ternary::matmul_scalar;
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

/// Run the given path's matmul implementation, falling back to scalar for
/// paths that aren't actually implemented/available on this hardware.
/// Mirrors `SIMDDispatcher::matmul`'s per-path fallback logic so profiling
/// measures the same code path real calls would take.
fn run_path(path: SIMDPath, a: &[i8], b: &[i8], m: usize, k: usize, n: usize)
    -> Result<Vec<f32>, crate::ntg::error::NtgError>
{
    match path {
        SIMDPath::Scalar => matmul_scalar(a, b, m, k, n),
        SIMDPath::AVX2 => {
            #[cfg(target_arch = "x86_64")]
            {
                if is_x86_feature_detected!("avx2") {
                    return super::avx2::matmul_avx2(a, b, m, k, n);
                }
            }
            matmul_scalar(a, b, m, k, n)
        }
        SIMDPath::NEON => {
            #[cfg(target_arch = "aarch64")]
            {
                if cfg!(target_feature = "neon") {
                    return super::neon::matmul_neon(a, b, m, k, n);
                }
            }
            matmul_scalar(a, b, m, k, n)
        }
        SIMDPath::SSE41 => matmul_scalar(a, b, m, k, n), // not implemented
    }
}

/// Profile a single SIMD path: real correctness check against the scalar
/// reference, then real wall-clock timing.
///
/// `test_size` is clamped to a small range regardless of what's passed in.
/// This is called from `SIMDDispatcher::new()` at every process startup,
/// so an uncapped large size (this has been called with 1000) would make
/// every startup pay for an O(size^3) scalar-reference matmul; the
/// correctness check and timing both only need a representative workload,
/// not the caller's literal requested size.
pub fn profile_simd_path(path: SIMDPath, test_size: usize) -> Result<ProfileResult, String> {
    let dim = test_size.clamp(8, 48);
    let m = dim;
    let k = dim;
    let n = dim;

    // Deterministic pseudo-random ternary data (no external RNG dependency).
    let mut state: u64 = 0xA5A5_1234_ABCD_EF01 ^ (path as u64);
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let gen = |len: usize, next: &mut dyn FnMut() -> u64| -> Vec<i8> {
        (0..len)
            .map(|_| match next() % 3 {
                0 => -1i8,
                1 => 0i8,
                _ => 1i8,
            })
            .collect()
    };
    let a = gen(m * k, &mut next);
    let b = gen(k * n, &mut next);

    let reference = matmul_scalar(&a, &b, m, k, n)
        .map_err(|e| format!("scalar reference failed: {e}"))?;

    let candidate = run_path(path, &a, &b, m, k, n);
    let passed_correctness = match &candidate {
        Ok(result) => *result == reference,
        Err(_) => false,
    };

    let (latency_us, _throughput) = benchmark_matmul(
        || run_path(path, &a, &b, m, k, n),
        dim,
        5,
    );

    Ok(ProfileResult {
        path,
        passed_correctness,
        latency_us,
        memory_bytes: (m * k + k * n + m * n) * std::mem::size_of::<f32>(),
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
        // Nanosecond resolution converted to fractional microseconds --
        // truncating to whole microseconds (the previous behavior) reports
        // 0.0 for anything faster than 1us, which is common for small
        // matmuls and was making this measurement meaningless for exactly
        // the fast paths it exists to distinguish.
        latencies.push(elapsed.as_nanos() as f64 / 1000.0);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Return median latency
    let median_latency = latencies[latencies.len() / 2];

    // Compute throughput (matrix multiplications per second). Guard
    // against a still-zero median (possible on very coarse-grained clocks)
    // rather than dividing by zero and producing infinity.
    let throughput = if median_latency > 0.0 {
        1_000_000.0 / median_latency
    } else {
        0.0
    };

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
        // Benchmarking a trivial 100-element allocation was flaky: on a
        // fast run it can complete faster than the timer's practical
        // resolution, making `latency > 0.0` fail nondeterministically
        // (observed: passes most runs, fails occasionally). Benchmarking
        // an actual matmul -- what this function exists to measure in
        // real use -- takes reliably measurable time and is a more
        // meaningful thing to assert about anyway.
        let dim = 64;
        let a = vec![1i8; dim * dim];
        let b = vec![1i8; dim * dim];
        let (latency, throughput) = benchmark_matmul(
            || matmul_scalar(&a, &b, dim, dim, dim),
            dim,
            5,
        );
        assert!(latency > 0.0);
        assert!(throughput > 0.0);
    }
}
