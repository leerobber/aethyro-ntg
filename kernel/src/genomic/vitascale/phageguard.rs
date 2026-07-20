//! Phageguard — L0 immune tissue for KAIROS (detect + quarantine).
//!
//! Honest scope (VITASCALE L0):
//! - **Detect** synthetic / structured threats under Guardian drills
//! - **Quarantine** with class split (Load vs Deterministic)
//! - **selection_veto** only from Deterministic detectors
//! - Journal-facing event log (cold path); full ledger `log_phage_event` comes later
//! - **Not** claiming brain heal via StateSlots

use std::collections::BTreeMap;

/// How a detector may act on selection vs proposals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectorClass {
    /// May set selection_veto / force-reject select paths.
    Deterministic,
    /// Load/observability only — quarantine proposals; never selection_veto.
    Observability,
}

/// Why a nano/agent is quarantined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuarantineClass {
    Load,
    Deterministic,
}

/// Named threat kinds used in toddler drills and real patrol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreatKind {
    /// Simulated ring-drop / pulse storm (load).
    PulseStorm,
    /// Self-mod flipped on while still under rails (deterministic).
    SelfModArmed,
    /// Synthetic inject: hostile proposal id (deterministic drill).
    HostileProposal,
    /// Weight collapse / garbage structure signal (deterministic).
    WeightAnomaly,
    /// Clear / benign probe (must NOT quarantine).
    Benign,
}

impl ThreatKind {
    pub fn name(self) -> &'static str {
        match self {
            ThreatKind::PulseStorm => "pulse_storm",
            ThreatKind::SelfModArmed => "self_mod_armed",
            ThreatKind::HostileProposal => "hostile_proposal",
            ThreatKind::WeightAnomaly => "weight_anomaly",
            ThreatKind::Benign => "benign",
        }
    }

    pub fn detector_class(self) -> DetectorClass {
        match self {
            ThreatKind::PulseStorm => DetectorClass::Observability,
            ThreatKind::SelfModArmed
            | ThreatKind::HostileProposal
            | ThreatKind::WeightAnomaly => DetectorClass::Deterministic,
            ThreatKind::Benign => DetectorClass::Observability,
        }
    }

    /// Whether a correct drill must quarantine this threat.
    pub fn should_quarantine(self) -> bool {
        !matches!(self, ThreatKind::Benign)
    }
}

/// One immune event (journal / cold audit).
#[derive(Clone, Debug)]
pub struct PhageEvent {
    pub kind: ThreatKind,
    pub agent_id: u32,
    pub quarantined: bool,
    pub selection_veto: bool,
    pub notes: String,
}

/// Immune configuration (L0 defaults).
#[derive(Clone, Debug)]
pub struct ImmuneConfig {
    pub utility_regression_tau: f32,
    pub immune_slack: f32,
    pub max_quarantines_per_patrol: u32,
    pub ring_drop_rate_threshold: f32,
    pub weight_floor: f32,
}

impl Default for ImmuneConfig {
    fn default() -> Self {
        Self {
            utility_regression_tau: 0.05,
            immune_slack: 0.02,
            max_quarantines_per_patrol: 8,
            ring_drop_rate_threshold: 0.01,
            weight_floor: 0.05,
        }
    }
}

/// Phageguard organ — single L0 immune tissue (no empty cell-type stubs).
#[derive(Clone, Debug)]
pub struct Phageguard {
    pub config: ImmuneConfig,
    pub quarantined: BTreeMap<u32, QuarantineClass>,
    /// True only if a Deterministic detector fired this patrol.
    pub selection_veto: bool,
    pub events: Vec<PhageEvent>,
    pub threats_seen: u32,
    pub quarantines_total: u32,
    pub clears_total: u32,
    pub drills_passed: u32,
    pub drills_failed: u32,
    pub patrols: u32,
}

impl Default for Phageguard {
    fn default() -> Self {
        Self::new()
    }
}

impl Phageguard {
    pub fn new() -> Self {
        Self {
            config: ImmuneConfig::default(),
            quarantined: BTreeMap::new(),
            selection_veto: false,
            events: Vec::new(),
            threats_seen: 0,
            quarantines_total: 0,
            clears_total: 0,
            drills_passed: 0,
            drills_failed: 0,
            patrols: 0,
        }
    }

    pub fn is_quarantined(&self, agent_id: u32) -> bool {
        self.quarantined.contains_key(&agent_id)
    }

    pub fn quarantine_class(&self, agent_id: u32) -> Option<QuarantineClass> {
        self.quarantined.get(&agent_id).copied()
    }

    /// Clear one agent from quarantine.
    pub fn clear(&mut self, agent_id: u32) -> bool {
        if self.quarantined.remove(&agent_id).is_some() {
            self.clears_total = self.clears_total.saturating_add(1);
            self.events.push(PhageEvent {
                kind: ThreatKind::Benign,
                agent_id,
                quarantined: false,
                selection_veto: false,
                notes: format!("CLEAR agent={agent_id}"),
            });
            true
        } else {
            false
        }
    }

    /// Clear all quarantines (Guardian reset after successful drill).
    pub fn clear_all(&mut self) {
        let ids: Vec<u32> = self.quarantined.keys().copied().collect();
        for id in ids {
            self.clear(id);
        }
        self.selection_veto = false;
    }

