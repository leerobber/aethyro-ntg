//! PressureMesh — aggregates nano fitness reports for dashboard + immune veto (ADR 0010 §4.3.5).
//!
//! KD15: `PressureMesh` surfaces veto and ephemeral bias only.
//! KD16: `selection_veto` is set only when a Deterministic condition fires
//!       (safety=0, utility regression, test inject).
//!       Load quarantine updates `pressure` only, never blocks `select_*`.

use super::nano_agent::NanoTickResult;

/// Per-axis contribution weights for pressure aggregation (observability only).
#[derive(Clone, Copy, Debug)]
pub struct FederationWeights {
    pub task: f32,
    pub bio: f32,
    pub safety: f32,
}

impl Default for FederationWeights {
    fn default() -> Self {
        Self {
            task: 0.4,
            bio: 0.3,
            safety: 0.3,
        }
    }
}

/// Aggregated pressure signal returned by `PressureMesh::federate`.
///
/// `selection_veto=true` may only be set by Deterministic detectors (KD16).
/// Load pressure is surfaced through `mean_utility` / `stress_level` only.
#[derive(Clone, Debug)]
pub struct PressureHint {
    pub mean_utility: f32,
    pub min_safety: f32,
    /// Normalised pressure [0,1]: 1-mean_utility, clamped.
    pub stress_level: f32,
    /// True only when a Deterministic detector fires (KD16). Never from Load alone.
    pub selection_veto: bool,
    pub veto_reason: Option<String>,
}

/// Aggregates per-neurocyte fitness reports into a single PressureHint.
///
/// The mesh is *not* a selection authority (KD15). It feeds the Omniradar
/// dashboard and Phageguard attention — it does not score axes.
pub struct PressureMesh {
    pub weights: FederationWeights,
    /// Safety at or below this value → Deterministic veto (KD16).
    /// Default 0.0 means only exactly-zero safety triggers a veto.
    pub veto_threshold_safety: f32,
}

impl Default for PressureMesh {
    fn default() -> Self {
        Self {
            weights: FederationWeights::default(),
            veto_threshold_safety: 0.0,
        }
    }
}

impl PressureMesh {
    pub fn new(weights: FederationWeights, veto_threshold_safety: f32) -> Self {
        Self {
            weights,
            veto_threshold_safety,
        }
    }

    /// Aggregate nano tick results → PressureHint.
    /// `selection_veto` is set only when safety ≤ veto_threshold_safety (Deterministic).
    pub fn federate(&self, reports: &[NanoTickResult]) -> PressureHint {
        if reports.is_empty() {
            return PressureHint {
                mean_utility: 0.0,
                min_safety: 1.0,
                stress_level: 0.0,
                selection_veto: false,
                veto_reason: None,
            };
        }

        let mut utility_sum = 0.0f32;
        let mut min_safety = 1.0f32;
        let mut veto = false;
        let mut veto_reason: Option<String> = None;

        for r in reports {
            let u = r.fitness.utility();
            utility_sum += u;
            if r.fitness.safety < min_safety {
                min_safety = r.fitness.safety;
            }
            // Deterministic veto: safety at or below the configured floor (KD16).
            if r.fitness.safety <= self.veto_threshold_safety && !veto {
                veto = true;
                veto_reason = Some(format!(
                    "Deterministic: agent safety={:.3} ≤ floor={:.3}",
                    r.fitness.safety, self.veto_threshold_safety
                ));
            }
        }

        let mean_utility = utility_sum / reports.len() as f32;
        PressureHint {
            mean_utility,
            min_safety,
            stress_level: (1.0 - mean_utility).clamp(0.0, 1.0),
            selection_veto: veto,
            veto_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::nano_agent::NanoTickResult;

    fn make_result(task: f32, bio: f32, cost: f32, safety: f32) -> NanoTickResult {
        use crate::genomic::vitascale::nano_agent::MultiAxisFitness;
        NanoTickResult::idle(MultiAxisFitness {
            task,
            bio,
            structural_cost: cost,
            safety,
        })
    }

    #[test]
    fn federate_empty_returns_defaults() {
        let mesh = PressureMesh::default();
        let hint = mesh.federate(&[]);
        assert_eq!(hint.mean_utility, 0.0);
        assert_eq!(hint.min_safety, 1.0);
        assert!(!hint.selection_veto);
    }

    #[test]
    fn federate_healthy_agents_no_veto() {
        let mesh = PressureMesh::default();
        let reports = vec![
            make_result(0.9, 0.9, 0.1, 1.0),
            make_result(0.8, 0.8, 0.2, 0.9),
        ];
        let hint = mesh.federate(&reports);
        assert!(!hint.selection_veto);
        assert!(hint.mean_utility > 0.8);
    }

    #[test]
    fn federate_safety_zero_triggers_deterministic_veto() {
        let mesh = PressureMesh::default(); // veto_threshold_safety = 0.0
        let reports = vec![
            make_result(0.9, 0.9, 0.1, 1.0),
            make_result(0.5, 0.5, 0.5, 0.0), // safety=0 → Deterministic veto
        ];
        let hint = mesh.federate(&reports);
        assert!(hint.selection_veto);
        assert!(hint.veto_reason.is_some());
    }

    #[test]
    fn federate_low_utility_no_veto_load_only() {
        // Load pressure is surfaced via stress_level, not selection_veto (KD16).
        let mesh = PressureMesh::default();
        let reports = vec![
            make_result(0.1, 0.1, 0.9, 0.5), // low utility, but safety > 0
        ];
        let hint = mesh.federate(&reports);
        assert!(!hint.selection_veto, "Load pressure must never set selection_veto");
        assert!(hint.stress_level > 0.5);
    }

    #[test]
    fn federate_tracks_min_safety() {
        let mesh = PressureMesh::default();
        let reports = vec![
            make_result(1.0, 1.0, 0.0, 0.9),
            make_result(1.0, 1.0, 0.0, 0.3),
            make_result(1.0, 1.0, 0.0, 0.7),
        ];
        let hint = mesh.federate(&reports);
        assert!((hint.min_safety - 0.3).abs() < 1e-6);
    }
}
