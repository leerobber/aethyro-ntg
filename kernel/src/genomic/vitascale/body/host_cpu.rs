//! IronChassis — L0 CPU body adapter (ADR 0010 §4.3.8.1).
//!
//! Reads available parallelism via `std::thread::available_parallelism`.
//! CPU load and RSS are sampled from `/proc/self/stat` on Linux when available;
//! falls back to zero on non-Linux targets (honesty principle: we don't guess).
//!
//! L2 (neuromorphic) and L3 (wetware) stubs are feature-gated and not present in L0.

use super::{BodyAdapter, BodyCommand, BodyError, BodyFrame, BodyHealth};
use crate::genomic::organ::Organ;
use crate::genomic::vitascale::organ_live::TissueLive;

/// L0 CPU chassis adapter.
pub struct IronChassis {
    thread_count: usize,
    /// Current throttle level [0, 1]; informational only in L0.
    throttle: f32,
    hibernating: bool,
}

impl IronChassis {
    /// Auto-detect thread count via `available_parallelism`.
    pub fn detect() -> Self {
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self {
            thread_count,
            throttle: 1.0,
            hibernating: false,
        }
    }

    /// Construct with an explicit thread count (useful in tests and CI).
    pub fn with_threads(thread_count: usize) -> Self {
        Self {
            thread_count,
            throttle: 1.0,
            hibernating: false,
        }
    }

    pub fn is_hibernating(&self) -> bool {
        self.hibernating
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }

    /// Attempt to read process RSS from `/proc/self/status` (Linux only).
    #[cfg(target_os = "linux")]
    fn read_rss_bytes() -> u64 {
        use std::fs;
        // VmRSS line: "VmRSS:   12345 kB"
        fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
            })
            .unwrap_or(0)
            * 1024 // kB → bytes
    }

    #[cfg(not(target_os = "linux"))]
    fn read_rss_bytes() -> u64 {
        0
    }

    /// Read the monotonic clock in nanoseconds.
    fn now_ns() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}

impl BodyAdapter for IronChassis {
    fn sample(&self) -> BodyFrame {
        BodyFrame {
            health: BodyHealth {
                // CPU load is not measurable without OS-level counters in L0.
                // Surface 0.0 honestly rather than guessing (honesty principle).
                cpu_load: 0.0,
                mem_used_bytes: Self::read_rss_bytes(),
                thread_count: self.thread_count,
            },
            timestamp_ns: Self::now_ns(),
        }
    }

    fn execute(&mut self, cmd: BodyCommand) -> Result<(), BodyError> {
        match cmd {
            BodyCommand::Throttle(t) => {
                self.throttle = t.clamp(0.0, 1.0);
                Ok(())
            }
            BodyCommand::Hibernate => {
                self.hibernating = true;
                Ok(())
            }
            BodyCommand::Spawn { .. } => {
                // L0 does not manage threads dynamically.
                Err(BodyError::Unsupported)
            }
        }
    }

    fn chassis_name(&self) -> &'static str {
        "iron_cpu"
    }
}

impl Organ for IronChassis {
    fn kind(&self) -> &'static str {
        "iron_chassis"
    }
    fn approx_memory_bytes(&self) -> u64 {
        std::mem::size_of::<IronChassis>() as u64
    }
    fn structure_fingerprint(&self) -> u64 {
        let mut h = 0x811c9dc5u64;
        h ^= self.thread_count as u64;
        h = h.rotate_left(7);
        h ^= (self.throttle * 1000.0) as u64;
        h
    }
    fn unit_count(&self) -> usize {
        self.thread_count
    }
}

impl TissueLive for IronChassis {
    fn health(&self) -> f32 {
        if self.hibernating {
            0.0
        } else {
            1.0 - (1.0 - self.throttle).clamp(0.0, 0.5)
        }
    }

    fn stress(&self) -> f32 {
        1.0 - self.throttle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_chassis_detect() {
        let c = IronChassis::detect();
        assert!(c.thread_count >= 1);
        assert!(!c.is_hibernating());
    }

    #[test]
    fn iron_chassis_sample_returns_thread_count() {
        let c = IronChassis::with_threads(8);
        let frame = c.sample();
        assert_eq!(frame.health.thread_count, 8);
        assert!(frame.timestamp_ns > 0);
    }

    #[test]
    fn iron_chassis_throttle_command() {
        let mut c = IronChassis::with_threads(4);
        c.execute(BodyCommand::Throttle(0.5)).unwrap();
        assert!((c.throttle() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn iron_chassis_hibernate_command() {
        let mut c = IronChassis::with_threads(4);
        c.execute(BodyCommand::Hibernate).unwrap();
        assert!(c.is_hibernating());
        assert_eq!(c.health(), 0.0);
    }

    #[test]
    fn iron_chassis_spawn_unsupported() {
        let mut c = IronChassis::with_threads(4);
        let err = c.execute(BodyCommand::Spawn { thread_count: 2 });
        assert_eq!(err, Err(BodyError::Unsupported));
    }

    #[test]
    fn iron_chassis_health_reflects_throttle() {
        let mut c = IronChassis::with_threads(4);
        c.execute(BodyCommand::Throttle(0.0)).unwrap();
        assert!(c.health() < 1.0);
        assert!(c.stress() > 0.0);
    }
}
