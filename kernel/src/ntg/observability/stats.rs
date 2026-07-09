//! Thread-safe, lock-free stats collector for OpStats aggregation.
//!
//! Adapts to the real FFI [`crate::ntg::ffi::OpStats`] shape (latency_us,
//! simd_path, memory_bytes) — not a fictional NtgStats type.

use crate::ntg::ffi::OpStats;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Aggregates per-call [`OpStats`] into process-wide counters.
///
/// All counters use `Relaxed` atomics — hot path must not contend.
#[derive(Debug)]
pub struct StatsCollector {
    ops_total: AtomicU64,
    time_total_ns: AtomicU64,
    memory_bytes_last: AtomicU64,
    ops_scalar: AtomicU64,
    ops_avx2: AtomicU64,
    ops_neon: AtomicU64,
    ops_sse: AtomicU64,
    errors_null: AtomicU64,
    errors_len: AtomicU64,
    errors_value: AtomicU64,
    errors_other: AtomicU64,
    start_time: Instant,
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            ops_total: AtomicU64::new(0),
            time_total_ns: AtomicU64::new(0),
            memory_bytes_last: AtomicU64::new(0),
            ops_scalar: AtomicU64::new(0),
            ops_avx2: AtomicU64::new(0),
            ops_neon: AtomicU64::new(0),
            ops_sse: AtomicU64::new(0),
            errors_null: AtomicU64::new(0),
            errors_len: AtomicU64::new(0),
            errors_value: AtomicU64::new(0),
            errors_other: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record a successful (or partially filled) OpStats sample.
    /// Counts one op; accumulates latency in nanoseconds (from µs field).
    #[inline]
    pub fn record(&self, stats: &OpStats) {
        self.ops_total.fetch_add(1, Ordering::Relaxed);
        let ns = stats.latency_us.saturating_mul(1_000);
        self.time_total_ns.fetch_add(ns, Ordering::Relaxed);
        self.memory_bytes_last
            .store(stats.memory_bytes, Ordering::Relaxed);

        match stats.simd_path {
            0 => {
                self.ops_scalar.fetch_add(1, Ordering::Relaxed);
            }
            1 => {
                self.ops_avx2.fetch_add(1, Ordering::Relaxed);
            }
            2 => {
                self.ops_neon.fetch_add(1, Ordering::Relaxed);
            }
            3 => {
                self.ops_sse.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record an FFI-style error code without an OpStats payload.
    /// Convention: -1 null, -2 length/shape, -3 invalid value, other = other.
    #[inline]
    pub fn record_error(&self, code: i32) {
        match code {
            -1 => {
                self.errors_null.fetch_add(1, Ordering::Relaxed);
            }
            -2 => {
                self.errors_len.fetch_add(1, Ordering::Relaxed);
            }
            -3 => {
                self.errors_value.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.errors_other.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            uptime: self.start_time.elapsed(),
            ops_total: self.ops_total.load(Ordering::Relaxed),
            time_total_ns: self.time_total_ns.load(Ordering::Relaxed),
            memory_bytes_last: self.memory_bytes_last.load(Ordering::Relaxed),
            ops_scalar: self.ops_scalar.load(Ordering::Relaxed),
            ops_avx2: self.ops_avx2.load(Ordering::Relaxed),
            ops_neon: self.ops_neon.load(Ordering::Relaxed),
            ops_sse: self.ops_sse.load(Ordering::Relaxed),
            errors_null: self.errors_null.load(Ordering::Relaxed),
            errors_len: self.errors_len.load(Ordering::Relaxed),
            errors_value: self.errors_value.load(Ordering::Relaxed),
            errors_other: self.errors_other.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub uptime: std::time::Duration,
    pub ops_total: u64,
    pub time_total_ns: u64,
    pub memory_bytes_last: u64,
    pub ops_scalar: u64,
    pub ops_avx2: u64,
    pub ops_neon: u64,
    pub ops_sse: u64,
    pub errors_null: u64,
    pub errors_len: u64,
    pub errors_value: u64,
    pub errors_other: u64,
}

impl StatsSnapshot {
    pub fn ops_per_second(&self) -> f64 {
        let s = self.uptime.as_secs_f64();
        if s > 0.0 {
            self.ops_total as f64 / s
        } else {
            0.0
        }
    }

    pub fn avg_latency_ns(&self) -> f64 {
        if self.ops_total > 0 {
            self.time_total_ns as f64 / self.ops_total as f64
        } else {
            0.0
        }
    }

    pub fn error_total(&self) -> u64 {
        self.errors_null + self.errors_len + self.errors_value + self.errors_other
    }

    /// Dominant dispatch path by op counts (not last-call).
    pub fn current_path(&self) -> &'static str {
        let a = self.ops_avx2;
        let n = self.ops_neon;
        let s = self.ops_scalar;
        let e = self.ops_sse;
        let m = a.max(n).max(s).max(e);
        if m == 0 {
            return "None";
        }
        if a == m {
            "AVX2"
        } else if n == m {
            "NEON"
        } else if e == m {
            "SSE4.1"
        } else {
            "Scalar"
        }
    }

    /// Compact one-line for notebooks / CLI.
    pub fn summary_line(&self) -> String {
        format!(
            "ops={} path={} avg_lat_ns={:.0} err={} uptime_ms={}",
            self.ops_total,
            self.current_path(),
            self.avg_latency_ns(),
            self.error_total(),
            self.uptime.as_millis()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_aggregates_paths() {
        let c = StatsCollector::new();
        c.record(&OpStats {
            latency_us: 10,
            memory_bytes: 64,
            simd_path: 0,
            timestamp_ns: 1,
        });
        c.record(&OpStats {
            latency_us: 5,
            memory_bytes: 128,
            simd_path: 1,
            timestamp_ns: 2,
        });
        c.record(&OpStats {
            latency_us: 5,
            memory_bytes: 128,
            simd_path: 1,
            timestamp_ns: 3,
        });
        let s = c.snapshot();
        assert_eq!(s.ops_total, 3);
        assert_eq!(s.ops_scalar, 1);
        assert_eq!(s.ops_avx2, 2);
        assert_eq!(s.time_total_ns, 20_000); // 10+5+5 µs → ns
        assert_eq!(s.current_path(), "AVX2");
        assert!((s.avg_latency_ns() - (20_000.0 / 3.0)).abs() < 1.0);
    }

    #[test]
    fn record_error_codes() {
        let c = StatsCollector::new();
        c.record_error(-1);
        c.record_error(-2);
        c.record_error(-3);
        c.record_error(99);
        let s = c.snapshot();
        assert_eq!(s.errors_null, 1);
        assert_eq!(s.errors_len, 1);
        assert_eq!(s.errors_value, 1);
        assert_eq!(s.errors_other, 1);
        assert_eq!(s.error_total(), 4);
    }

    #[test]
    fn summary_line_nonempty() {
        let c = StatsCollector::new();
        c.record(&OpStats::default());
        let line = c.snapshot().summary_line();
        assert!(line.contains("ops=1"));
    }
}
