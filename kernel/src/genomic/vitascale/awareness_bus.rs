//! Sensefield — in-process pub/sub bus for VITASCALE awareness (ADR 0010 §4.3.4).
//!
//! Bounded queues per topic. Drop-with-counter semantics — a full queue is itself
//! a visible stress signal. Single-threaded L0: deterministic drain order.

use std::collections::VecDeque;

/// Identifies which awareness dimension a signal belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamId {
    Structural,
    Temporal,
    Evolutionary,
    Biological,
    Immune,
    /// Oculus sub-stream index.
    Eye(u16),
    /// Body/chassis sub-stream index.
    Body(u16),
    /// Per-neurocyte stream, keyed by agent_id.
    Nano(u32),
}

/// A typed sparse event on the Sensefield bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeuroSignal {
    pub stream: StreamId,
    /// 0 = info, 1 = warning, 2 = critical.
    pub severity: u8,
    /// Domain-specific code (caller-defined schema).
    pub code: u16,
    /// Compact payload (4 × u32 = 16 bytes).
    pub payload: [u32; 4],
    /// Observability timestamp (ns). Not used for determinism.
    pub t_ns: u64,
}

impl NeuroSignal {
    pub fn new(stream: StreamId, severity: u8, code: u16) -> Self {
        Self {
            stream,
            severity,
            code,
            payload: [0; 4],
            t_ns: 0,
        }
    }

    pub fn with_payload(mut self, payload: [u32; 4]) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_ts(mut self, t_ns: u64) -> Self {
        self.t_ns = t_ns;
        self
    }
}

/// Per-stream bounded queue.
struct StreamQueue {
    id: StreamId,
    queue: VecDeque<NeuroSignal>,
    capacity: usize,
    drops: u64,
}

impl StreamQueue {
    fn new(id: StreamId, capacity: usize) -> Self {
        Self {
            id,
            queue: VecDeque::with_capacity(capacity),
            capacity,
            drops: 0,
        }
    }

    fn push(&mut self, sig: NeuroSignal) -> bool {
        if self.queue.len() >= self.capacity {
            self.drops += 1;
            return false;
        }
        self.queue.push_back(sig);
        true
    }

    fn drain_into(&mut self, out: &mut Vec<NeuroSignal>, max: usize) {
        while out.len() < max {
            match self.queue.pop_front() {
                Some(s) => out.push(s),
                None => break,
            }
        }
    }
}

/// Multi-stream awareness bus. Single-threaded L0.
pub struct Sensefield {
    queues: Vec<StreamQueue>,
    default_capacity: usize,
}

impl Sensefield {
    pub fn new(default_capacity: usize) -> Self {
        Self {
            queues: Vec::new(),
            default_capacity,
        }
    }

    /// Register a stream with its own capacity limit. No-op if already registered.
    pub fn register(&mut self, id: StreamId, capacity: usize) {
        if !self.queues.iter().any(|q| q.id == id) {
            self.queues.push(StreamQueue::new(id, capacity));
        }
    }

    /// Push a signal. Auto-registers the stream with default capacity if needed.
    pub fn push(&mut self, sig: NeuroSignal) {
        let id = sig.stream;
        let default_cap = self.default_capacity;
        if let Some(q) = self.queues.iter_mut().find(|q| q.id == id) {
            q.push(sig);
        } else {
            let mut q = StreamQueue::new(id, default_cap);
            q.push(sig);
            self.queues.push(q);
        }
    }

    /// Drain up to `max` signals from the given stream.
    pub fn drain_stream(&mut self, id: StreamId, out: &mut Vec<NeuroSignal>, max: usize) {
        if let Some(q) = self.queues.iter_mut().find(|q| q.id == id) {
            q.drain_into(out, max);
        }
    }

    /// Drain all streams (round-robin order, up to `max` total).
    pub fn drain_all(&mut self, out: &mut Vec<NeuroSignal>, max: usize) {
        let n = self.queues.len();
        if n == 0 {
            return;
        }
        let mut i = 0;
        while out.len() < max {
            let q = &mut self.queues[i % n];
            if let Some(s) = q.queue.pop_front() {
                out.push(s);
            }
            i += 1;
            if i >= n * 2 {
                break;
            }
        }
    }

    /// Count of signals pending across all streams.
    pub fn pending(&self) -> usize {
        self.queues.iter().map(|q| q.queue.len()).sum()
    }

    /// Total drops since creation (full-queue pressure signal).
    pub fn total_drops(&self) -> u64 {
        self.queues.iter().map(|q| q.drops).sum::<u64>()
    }

    /// Registered stream count.
    pub fn stream_count(&self) -> usize {
        self.queues.len()
    }

    /// Per-stream drop count.
    pub fn stream_drops(&self, id: StreamId) -> u64 {
        self.queues
            .iter()
            .find(|q| q.id == id)
            .map(|q| q.drops)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_drain_roundtrip() {
        let mut bus = Sensefield::new(8);
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 1));
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 2));
        let mut out = Vec::new();
        bus.drain_stream(StreamId::Structural, &mut out, 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].code, 1);
        assert_eq!(out[1].code, 2);
    }

    #[test]
    fn bounded_queue_drops_on_full() {
        let mut bus = Sensefield::new(2);
        bus.register(StreamId::Immune, 2);
        for i in 0..5 {
            bus.push(NeuroSignal::new(StreamId::Immune, 0, i));
        }
        assert_eq!(bus.stream_drops(StreamId::Immune), 3);
        assert_eq!(bus.total_drops(), 3);
    }

    #[test]
    fn drain_all_round_robins_streams() {
        let mut bus = Sensefield::new(8);
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 10));
        bus.push(NeuroSignal::new(StreamId::Biological, 0, 20));
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 11));
        let mut out = Vec::new();
        bus.drain_all(&mut out, 10);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn auto_register_on_push() {
        let mut bus = Sensefield::new(4);
        assert_eq!(bus.stream_count(), 0);
        bus.push(NeuroSignal::new(StreamId::Nano(42), 0, 1));
        assert_eq!(bus.stream_count(), 1);
    }

    #[test]
    fn pending_count() {
        let mut bus = Sensefield::new(8);
        bus.push(NeuroSignal::new(StreamId::Temporal, 0, 1));
        bus.push(NeuroSignal::new(StreamId::Temporal, 0, 2));
        assert_eq!(bus.pending(), 2);
    }
}
