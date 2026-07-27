//! Self-awareness probe — aggregates per-tick observability into a SenseReport
//! and drives the self-healing feedback loop (ADR 0011 §5).
//!
//! SelfAwarenessProbe is owned by the caller (typically the same process that
//! owns Crown) and consumes the output of Crown::tick_all() + Phageguard::inspect()
//! to produce a SenseReport each tick.
//!
//! Self-healing actions:
//!   - Quarantine: delegate to Crown::quarantine_agent (Phageguard already decided)
//!   - Restore: restore an agent once its safety/utility recover
//!   - Bus pressure: logged as a drop-rate signal; Crown is not stalled (L0 has no
//!     backpressure mechanism — full queues are the signal)

use super::awareness_bus::Sensefield;
use super::immune_organ::PhageEvent;
use super::nano_agent::{MultiAxisFitness, NanoTickResult, QuarantineClass};
use super::oculus_organ::AwarenessFrame;

/// Per-tick snapshot of the system's self-model.
#[derive(Clone, Debug, Default)]
pub struct SenseReport {
    pub tick: u64,
    /// Mean utility across all agents this tick.
    pub mean_utility: f32,
    /// Minimum safety score observed this tick.
    pub min_safety: f32,
    /// Proportion of agents in any quarantine state.
    pub quarantine_fraction: f32,
    /// Sensefield drop rate this tick (drops / total pushes attempted).
    pub bus_drop_rate: f32,
    /// Oculus coverage (streams on SLA / registered streams).
    pub awareness_coverage: f32,
    /// Count of new phage events this tick.
    pub phage_events_this_tick: usize,
    /// Running total phage events since probe creation.
    pub phage_events_total: usize,
    /// True if any Deterministic quarantine fired this tick.
    pub deterministic_alarm: bool,
}

impl SenseReport {
    /// Overall system health score [0, 1].
    ///
    /// Composite: utility (40%) + safety (40%) + coverage (20%).
    /// Penalised by quarantine fraction and deterministic alarms.
    pub fn health_score(&self) -> f32 {
        let base = 0.4 * self.mean_utility
            + 0.4 * self.min_safety
            + 0.2 * self.awareness_coverage;
        let penalty = if self.deterministic_alarm { 0.3 } else { 0.0 }
            + self.quarantine_fraction * 0.2;
        (base - penalty).clamp(0.0, 1.0)
    }

    /// True if the system should trigger a self-healing response.
    pub fn needs_healing(&self) -> bool {
        self.deterministic_alarm
            || self.quarantine_fraction > 0.5
            || self.min_safety <= 0.0
            || self.bus_drop_rate > 0.3
    }
}

/// Self-awareness probe: integrates organ outputs into SenseReports.
pub struct SelfAwarenessProbe {
    tick: u64,
    total_phage: usize,
    /// Rolling bus-drop baseline (accumulated total at last tick start).
    last_total_drops: u64,
    /// Rolling total-pushes proxy: incremented by pending count observed.
    total_pushes_proxy: u64,
    history: Vec<SenseReport>,
    /// Max history depth (bounded memory).
    history_cap: usize,
}

impl SelfAwarenessProbe {
    pub fn new(history_cap: usize) -> Self {
        Self {
            tick: 0,
            total_phage: 0,
            last_total_drops: 0,
            total_pushes_proxy: 0,
            history: Vec::with_capacity(history_cap.min(256)),
            history_cap,
        }
    }

    /// Produce a SenseReport from one tick's outputs.
    ///
    /// `quarantined_count` and `total_agents` come from Crown's handle table.
    pub fn sense(
        &mut self,
        results: &[NanoTickResult],
        phage_events: &[PhageEvent],
        oculus_frame: Option<&AwarenessFrame>,
        bus: &Sensefield,
        quarantined_count: usize,
        total_agents: usize,
    ) -> SenseReport {
        self.tick += 1;

        let fitness_iter = results.iter().map(|r| r.fitness);
        let (mean_u, min_s) = aggregate_fitness(fitness_iter, results.len());

        let new_drops = bus.total_drops().saturating_sub(self.last_total_drops);
        self.last_total_drops = bus.total_drops();
        // Pending signals + new drops approximates total pushes this tick.
        let push_estimate = bus.pending() as u64 + new_drops;
        self.total_pushes_proxy += push_estimate;
        let drop_rate = if push_estimate > 0 {
            new_drops as f32 / push_estimate as f32
        } else {
            0.0
        };

        let det_alarm = phage_events
            .iter()
            .any(|e| e.class == QuarantineClass::Deterministic);

        self.total_phage += phage_events.len();

        let report = SenseReport {
            tick: self.tick,
            mean_utility: mean_u,
            min_safety: min_s,
            quarantine_fraction: if total_agents > 0 {
                quarantined_count as f32 / total_agents as f32
            } else {
                0.0
            },
            bus_drop_rate: drop_rate,
            awareness_coverage: oculus_frame.map(|f| f.coverage).unwrap_or(1.0),
            phage_events_this_tick: phage_events.len(),
            phage_events_total: self.total_phage,
            deterministic_alarm: det_alarm,
        };

        if self.history.len() >= self.history_cap {
            self.history.remove(0);
        }
        self.history.push(report.clone());

        report
    }

