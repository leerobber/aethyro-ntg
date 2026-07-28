//! Neurocyte — control handles and NanoBrain trait (VITASCALE L0, ADR 0010 §4.3.2).
//!
//! Crown owns tissues. NeurocyteHandle is a lightweight handle, not a tissue owner.
//! L0 scheduling: deterministic round-robin on one thread.

use super::awareness_bus::{NeuroSignal, Sensefield};

/// Identifies which tissue kind a neurocyte controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrganKey {
    Genomic,
    Language,
    Eye,
    Immune,
    Body,
    Custom(u16),
}

/// Live/Quarantined/Hibernating lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NanoStatus {
    Live,
    /// Quarantined by Phageguard. Class determines what is frozen.
    Quarantined(QuarantineClass),
    Hibernating,
}

/// Split quarantine classes (KD16): Load vs Deterministic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuarantineClass {
    /// Ring pressure / heartbeat silence — proposals frozen; `select_*` still runs.
    Load,
    /// Ledger/safety/utility gate fired — accompanies `selection_veto`.
    Deterministic,
}

/// Per-agent local fitness sample (observability / dashboard only in L0 — not scored into axes).
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiAxisFitness {
    pub task: f32,
    pub bio: f32,
    pub structural_cost: f32,
    pub safety: f32,
}

impl MultiAxisFitness {
    /// Simple utility readout for stress/dashboard (not used in `select_*`).
    pub fn utility(&self) -> f32 {
        (self.task + self.bio + (1.0 - self.structural_cost) + self.safety) / 4.0
    }
}

/// Full local goals (not stored in the 48-byte StateSlot — lives on the handle).
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalGoal {
    pub task_target: f32,
    pub bio_floor: f32,
    pub max_structural_cost: f32,
    pub min_safety: f32,
}

impl LocalGoal {
    /// Lossy i32 summary for StateSlot.desires field.
    /// Low 16 bits: task_target as milli-units (×1000, clamped to i16).
    /// High 16 bits: bio_floor as milli-units.
    pub fn to_desires_i32(&self) -> i32 {
        let task_mu = (self.task_target * 1000.0).clamp(-32768.0, 32767.0) as i32;
        let bio_mu = (self.bio_floor * 1000.0).clamp(-32768.0, 32767.0) as i32;
        (task_mu & 0xffff) | ((bio_mu & 0xffff) << 16)
    }
}

/// A proposed structural change — inert until Crown + SelfMod path; frozen if load-quarantined.
#[derive(Clone, Debug)]
pub struct NeuroProposal {
    pub agent_id: u32,
    pub description: String,
    pub urgency: f32,
}

/// Result of one `NanoBrain::tick` call.
#[derive(Clone, Debug)]
pub struct NanoTickResult {
    pub signals: Vec<NeuroSignal>,
    pub fitness: MultiAxisFitness,
    pub proposal: Option<NeuroProposal>,
}

impl NanoTickResult {
    pub fn idle(fitness: MultiAxisFitness) -> Self {
        Self {
            signals: Vec::new(),
            fitness,
            proposal: None,
        }
    }
}

/// Read-mostly Crown façade for one neurocyte tick.
/// Mutating methods only touch bus/handle tables — not tissue topology.
pub struct CrownView<'a> {
    pub organ_healths: &'a [(OrganKey, f32)],
    pub quarantine_table: &'a [(u32, QuarantineClass)],
    pub bus: &'a mut Sensefield,
}

impl CrownView<'_> {
    pub fn organ_health(&self, key: OrganKey) -> f32 {
        self.organ_healths
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| *h)
            .unwrap_or(1.0)
    }

    pub fn is_quarantined(&self, agent_id: u32) -> bool {
        self.quarantine_table.iter().any(|(id, _)| *id == agent_id)
    }

    pub fn quarantine_class(&self, agent_id: u32) -> Option<QuarantineClass> {
        self.quarantine_table
            .iter()
            .find(|(id, _)| *id == agent_id)
            .map(|(_, c)| *c)
    }

    pub fn push_signal(&mut self, sig: NeuroSignal) {
        self.bus.push(sig);
    }
}

