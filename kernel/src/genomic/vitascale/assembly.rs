//! Crown — hierarchical tissue owner and VITASCALE L0 orchestrator (ADR 0010 §4.3.3).
//!
//! Crown owns tissues. NeurocyteHandle is the lightweight control handle (KD14).
//! Single SovereignFitnessContext prevents double-scoring (KD15).
//! Optional PulseHandles; zero cost when None (KD13).
//! L0 scheduling: deterministic round-robin on one thread.

use super::awareness_bus::{NeuroSignal, Sensefield};
use super::nano_agent::{
    CrownView, NanoBrain, NanoStatus, NanoTickResult, NeuroProposal,
    NeurocyteHandle, OrganKey, QuarantineClass,
};
use super::organ_live::TissueLive;
use super::pulsewire::PulseHandles;
use crate::genomic::sovereign_fitness::SovereignFitnessContext;

/// Summary produced after a full VITASCALE loop run.
#[derive(Debug, Default)]
pub struct GestaltReport {
    pub ticks_run: u32,
    pub signals_emitted: usize,
    pub proposals_pending: usize,
    pub mean_utility: f32,
    pub bus_drops: u64,
}

/// Hierarchical tissue owner — the single orchestration root (ADR 0010 §4.3.3).
///
/// - Owns tissue data (`Box<dyn TissueLive>`).
/// - One lightweight `NeurocyteHandle` per tissue (KD14: handle ≠ owner).
/// - Single `SovereignFitnessContext`: the only `select_*` authority (KD15).
/// - Optional `PulseHandles`: no process-global ring (KD13).
pub struct Crown {
    tissues: Vec<Box<dyn TissueLive>>,
    brains: Vec<Box<dyn NanoBrain>>,
    handles: Vec<NeurocyteHandle>,
    /// Single selection authority — no other path scores axes (KD15).
    pub ctx: SovereignFitnessContext,
    /// Zero-cost when None (KD13).
    pub pulse: Option<PulseHandles>,
    /// In-process awareness bus.
    pub bus: Sensefield,
    organ_healths: Vec<(OrganKey, f32)>,
    quarantine_table: Vec<(u32, QuarantineClass)>,
    pending_proposals: Vec<NeuroProposal>,
}

impl Crown {
    pub fn new(ctx: SovereignFitnessContext, pulse: Option<PulseHandles>) -> Self {
        Self {
            tissues: Vec::new(),
            brains: Vec::new(),
            handles: Vec::new(),
            ctx,
            pulse,
            bus: Sensefield::new(64),
            organ_healths: Vec::new(),
            quarantine_table: Vec::new(),
            pending_proposals: Vec::new(),
        }
    }

    /// Register a tissue + its control brain. Handle is created automatically.
    pub fn add_tissue(&mut self, tissue: Box<dyn TissueLive>, brain: Box<dyn NanoBrain>) {
        let handle = NeurocyteHandle::new(brain.agent_id(), brain.organ_key());
        self.organ_healths.push((brain.organ_key(), 1.0));
        self.tissues.push(tissue);
        self.handles.push(handle);
        self.brains.push(brain);
    }

    pub fn tissue_count(&self) -> usize {
        self.tissues.len()
    }

    /// Sample health from all TissueLive impls into the organ_healths table.
    pub fn sample_organ_healths(&mut self) {
        for (i, tissue) in self.tissues.iter().enumerate() {
            if let Some(entry) = self.organ_healths.get_mut(i) {
                entry.1 = tissue.health();
            }
        }
    }