    pub fn history(&self) -> &[SenseReport] {
        &self.history
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Rolling mean health over the retained history window.
    pub fn rolling_health(&self) -> f32 {
        if self.history.is_empty() {
            return 1.0;
        }
        let sum: f32 = self.history.iter().map(|r| r.health_score()).sum();
        sum / self.history.len() as f32
    }

    /// True if rolling health has been below threshold for the given number
    /// of consecutive recent ticks.
    pub fn sustained_degradation(&self, threshold: f32, window: usize) -> bool {
        if self.history.len() < window {
            return false;
        }
        self.history[self.history.len() - window..]
            .iter()
            .all(|r| r.health_score() < threshold)
    }
}

fn aggregate_fitness(
    mut iter: impl Iterator<Item = MultiAxisFitness>,
    count: usize,
) -> (f32, f32) {
    if count == 0 {
        return (0.0, 1.0);
    }
    let first = iter.next().unwrap();
    let mut sum_u = first.utility();
    let mut min_s = first.safety;
    for f in iter {
        sum_u += f.utility();
        if f.safety < min_s {
            min_s = f.safety;
        }
    }
    (sum_u / count as f32, min_s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::{
        awareness_bus::{NeuroSignal, Sensefield, StreamId},
        immune_organ::PhageEvent,
        nano_agent::{MultiAxisFitness, NanoTickResult, QuarantineClass},
    };

    fn make_result(task: f32, safety: f32) -> NanoTickResult {
        NanoTickResult::idle(MultiAxisFitness {
            task,
            bio: task,
            structural_cost: 1.0 - task,
            safety,
        })
    }

    fn empty_bus() -> Sensefield {
        Sensefield::new(16)
    }

    #[test]
    fn sense_healthy_agents() {
        let mut probe = SelfAwarenessProbe::new(10);
        let results = vec![make_result(0.9, 1.0), make_result(0.8, 1.0)];
        let report = probe.sense(&results, &[], None, &empty_bus(), 0, 2);
        assert!(report.mean_utility > 0.7);
        assert_eq!(report.min_safety, 1.0);
        assert!(!report.deterministic_alarm);
        assert!(report.health_score() > 0.7);
        assert!(!report.needs_healing());
    }

    #[test]
    fn sense_deterministic_alarm() {
        let mut probe = SelfAwarenessProbe::new(10);
        let results = vec![make_result(0.5, 0.0)];
        let phage = vec![PhageEvent {
            agent_id: 1,
            class: QuarantineClass::Deterministic,
            reason: "safety=0".into(),
            tick: 1,
        }];
        let report = probe.sense(&results, &phage, None, &empty_bus(), 1, 1);
        assert!(report.deterministic_alarm);
        assert!(report.needs_healing());
        assert!(report.health_score() < 0.8);
    }

    #[test]
    fn health_score_composite() {
        let mut probe = SelfAwarenessProbe::new(10);
        let results = vec![make_result(1.0, 1.0)];
        let report = probe.sense(&results, &[], None, &empty_bus(), 0, 1);
        // utility=1, safety=1, coverage=1 → base = 0.4+0.4+0.2 = 1.0, no penalty
        assert!((report.health_score() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn bus_drop_rate_detected() {
        let mut probe = SelfAwarenessProbe::new(10);
        let mut bus = Sensefield::new(1); // capacity 1 — drops on 2nd push
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 1));
        bus.push(NeuroSignal::new(StreamId::Structural, 0, 2)); // drop
        let results = vec![make_result(0.9, 1.0)];
        let report = probe.sense(&results, &[], None, &bus, 0, 1);
        assert!(report.bus_drop_rate > 0.0);
    }

    #[test]
    fn rolling_health_and_degradation() {
        let mut probe = SelfAwarenessProbe::new(10);
        let bad = vec![make_result(0.0, 0.0)];
        let bus = empty_bus();
        for _ in 0..3 {
            probe.sense(&bad, &[], None, &bus, 0, 1);
        }
        assert!(probe.rolling_health() < 0.5);
        assert!(probe.sustained_degradation(0.5, 3));
    }

    #[test]
    fn sustained_degradation_requires_full_window() {
        let mut probe = SelfAwarenessProbe::new(10);
        let bad = vec![make_result(0.0, 0.0)];
        let good = vec![make_result(1.0, 1.0)];
        let bus = empty_bus();
        probe.sense(&bad, &[], None, &bus, 0, 1);
        probe.sense(&bad, &[], None, &bus, 0, 1);
        probe.sense(&good, &[], None, &bus, 0, 1); // breaks the streak
        // window=3 requires ALL 3 recent to be bad
        assert!(!probe.sustained_degradation(0.5, 3));
    }
}
