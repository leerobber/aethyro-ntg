//! Pulsewire — binary hot-path vital ring + VitalMeters (VITASCALE Stage 0+).
//!
//! No UTF-8 / JSONL on the hot path. Fixed-size events only.

use std::sync::atomic::{AtomicU64, Ordering};

/// Source ids (stable for Omniradar / offline decode).
pub const SRC_HEARTBEAT: u8 = 1;
pub const SRC_ACTIVATE: u8 = 2;
pub const SRC_SCORE: u8 = 3;
pub const SRC_SELECT: u8 = 4;
pub const SRC_LD: u8 = 5;
pub const SRC_PHAGE: u8 = 6;
pub const SRC_OCULUS: u8 = 7;

pub const KIND_BEGIN: u8 = 1;
pub const KIND_END: u8 = 2;
pub const KIND_TICK: u8 = 3;
pub const KIND_DROP: u8 = 4;

/// 32-byte packed vital event (schema v1 in low nibble of `flags`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PulseEvent {
    pub tsc_or_ns: u64,
    pub source: u8,
    pub kind: u8,
    pub flags: u8,
    pub _pad: u8,
    pub gen: u32,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

const _: () = assert!(std::mem::size_of::<PulseEvent>() == 32);

impl PulseEvent {
    pub const SCHEMA_V1: u8 = 1;

    pub fn schema_version(&self) -> u8 {
        self.flags & 0x0f
    }

    pub fn with_schema(mut self, ver: u8) -> Self {
        self.flags = (self.flags & 0xf0) | (ver & 0x0f);
        self
    }

    pub fn heartbeat(gen: u32, tick: u32, ns: u64) -> Self {
        Self {
            tsc_or_ns: ns,
            source: SRC_HEARTBEAT,
            kind: KIND_TICK,
            flags: Self::SCHEMA_V1,
            _pad: 0,
            gen,
            a: tick,
            b: 0,
            c: 0,
        }
    }
}

/// Single-producer ring (L0: one thread). Drop-on-full increments VitalMeters.
#[derive(Debug)]
pub struct Pulsewire {
    buf: Box<[PulseEvent]>,
    head: usize,
    tail: usize,
    mask: usize,
}

impl Pulsewire {
    /// Capacity rounded up to power of two, minimum 8.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(8).next_power_of_two();
        Self {
            buf: vec![PulseEvent::default(); cap].into_boxed_slice(),
            head: 0,
            tail: 0,
            mask: cap - 1,
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn len(&self) -> usize {
        self.head.wrapping_sub(self.tail)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    /// Push; returns false if full (caller should record drop).
    #[inline]
    pub fn try_push(&mut self, ev: PulseEvent) -> bool {
        let next = self.head.wrapping_add(1);
        if next.wrapping_sub(self.tail) > self.mask {
            return false;
        }
        self.buf[self.head & self.mask] = ev;
        self.head = next;
        true
    }

    #[inline]
    pub fn try_pop(&mut self) -> Option<PulseEvent> {
        if self.is_empty() {
            return None;
        }
        let ev = self.buf[self.tail & self.mask];
        self.tail = self.tail.wrapping_add(1);
        Some(ev)
    }

    /// Drain up to `max` events into `out`.
    pub fn drain_into(&mut self, out: &mut Vec<PulseEvent>, max: usize) {
        while out.len() < max {
            match self.try_pop() {
                Some(ev) => out.push(ev),
                None => break,
            }
        }
    }
}

/// Atomic vital counters (safe to share; L0 often single-thread updates).
#[derive(Debug, Default)]
pub struct VitalMeters {
    pub heartbeats: AtomicU64,
    pub pushes: AtomicU64,
    pub drops: AtomicU64,
    pub activate_calls: AtomicU64,
    pub score_calls: AtomicU64,
    pub select_calls: AtomicU64,
    pub ld_samples: AtomicU64,
}

impl VitalMeters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> VitalSnapshot {
        VitalSnapshot {
            heartbeats: self.heartbeats.load(Ordering::Relaxed),
            pushes: self.pushes.load(Ordering::Relaxed),
            drops: self.drops.load(Ordering::Relaxed),
            activate_calls: self.activate_calls.load(Ordering::Relaxed),
            score_calls: self.score_calls.load(Ordering::Relaxed),
            select_calls: self.select_calls.load(Ordering::Relaxed),
            ld_samples: self.ld_samples.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VitalSnapshot {
    pub heartbeats: u64,
    pub pushes: u64,
    pub drops: u64,
    pub activate_calls: u64,
    pub score_calls: u64,
    pub select_calls: u64,
    pub ld_samples: u64,
}

/// Optional handles for instrumented paths (no globals).
#[derive(Debug)]
pub struct PulseHandles {
    pub ring: Pulsewire,
    pub meters: VitalMeters,
}

impl PulseHandles {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            ring: Pulsewire::new(ring_capacity),
            meters: VitalMeters::new(),
        }
    }

    /// Push heartbeat; always bumps meters.
    pub fn beat(&mut self, gen: u32, tick: u32, ns: u64) {
        self.meters.heartbeats.fetch_add(1, Ordering::Relaxed);
        let ev = PulseEvent::heartbeat(gen, tick, ns);
        if self.ring.try_push(ev) {
            self.meters.pushes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.meters.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn push(&mut self, ev: PulseEvent) {
        if self.ring.try_push(ev) {
            self.meters.pushes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.meters.drops.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_is_32_bytes() {
        assert_eq!(std::mem::size_of::<PulseEvent>(), 32);
    }

    #[test]
    fn ring_push_pop_roundtrip() {
        let mut r = Pulsewire::new(16);
        let ev = PulseEvent::heartbeat(1, 7, 99);
        assert!(r.try_push(ev));
        assert_eq!(r.try_pop(), Some(ev));
        assert!(r.is_empty());
    }

    #[test]
    fn ring_drop_on_full() {
        // Classic SPSC: capacity N power-of-two ⇒ N-1 usable slots.
        let mut r = Pulsewire::new(8);
        let mut pushed = 0u32;
        let mut failed = 0u32;
        for i in 0..32 {
            if r.try_push(PulseEvent::heartbeat(0, i, 0)) {
                pushed += 1;
            } else {
                failed += 1;
            }
        }
        // cap 8 ⇒ 7 usable SPSC slots
        assert_eq!(pushed, 7);
        assert_eq!(failed, 25);
    }

    #[test]
    fn beat_increments_vitals() {
        let mut h = PulseHandles::new(32);
        h.beat(0, 1, 1000);
        h.beat(0, 2, 2000);
        let s = h.meters.snapshot();
        assert_eq!(s.heartbeats, 2);
        assert_eq!(s.pushes, 2);
        assert_eq!(s.drops, 0);
    }
}