/// Trait implemented by all controlled agents (NanoBrain = control automaton).
pub trait NanoBrain {
    fn agent_id(&self) -> u32;
    fn organ_key(&self) -> OrganKey;

    /// Single deterministic tick. May push signals to bus; may return a proposal.
    fn tick(&mut self, view: &mut CrownView<'_>) -> NanoTickResult;

    /// Local fitness sample (observability only in L0).
    fn local_fitness(&self) -> MultiAxisFitness;
}

/// Lightweight control handle owned by Crown. Does not own tissue data.
#[derive(Debug)]
pub struct NeurocyteHandle {
    pub agent_id: u32,
    pub organ_key: OrganKey,
    pub generation: u32,
    pub last_fitness: MultiAxisFitness,
    pub status: NanoStatus,
    /// Lossy summary for StateSlot.desires only.
    pub desires_summary: i32,
    /// Authoritative goals live here (not in StateSlot).
    pub goal: LocalGoal,
}

impl NeurocyteHandle {
    pub fn new(agent_id: u32, organ_key: OrganKey) -> Self {
        Self {
            agent_id,
            organ_key,
            generation: 0,
            last_fitness: MultiAxisFitness::default(),
            status: NanoStatus::Live,
            desires_summary: 0,
            goal: LocalGoal::default(),
        }
    }

    pub fn is_live(&self) -> bool {
        self.status == NanoStatus::Live
    }

    pub fn quarantine(&mut self, class: QuarantineClass) {
        self.status = NanoStatus::Quarantined(class);
    }

    pub fn restore(&mut self) {
        self.status = NanoStatus::Live;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::awareness_bus::StreamId;

    struct PingBrain {
        id: u32,
        ticks: u32,
    }

    impl NanoBrain for PingBrain {
        fn agent_id(&self) -> u32 {
            self.id
        }
        fn organ_key(&self) -> OrganKey {
            OrganKey::Genomic
        }
        fn tick(&mut self, view: &mut CrownView<'_>) -> NanoTickResult {
            self.ticks += 1;
            view.push_signal(
                NeuroSignal::new(StreamId::Nano(self.id), 0, self.ticks as u16)
                    .with_payload([self.ticks, 0, 0, 0]),
            );
            NanoTickResult::idle(self.local_fitness())
        }
        fn local_fitness(&self) -> MultiAxisFitness {
            MultiAxisFitness {
                task: 0.9,
                bio: 0.8,
                structural_cost: 0.1,
                safety: 1.0,
            }
        }
    }

    #[test]
    fn nano_tick_pushes_signal() {
        let mut brain = PingBrain { id: 1, ticks: 0 };
        let healths = [(OrganKey::Genomic, 1.0f32)];
        let quarantine: Vec<(u32, QuarantineClass)> = vec![];
        let mut bus = Sensefield::new(16);
        let mut view = CrownView {
            organ_healths: &healths,
            quarantine_table: &quarantine,
            bus: &mut bus,
        };
        let result = brain.tick(&mut view);
        assert_eq!(result.fitness.task, 0.9);
        assert_eq!(bus.pending(), 1);
    }

    #[test]
    fn handle_quarantine_restore() {
        let mut h = NeurocyteHandle::new(42, OrganKey::Language);
        assert!(h.is_live());
        h.quarantine(QuarantineClass::Load);
        assert!(!h.is_live());
        assert_eq!(h.status, NanoStatus::Quarantined(QuarantineClass::Load));
        h.restore();
        assert!(h.is_live());
    }

    #[test]
    fn local_goal_desires_roundtrip() {
        let goal = LocalGoal {
            task_target: 0.8,
            bio_floor: 0.5,
            max_structural_cost: 0.3,
            min_safety: 0.9,
        };
        let d = goal.to_desires_i32();
        // low 16: 800, high 16: 500
        let task_mu = d & 0xffff;
        let bio_mu = (d >> 16) & 0xffff;
        assert_eq!(task_mu, 800);
        assert_eq!(bio_mu, 500);
    }

    #[test]
    fn multi_axis_utility() {
        let f = MultiAxisFitness {
            task: 1.0,
            bio: 1.0,
            structural_cost: 0.0,
            safety: 1.0,
        };
        assert!((f.utility() - 1.0).abs() < 1e-6);
    }
}
