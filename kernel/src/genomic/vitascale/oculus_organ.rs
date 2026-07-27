//! Oculus — multi-stream awareness fusion organ (ADR 0010 §4.3.6).
//!
//! Coverage SLA formula: count(streams on SLA) / count(registered).
//! A stream is "on SLA" if it received ≥ sla_min_signals_per_tick signals
//! since the last call to begin_tick().

use std::collections::HashMap;

use super::awareness_bus::{NeuroSignal, StreamId};
use super::organ_live::TissueLive;
use crate::genomic::organ::Organ;

/// Registered awareness stream with its SLA floor.
#[derive(Clone, Debug)]
pub struct EyeStream {
    pub id: StreamId,
    /// Minimum signals per tick to be considered on SLA. 0 = any signal counts.
    pub sla_min_signals_per_tick: u32,
}

/// Snapshot of awareness coverage for one tick.
#[derive(Clone, Debug)]
pub struct AwarenessFrame {
    pub tick: u64,
    /// count(streams on SLA) / count(registered); 1.0 if no streams registered.
    pub coverage: f32,
    pub streams_on_sla: usize,
    pub streams_registered: usize,
    pub pending_total: usize,
}

/// Multi-stream awareness fusion organ. Implements TissueLive so Crown routes
/// bus signals to `on_bus_signal`, keeping the signal-count-per-stream table
/// up to date without polling the bus directly.
pub struct Oculus {
    registered: Vec<EyeStream>,
    /// Signals received per stream since last begin_tick().
    signals_this_tick: HashMap<StreamId, u32>,
    tick: u64,
    pub last_frame: Option<AwarenessFrame>,
}

impl Oculus {
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
            signals_this_tick: HashMap::new(),
            tick: 0,
            last_frame: None,
        }
    }

    /// Register a stream with an SLA floor. No-op if already registered.
    pub fn register_stream(&mut self, stream: EyeStream) {
        if !self.registered.iter().any(|s| s.id == stream.id) {
            self.registered.push(stream);
        }
    }

    /// Reset per-tick signal counts (call once at the start of each Crown tick).
    pub fn begin_tick(&mut self) {
        self.signals_this_tick.clear();
        self.tick += 1;
    }

    /// Compute and store an AwarenessFrame. `pending_total` is from `Sensefield::pending()`.
    pub fn compute_frame(&mut self, pending_total: usize) -> AwarenessFrame {
        let n = self.registered.len();
        let on_sla = self
            .registered
            .iter()
            .filter(|s| {
                let count = self.signals_this_tick.get(&s.id).copied().unwrap_or(0);
                count > s.sla_min_signals_per_tick
                    || (s.sla_min_signals_per_tick == 0 && count > 0)
            })
            .count();

        let coverage = if n == 0 { 1.0 } else { on_sla as f32 / n as f32 };

        let frame = AwarenessFrame {
            tick: self.tick,
            coverage,
            streams_on_sla: on_sla,
            streams_registered: n,
            pending_total,
        };
        self.last_frame = Some(frame.clone());
        frame
    }

    pub fn registered_count(&self) -> usize {
        self.registered.len()
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }
}

impl Default for Oculus {
    fn default() -> Self {
        Self::new()
    }
}

impl Organ for Oculus {
    fn kind(&self) -> &'static str {
        "oculus"
    }
    fn approx_memory_bytes(&self) -> u64 {
        (self.registered.len() * 32 + self.signals_this_tick.len() * 16) as u64
    }
    fn structure_fingerprint(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        h ^= self.registered.len() as u64;
        h = h.rotate_left(7);
        h ^= self.tick;
        h
    }
    fn unit_count(&self) -> usize {
        self.registered.len()
    }
}

impl TissueLive for Oculus {
    fn on_bus_signal(&mut self, sig: &NeuroSignal) {
        *self.signals_this_tick.entry(sig.stream).or_insert(0) += 1;
    }

    fn health(&self) -> f32 {
        self.last_frame
            .as_ref()
            .map(|f| f.coverage)
            .unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::awareness_bus::NeuroSignal;

    #[test]
    fn oculus_coverage_no_streams() {
        let mut o = Oculus::new();
        o.begin_tick();
        let frame = o.compute_frame(0);
        assert_eq!(frame.coverage, 1.0, "no registered streams → coverage = 1.0");
    }

    #[test]
    fn oculus_coverage_all_on_sla() {
        let mut o = Oculus::new();
        o.register_stream(EyeStream {
            id: StreamId::Structural,
            sla_min_signals_per_tick: 0,
        });
        o.register_stream(EyeStream {
            id: StreamId::Biological,
            sla_min_signals_per_tick: 0,
        });
        o.begin_tick();
        o.on_bus_signal(&NeuroSignal::new(StreamId::Structural, 0, 1));
        o.on_bus_signal(&NeuroSignal::new(StreamId::Biological, 0, 2));
        let frame = o.compute_frame(2);
        assert_eq!(frame.streams_on_sla, 2);
        assert_eq!(frame.coverage, 1.0);
    }

    #[test]
    fn oculus_coverage_partial_sla() {
        let mut o = Oculus::new();
        o.register_stream(EyeStream {
            id: StreamId::Structural,
            sla_min_signals_per_tick: 0,
        });
        o.register_stream(EyeStream {
            id: StreamId::Temporal,
            sla_min_signals_per_tick: 0,
        });
        o.begin_tick();
        // Only Structural gets a signal.
        o.on_bus_signal(&NeuroSignal::new(StreamId::Structural, 0, 1));
        let frame = o.compute_frame(1);
        assert_eq!(frame.streams_on_sla, 1);
        assert!((frame.coverage - 0.5).abs() < 1e-6);
    }

    #[test]
    fn oculus_health_reflects_coverage() {
        let mut o = Oculus::new();
        o.register_stream(EyeStream {
            id: StreamId::Immune,
            sla_min_signals_per_tick: 0,
        });
        o.begin_tick();
        // No signals → coverage = 0.
        let _ = o.compute_frame(0);
        assert_eq!(o.health(), 0.0);
    }

    #[test]
    fn oculus_sla_min_signals_respected() {
        let mut o = Oculus::new();
        o.register_stream(EyeStream {
            id: StreamId::Evolutionary,
            sla_min_signals_per_tick: 2, // needs >2 signals (i.e. ≥3)
        });
        o.begin_tick();
        o.on_bus_signal(&NeuroSignal::new(StreamId::Evolutionary, 0, 1));
        o.on_bus_signal(&NeuroSignal::new(StreamId::Evolutionary, 0, 2));
        let frame = o.compute_frame(2); // only 2 signals, need >2
        assert_eq!(frame.streams_on_sla, 0);
        // Add one more
        o.begin_tick();
        for _ in 0..3 {
            o.on_bus_signal(&NeuroSignal::new(StreamId::Evolutionary, 0, 3));
        }
        let frame2 = o.compute_frame(3);
        assert_eq!(frame2.streams_on_sla, 1);
    }
}