    /// Patrol one injected or observed threat.
    pub fn detect_and_act(&mut self, kind: ThreatKind, agent_id: u32) -> PhageEvent {
        self.threats_seen = self.threats_seen.saturating_add(1);
        let class = kind.detector_class();
        let mut selection_veto = false;
        let mut quarantined = false;

        if kind.should_quarantine() {
            let qclass = match class {
                DetectorClass::Deterministic => {
                    selection_veto = true;
                    QuarantineClass::Deterministic
                }
                DetectorClass::Observability => QuarantineClass::Load,
            };
            if self.quarantined.len() < self.config.max_quarantines_per_patrol as usize
                || self.quarantined.contains_key(&agent_id)
            {
                self.quarantined.insert(agent_id, qclass);
                quarantined = true;
                self.quarantines_total = self.quarantines_total.saturating_add(1);
            }
            if selection_veto {
                self.selection_veto = true;
            }
        }

        let ev = PhageEvent {
            kind,
            agent_id,
            quarantined,
            selection_veto,
            notes: format!(
                "phage_{} agent={} q={} veto={}",
                kind.name(),
                agent_id,
                quarantined,
                selection_veto
            ),
        };
        self.events.push(ev.clone());
        // Cap cold log growth
        if self.events.len() > 256 {
            let drop_n = self.events.len() - 256;
            self.events.drain(0..drop_n);
        }
        ev
    }

    /// Observability patrol from live vitals (ring drops vs pushes).
    pub fn patrol_vitals(&mut self, pushes: u64, drops: u64, agent_id: u32) -> Option<PhageEvent> {
        self.patrols = self.patrols.saturating_add(1);
        if pushes == 0 {
            return None;
        }
        let rate = drops as f32 / pushes as f32;
        if rate >= self.config.ring_drop_rate_threshold && drops > 0 {
            Some(self.detect_and_act(ThreatKind::PulseStorm, agent_id))
        } else {
            None
        }
    }

    /// Deterministic check: self-mod must stay off under rails.
    pub fn patrol_self_mod(&mut self, self_mod_enabled: bool, agent_id: u32) -> Option<PhageEvent> {
        self.patrols = self.patrols.saturating_add(1);
        if self_mod_enabled {
            Some(self.detect_and_act(ThreatKind::SelfModArmed, agent_id))
        } else {
            None
        }
    }

    /// Deterministic check: mean weight collapse.
    pub fn patrol_weight(&mut self, mean_weight: f32, agent_id: u32) -> Option<PhageEvent> {
        self.patrols = self.patrols.saturating_add(1);
        if mean_weight < self.config.weight_floor {
            Some(self.detect_and_act(ThreatKind::WeightAnomaly, agent_id))
        } else {
            None
        }
    }

    /// Supervised drill: inject threats, expect correct quarantine/veto behavior.
    /// Returns (passed, detail).
    pub fn run_drill(&mut self, scenarios: &[(ThreatKind, u32)]) -> (bool, String) {
        self.patrols = self.patrols.saturating_add(1);
        let mut ok = true;
        let mut parts = Vec::new();
        // Isolate drill state from prior quarantines for clean scoring.
        self.quarantined.clear();
        self.selection_veto = false;

        for &(kind, agent_id) in scenarios {
            let ev = self.detect_and_act(kind, agent_id);
            let expect_q = kind.should_quarantine();
            let expect_veto = matches!(kind.detector_class(), DetectorClass::Deterministic)
                && kind.should_quarantine();
            let step_ok = ev.quarantined == expect_q && ev.selection_veto == expect_veto;
            if !step_ok {
                ok = false;
            }
            parts.push(format!(
                "{}:q={}v={}{}",
                kind.name(),
                ev.quarantined as u8,
                ev.selection_veto as u8,
                if step_ok { "ok" } else { "FAIL" }
            ));
        }

        // Benign must never leave selection_veto if it was the only threat — already handled.
        // After mixed drill, clear load quarantines but keep score.
        if ok {
            self.drills_passed = self.drills_passed.saturating_add(1);
        } else {
            self.drills_failed = self.drills_failed.saturating_add(1);
        }
        // Toddler discipline: clear after successful learning pass so host isn't stuck.
        if ok {
            self.clear_all();
        }
        (ok, parts.join("; "))
    }

    /// Standard toddler drill pack (lean, not abundant).
    pub fn toddler_drill_pack() -> Vec<(ThreatKind, u32)> {
        vec![
            (ThreatKind::HostileProposal, 101),
            (ThreatKind::PulseStorm, 102),
            (ThreatKind::SelfModArmed, 103),
            (ThreatKind::Benign, 104),
            (ThreatKind::WeightAnomaly, 105),
        ]
    }

    pub fn summary_line(&self) -> String {
        format!(
            "phage threats={} q={} clears={} drills_pass={} drills_fail={} veto={} quarantined_now={}",
            self.threats_seen,
            self.quarantines_total,
            self.clears_total,
            self.drills_passed,
            self.drills_failed,
            self.selection_veto,
            self.quarantined.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drill_pack_passes() {
        let mut p = Phageguard::new();
        let (ok, detail) = p.run_drill(&Phageguard::toddler_drill_pack());
        assert!(ok, "{detail}");
        assert_eq!(p.drills_passed, 1);
        assert!(p.quarantined.is_empty()); // cleared after pass
    }

    #[test]
    fn deterministic_sets_veto_load_does_not_alone() {
        let mut p = Phageguard::new();
        let ev = p.detect_and_act(ThreatKind::PulseStorm, 1);
        assert!(ev.quarantined);
        assert!(!ev.selection_veto);
        assert!(!p.selection_veto);

        let ev2 = p.detect_and_act(ThreatKind::HostileProposal, 2);
        assert!(ev2.quarantined);
        assert!(ev2.selection_veto);
        assert!(p.selection_veto);
    }

    #[test]
    fn benign_not_quarantined() {
        let mut p = Phageguard::new();
        let ev = p.detect_and_act(ThreatKind::Benign, 9);
        assert!(!ev.quarantined);
        assert!(!p.is_quarantined(9));
    }
}
