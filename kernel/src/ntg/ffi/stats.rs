//! Operation statistics: captured per FFI call for ledger ingestion.
//!
//! Each matmul operation produces an OpStats that records:
//! - Performance (latency, memory)
//! - Which SIMD path was used
//! - Timestamp for sequencing
//!
//! These flow directly into Phase 3's TamperEvidentLedger.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpStats {
    /// Forward pass latency in microseconds
    pub latency_us: u64,
    /// Peak memory usage in bytes
    pub memory_bytes: u64,
    /// Which SIMD path was used (0=Scalar, 1=AVX2, 2=NEON, 3=SSE4.1)
    pub simd_path: u8,
    /// Wall-clock timestamp in nanoseconds (Unix epoch)
    pub timestamp_ns: u64,
}

impl OpStats {
    pub fn new() -> Self {
        Self {
            latency_us: 0,
            memory_bytes: 0,
            simd_path: 0,
            timestamp_ns: 0,
        }
    }

    /// Get human-readable SIMD path name
    pub fn simd_path_name(&self) -> &'static str {
        match self.simd_path {
            0 => "Scalar",
            1 => "AVX2",
            2 => "NEON",
            3 => "SSE4.1",
            _ => "Unknown",
        }
    }

    /// Dual-objective fitness improvement check (Phase 3 compatible)
    pub fn improves_over(&self, baseline: &OpStats, threshold: f32) -> bool {
        let latency_ratio = self.latency_us as f32 / baseline.latency_us.max(1) as f32;
        let memory_ratio = self.memory_bytes as f32 / baseline.memory_bytes.max(1) as f32;
        latency_ratio <= threshold && memory_ratio <= threshold
    }

    /// Format as JSON for ledger logging
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"latency_us":{},"memory_bytes":{},"simd_path":"{}","timestamp_ns":{}}}"#,
            self.latency_us, self.memory_bytes, self.simd_path_name(), self.timestamp_ns
        )
    }
}

impl Default for OpStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opstats_default() {
        let stats = OpStats::default();
        assert_eq!(stats.latency_us, 0);
        assert_eq!(stats.memory_bytes, 0);
        assert_eq!(stats.simd_path, 0);
    }

    #[test]
    fn simd_path_names() {
        let s_scalar = OpStats {
            simd_path: 0,
            ..Default::default()
        };
        assert_eq!(s_scalar.simd_path_name(), "Scalar");

        let s_avx2 = OpStats {
            simd_path: 1,
            ..Default::default()
        };
        assert_eq!(s_avx2.simd_path_name(), "AVX2");
    }

    #[test]
    fn improvement_check() {
        let baseline = OpStats {
            latency_us: 5000,
            memory_bytes: 1024,
            ..Default::default()
        };

        // 1% improvement
        let improved = OpStats {
            latency_us: 4950,
            memory_bytes: 1000,
            ..Default::default()
        };
        assert!(improved.improves_over(&baseline, 1.01));

        // Regression
        let regressed = OpStats {
            latency_us: 5100,
            memory_bytes: 1100,
            ..Default::default()
        };
        assert!(!regressed.improves_over(&baseline, 1.01));
    }

    #[test]
    fn to_json() {
        let stats = OpStats {
            latency_us: 1000,
            memory_bytes: 512,
            simd_path: 1,
            timestamp_ns: 1234567890,
        };
        let json = stats.to_json();
        assert!(json.contains("1000"));
        assert!(json.contains("512"));
        assert!(json.contains("AVX2"));
    }
}
