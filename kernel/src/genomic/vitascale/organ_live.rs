//! TissueLive — extends Organ with bus-signal hook and health probe (VITASCALE L0).
//!
//! Existing `Organ` implementations remain unchanged; `TissueLive` is additive.

use super::awareness_bus::NeuroSignal;
use crate::genomic::organ::Organ;

/// Extended lifecycle for tissues that participate in the Sensefield bus.
///
/// Default impls make adoption zero-cost for tissues that don't need the bus.
pub trait TissueLive: Organ {
    /// Called by Crown when a `NeuroSignal` is routed to this tissue's stream.
    /// Default: no-op.
    fn on_bus_signal(&mut self, _sig: &NeuroSignal) {}

    /// Normalized [0.0, 1.0] health probe for Phageguard and Omniradar.
    /// Default: always healthy.
    fn health(&self) -> f32 {
        1.0
    }

    /// Optional stress indicator surfaced by Phageguard attention.
    /// Default: 0.0 (no stress).
    fn stress(&self) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyOrgan {
        health: f32,
    }

    impl Organ for DummyOrgan {
        fn kind(&self) -> &'static str {
            "dummy"
        }
        fn approx_memory_bytes(&self) -> u64 {
            0
        }
        fn structure_fingerprint(&self) -> u64 {
            42
        }
        fn unit_count(&self) -> usize {
            1
        }
    }

    impl TissueLive for DummyOrgan {
        fn health(&self) -> f32 {
            self.health
        }
    }

    #[test]
    fn tissue_live_defaults() {
        let d = DummyOrgan { health: 0.75 };
        assert_eq!(d.health(), 0.75);
        assert_eq!(d.stress(), 0.0);
    }

    #[test]
    fn on_bus_signal_default_noop() {
        let mut d = DummyOrgan { health: 1.0 };
        let sig = NeuroSignal {
            stream: crate::genomic::vitascale::awareness_bus::StreamId::Structural,
            severity: 0,
            code: 1,
            payload: [0; 4],
            t_ns: 0,
        };
        d.on_bus_signal(&sig); // must not panic
    }
}