    /// Single deterministic round of ticks over all live NanoBrains.
    ///
    /// Signals emitted by brains are pushed to `self.bus` during tick, then
    /// fanned out to tissue `on_bus_signal` callbacks before returning.
    pub fn tick_all(&mut self) -> Vec<NanoTickResult> {
        self.sample_organ_healths();

        // Rebuild quarantine table from live handle statuses.
        self.quarantine_table.clear();
        for h in &self.handles {
            if let NanoStatus::Quarantined(class) = h.status {
                self.quarantine_table.push((h.agent_id, class));
            }
        }

        // Snapshot inputs for CrownView (avoids split-borrow with self.bus).
        let healths_snap: Vec<(OrganKey, f32)> = self.organ_healths.clone();
        let qt_snap: Vec<(u32, QuarantineClass)> = self.quarantine_table.clone();

        let n = self.brains.len();
        let mut results: Vec<NanoTickResult> = Vec::with_capacity(n);

        for i in 0..n {
            // brains[i], handles[i], bus are distinct fields — borrow-safe.
            let mut view = CrownView {
                organ_healths: &healths_snap,
                quarantine_table: &qt_snap,
                bus: &mut self.bus,
            };
            let result = self.brains[i].tick(&mut view);
            self.handles[i].last_fitness = result.fitness;
            self.handles[i].desires_summary = self.handles[i].goal.to_desires_i32();

            // Proposals frozen under Load quarantine (KD16).
            let frozen = matches!(
                self.handles[i].status,
                NanoStatus::Quarantined(QuarantineClass::Load)
                    | NanoStatus::Quarantined(QuarantineClass::Deterministic)
            );
            if !frozen {
                if let Some(ref prop) = result.proposal {
                    self.pending_proposals.push(prop.clone());
                }
            }

            results.push(result);
        }

        // Fan-out bus signals to tissue on_bus_signal callbacks.
        let mut drained: Vec<NeuroSignal> = Vec::new();
        self.bus.drain_all(&mut drained, 4096);
        for sig in &drained {
            for tissue in self.tissues.iter_mut() {
                tissue.on_bus_signal(sig);
            }
        }
        // Re-enqueue so Oculus / external consumers can still drain.
        for sig in drained {
            self.bus.push(sig);
        }

        results
    }

    /// Run the VITASCALE loop for `n_ticks`. Returns a Gestalt summary.
    pub fn run_vitascale_loop(&mut self, n_ticks: u32) -> GestaltReport {
        let mut signals_emitted = 0usize;
        let mut utility_sum = 0.0f32;
        let mut utility_count = 0usize;

        for _ in 0..n_ticks {
            let results = self.tick_all();
            for r in &results {
                signals_emitted += r.signals.len();
                utility_sum += r.fitness.utility();
                utility_count += 1;
            }
            if let Some(ref mut ph) = self.pulse {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0);
                ph.beat(0, 0, ts);
            }
        }

