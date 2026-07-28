//! Phageguard — detect and quarantine misbehaving agents (ADR 0010 §4.3.7).
//!
//! KD16 split:
//! - Deterministic detector → may set `selection_veto` in PressureMesh;
//!   quarantine class = `QuarantineClass::Deterministic`.
//! - Load detector (ring pressure, heartbeat silence) → freezes proposals only;
//!   quarantine class = `QuarantineClass::Load`.
//!
//! Phageguard reports phage events to Crown; Crown applies quarantine_agent().
//! Phage events should be logged to the ledger via `log_phage_event`.

use super::awareness_bus::{NeuroSignal, StreamId};
use super::nano_agent::{NanoTickResult, QuarantineClass};
use super::organ_live::TissueLive;
use crate::genomic::organ::Organ;

/// Configuration thresholds for Phageguard detection.
#[derive(Clone, Debug)]
pub struct ImmuneConfig {
    /// Safety ≤ this → Deterministic quarantine (KD16). Default 0.0.
    pub safety_floor: f32,
    /// Utility below this for two consecutive ticks → Load quarantine.
    pub utility_floor: f32,
    /// Heartbeat silence for this many ticks → Load quarantine (not yet wired in L0).
    pub heartbeat_silence_ticks: u32,
}

impl Default for ImmuneConfig {
    fn default() -> Self {
        Self {
            safety_floor: 0.0,
            utility_floor: 0.05,
            heartbeat_silence_ticks: 10,
        }
    }
}

/// A recorded immune event (quarantine action taken).
#[derive(Clone, Debug)]
pub struct PhageEvent {
    pub agent_id: u32,
    pub class: QuarantineClass,
    pub reason: String,
    pub tick: u64,
}

/// Immune guard: inspects each tick's results and emits quarantine directives.
pub struct Phageguard {
    pub config: ImmuneConfig,
    events: Vec<PhageEvent>,
    tick: u64,
    /// Per-agent consecutive low-utility count (for Load detection).
    low_utility_streak: std::collections::HashMap<u32, u32>,
}

impl Phageguard {
    pub fn new(config: ImmuneConfig) -> Self {
        Self {
            config,
            events: Vec::new(),
            tick: 0,
            low_utility_streak: std::collections::HashMap::new(),
        }
    }

    /// Inspect one round of tick results. Returns quarantine directives to apply.
    /// `agent_ids` must align index-wise with `results`.
    pub fn inspect(
        &mut self,
        results: &[NanoTickResult],
        agent_ids: &[u32],
    ) -> Vec<PhageEvent> {
        self.tick += 1;
        let mut new_events = Vec::new();

        for (result, &agent_id) in results.iter().zip(agent_ids.iter()) {
            let utility = result.fitness.utility();

            // Deterministic: safety at/below floor (KD16).
            if result.fitness.safety <= self.config.safety_floor {
                let event = PhageEvent {
                    agent_id,
                    class: QuarantineClass::Deterministic,
                    reason: format!(
                        "safety={:.3} ≤ floor={:.3}",
                        result.fitness.safety, self.config.safety_floor
                    ),
                    tick: self.tick,
                };
                self.events.push(event.clone());
                new_events.push(event);
                // Reset streak — agent is Deterministically frozen.
                self.low_utility_streak.remove(&agent_id);
                continue;
            }

            // Load: consecutive low utility (ring pressure proxy for L0).
            if utility < self.config.utility_floor {
                let streak = self.low_utility_streak.entry(agent_id).or_insert(0);
                *streak += 1;
                if *streak >= 2 {
                    let event = PhageEvent {
                        agent_id,
                        class: QuarantineClass::Load,
                        reason: format!(
                            "utility={:.3} < floor={:.3} for {} consecutive ticks",
                            utility, self.config.utility_floor, streak
                        ),
                        tick: self.tick,
                    };
                    self.events.push(event.clone());
                    new_events.push(event);
                    *streak = 0; // Reset so we don't fire on every tick.
                }
            } else {
                // Healthy — clear the streak.
                self.low_utility_streak.remove(&agent_id);
            }
        }

        new_events
    }

    pub fn events(&self) -> &[PhageEvent] {
        &self.events
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }
}

