//! Body adapter — abstract chassis interface for VITASCALE L0 (ADR 0010 §4.3.8).
//!
//! `BodyAdapter` is the trait implemented by each chassis kind:
//!   L0 → IronChassis (CPU polling, this module)
//!   L2 → neuromorphic (feature-gated `body_neuromorphic`, not yet implemented)
//!   L3 → wetware (feature-gated `body_wetware`, not yet implemented)
//!
//! All L2/L3 paths remain behind Cargo feature flags; the default build is L0 only.

pub mod host_cpu;

/// Snapshot of the body's physical health at one instant.
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyHealth {
    /// Normalised CPU load [0, 1]; 1.0 = fully saturated.
    pub cpu_load: f32,
    /// Process RSS in bytes (best-effort estimate).
    pub mem_used_bytes: u64,
    /// Available OS thread count.
    pub thread_count: usize,
}

/// A complete body-layer frame: health metrics + timestamp.
#[derive(Clone, Copy, Debug)]
pub struct BodyFrame {
    pub health: BodyHealth,
    /// Monotonic nanosecond timestamp (for observability / ordering only).
    pub timestamp_ns: u64,
}

/// Commands Crown may send to the body adapter.
#[derive(Clone, Debug, PartialEq)]
pub enum BodyCommand {
    /// Throttle CPU usage to this fraction [0, 1].
    Throttle(f32),
    /// Spawn additional worker threads (L2/L3 only in practice).
    Spawn { thread_count: u8 },
    /// Enter hibernation (stop accepting new ticks).
    Hibernate,
}

/// Error returned when a BodyCommand cannot be executed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyError {
    /// Command not supported by this chassis type.
    Unsupported,
    /// Chassis lacks sufficient resources to fulfil the command.
    ResourceExhausted,
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::Unsupported => write!(f, "body command not supported"),
            BodyError::ResourceExhausted => write!(f, "body resource exhausted"),
        }
    }
}

/// Abstract chassis trait. Implement for each physical substrate.
pub trait BodyAdapter {
    /// Sample the current body health frame.
    fn sample(&self) -> BodyFrame;
    /// Execute a chassis command. Returns Err if not supported or resources are exhausted.
    fn execute(&mut self, cmd: BodyCommand) -> Result<(), BodyError>;
    /// Human-readable chassis type name (e.g. "iron_cpu", "neuromorphic_stub").
    fn chassis_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyChassis;

    impl BodyAdapter for DummyChassis {
        fn sample(&self) -> BodyFrame {
            BodyFrame {
                health: BodyHealth {
                    cpu_load: 0.1,
                    mem_used_bytes: 1024,
                    thread_count: 4,
                },
                timestamp_ns: 0,
            }
        }
        fn execute(&mut self, _cmd: BodyCommand) -> Result<(), BodyError> {
            Err(BodyError::Unsupported)
        }
        fn chassis_name(&self) -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn body_adapter_sample() {
        let c = DummyChassis;
        let frame = c.sample();
        assert_eq!(frame.health.thread_count, 4);
        assert!((frame.health.cpu_load - 0.1).abs() < 1e-6);
    }

    #[test]
    fn body_command_unsupported() {
        let mut c = DummyChassis;
        let err = c.execute(BodyCommand::Throttle(0.5));
        assert_eq!(err, Err(BodyError::Unsupported));
    }
}