        GestaltReport {
            ticks_run: n_ticks,
            signals_emitted,
            proposals_pending: self.pending_proposals.len(),
            mean_utility: if utility_count > 0 {
                utility_sum / utility_count as f32
            } else {
                0.0
            },
            bus_drops: self.bus.total_drops(),
        }
    }

    /// Take pending proposals and clear the internal buffer.
    pub fn take_proposals(&mut self) -> Vec<NeuroProposal> {
        std::mem::take(&mut self.pending_proposals)
    }

    pub fn organ_health(&self, key: OrganKey) -> f32 {
        self.organ_healths
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| *h)
            .unwrap_or(1.0)
    }

    pub fn handle(&self, agent_id: u32) -> Option<&NeurocyteHandle> {
        self.handles.iter().find(|h| h.agent_id == agent_id)
    }

    pub fn quarantine_agent(&mut self, agent_id: u32, class: QuarantineClass) {
        if let Some(h) = self.handles.iter_mut().find(|h| h.agent_id == agent_id) {
            h.quarantine(class);
        }
    }

    pub fn restore_agent(&mut self, agent_id: u32) {
        if let Some(h) = self.handles.iter_mut().find(|h| h.agent_id == agent_id) {
            h.restore();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::organ::Organ;
    use crate::genomic::sovereign_fitness::SovereignFitnessContext;
    use crate::genomic::vitascale::awareness_bus::StreamId;
    use crate::genomic::vitascale::nano_agent::MultiAxisFitness;

    // --- Minimal stub tissue ---

    struct StubOrgan {
        id: u32,
    }

    impl Organ for StubOrgan {
        fn kind(&self) -> &'static str {
            "stub"
        }
        fn approx_memory_bytes(&self) -> u64 {
            0
        }
        fn structure_fingerprint(&self) -> u64 {
            self.id as u64
        }
        fn unit_count(&self) -> usize {
            1
        }
    }

    impl TissueLive for StubOrgan {}

    // --- Minimal stub brain ---

    struct StubBrain {
        id: u32,
        ticks: u32,
    }

    impl NanoBrain for StubBrain {
        fn agent_id(&self) -> u32 {
            self.id
        }
        fn organ_key(&self) -> OrganKey {
            OrganKey::Body
        }
        fn tick(&mut self, view: &mut CrownView<'_>) -> NanoTickResult {
            self.ticks += 1;
            use crate::genomic::vitascale::awareness_bus::NeuroSignal;
            view.push_signal(
                NeuroSignal::new(StreamId::Nano(self.id), 0, self.ticks as u16)
                    .with_payload([self.ticks, 0, 0, 0]),
            );
            NanoTickResult::idle(self.local_fitness())
        }
        fn local_fitness(&self) -> MultiAxisFitness {
            MultiAxisFitness {
                task: 0.8,
                bio: 0.8,
                structural_cost: 0.1,
                safety: 1.0,
            }
        }
    }

    fn make_crown() -> Crown {
        let ctx = SovereignFitnessContext::new().unwrap();
        Crown::new(ctx, None)
    }

    #[test]
    fn crown_add_tissue_and_tick() {
        let mut crown = make_crown();
        crown.add_tissue(
            Box::new(StubOrgan { id: 1 }),
            Box::new(StubBrain { id: 1, ticks: 0 }),
        );
        assert_eq!(crown.tissue_count(), 1);
        let results = crown.tick_all();
        assert_eq!(results.len(), 1);
        assert!((results[0].fitness.task - 0.8).abs() < 1e-6);
    }

    #[test]
    fn crown_run_vitascale_loop() {
        let mut crown = make_crown();
        crown.add_tissue(
            Box::new(StubOrgan { id: 2 }),
            Box::new(StubBrain { id: 2, ticks: 0 }),
        );
        let report = crown.run_vitascale_loop(5);
        assert_eq!(report.ticks_run, 5);
        assert!(report.mean_utility > 0.0);
    }

    #[test]
    fn crown_quarantine_freezes_proposals() {
        let mut crown = make_crown();

        struct ProposingBrain {
            id: u32,
        }
        impl NanoBrain for ProposingBrain {
            fn agent_id(&self) -> u32 {
                self.id
            }
            fn organ_key(&self) -> OrganKey {
                OrganKey::Body
            }
            fn tick(&mut self, _view: &mut CrownView<'_>) -> NanoTickResult {
                NanoTickResult {
                    signals: vec![],
                    fitness: MultiAxisFitness::default(),
                    proposal: Some(NeuroProposal {
                        agent_id: self.id,
                        description: "resize buffer".into(),
                        urgency: 0.5,
                    }),
                }
            }
            fn local_fitness(&self) -> MultiAxisFitness {
                MultiAxisFitness::default()
            }
        }

        crown.add_tissue(
            Box::new(StubOrgan { id: 3 }),
            Box::new(ProposingBrain { id: 3 }),
        );
        crown.quarantine_agent(3, QuarantineClass::Load);
        let _ = crown.tick_all();
        assert!(crown.take_proposals().is_empty(), "load-quarantined proposals must be frozen");
    }

    #[test]
    fn crown_gestalt_mean_utility() {
        let mut crown = make_crown();
        crown.add_tissue(
            Box::new(StubOrgan { id: 10 }),
            Box::new(StubBrain { id: 10, ticks: 0 }),
        );
        let report = crown.run_vitascale_loop(3);
        // StubBrain utility = (0.8 + 0.8 + (1-0.1) + 1.0) / 4 = 3.5/4 = 0.875
        assert!((report.mean_utility - 0.875).abs() < 1e-4);
    }
}