impl Organ for Phageguard {
    fn kind(&self) -> &'static str {
        "phageguard"
    }
    fn approx_memory_bytes(&self) -> u64 {
        (self.events.len() * 64) as u64
    }
    fn structure_fingerprint(&self) -> u64 {
        let mut h = 0x811c9dc5u64;
        h ^= self.tick;
        h = h.rotate_left(7);
        h ^= self.events.len() as u64;
        h
    }
    fn unit_count(&self) -> usize {
        self.events.len()
    }
}

impl TissueLive for Phageguard {
    fn on_bus_signal(&mut self, sig: &NeuroSignal) {
        // Immune streams (SRC_PHAGE = 6) can carry self-report events.
        if sig.stream == StreamId::Immune {
            // payload[0]: agent_id reporting distress — increment its streak.
            let agent_id = sig.payload[0];
            if agent_id > 0 {
                *self.low_utility_streak.entry(agent_id).or_insert(0) += 1;
            }
        }
    }

    fn health(&self) -> f32 {
        // Healthy if no Deterministic events in the last tick.
        let det_in_last = self
            .events
            .iter()
            .filter(|e| e.tick == self.tick && e.class == QuarantineClass::Deterministic)
            .count();
        if det_in_last > 0 {
            0.5
        } else {
            1.0
        }
    }

    fn stress(&self) -> f32 {
        // Proportion of events that are Load-class (ring pressure).
        if self.events.is_empty() {
            return 0.0;
        }
        let load = self
            .events
            .iter()
            .filter(|e| e.class == QuarantineClass::Load)
            .count();
        (load as f32 / self.events.len() as f32).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::nano_agent::{MultiAxisFitness, NanoTickResult};

    fn make_result(safety: f32, utility_bias: f32) -> NanoTickResult {
        NanoTickResult::idle(MultiAxisFitness {
            task: utility_bias,
            bio: utility_bias,
            structural_cost: 1.0 - utility_bias,
            safety,
        })
    }

    #[test]
    fn phageguard_deterministic_on_safety_zero() {
        let mut pg = Phageguard::new(ImmuneConfig::default());
        let results = vec![make_result(0.0, 0.5)];
        let events = pg.inspect(&results, &[1]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].class, QuarantineClass::Deterministic);
    }

    #[test]
    fn phageguard_load_after_two_consecutive_low_utility() {
        // make_result(1.0, 0.01): utility=(0.01+0.01+0.01+1.0)/4 ≈ 0.2575
        // Use floor=0.3 so this is detected as low utility.
        let mut pg = Phageguard::new(ImmuneConfig {
            utility_floor: 0.3,
            ..ImmuneConfig::default()
        });
        let results = vec![make_result(1.0, 0.01)]; // utility ≈ 0.2575
        // First tick — streak = 1, no event yet.
        let e1 = pg.inspect(&results, &[5]);
        assert!(e1.is_empty());
        // Second tick — streak = 2, Load event fires.
        let e2 = pg.inspect(&results, &[5]);
        assert_eq!(e2.len(), 1);
        assert_eq!(e2[0].class, QuarantineClass::Load);
    }

    #[test]
    fn phageguard_healthy_clears_streak() {
        let mut pg = Phageguard::new(ImmuneConfig {
            utility_floor: 0.3,
            ..ImmuneConfig::default()
        });
        let bad = vec![make_result(1.0, 0.01)];
        let good = vec![make_result(1.0, 0.9)];
        pg.inspect(&bad, &[7]);
        // Good tick clears streak.
        pg.inspect(&good, &[7]);
        // Another bad tick — streak restarted, still no event (only 1 bad tick).
        let e = pg.inspect(&bad, &[7]);
        assert!(e.is_empty());
    }

    #[test]
    fn phageguard_health_drops_on_deterministic_event() {
        let mut pg = Phageguard::new(ImmuneConfig::default());
        let results = vec![make_result(0.0, 0.5)];
        pg.inspect(&results, &[2]);
        assert_eq!(pg.health(), 0.5);
    }

    #[test]
    fn phageguard_stress_from_load_events() {
        // make_result(1.0, 0.01) utility ≈ 0.2575; floor=0.3 catches it.
        let mut pg = Phageguard::new(ImmuneConfig {
            utility_floor: 0.3,
            ..ImmuneConfig::default()
        });
        let bad = vec![make_result(1.0, 0.01)];
        pg.inspect(&bad, &[9]);
        pg.inspect(&bad, &[9]); // fires Load event
        assert!(pg.stress() > 0.0);
    }
}
