//! Rung 2: multi-axis fitness for sovereign / genomic + kernel evolution.
//!
//! Extends the dual (latency, memory) graph fitness with axes that make
//! self-modification grade *intelligence-relevant* progress:
//!
//! | Axis | Direction | Meaning |
//! |------|-----------|---------|
//! | `task_accuracy` | ↑ better | Task / schooling / agent score in [0, 1] |
//! | `structural_cost` | ↓ better | Normalized cost from size/latency/memory |
//! | `biological_consistency` | ↑ better | Validation / LD-structure fidelity [0, 1] |
//! | `safety` | ↑ better | Ledger/budget/health in [0, 1] |
//!
//! Existing [`FitnessScore`] remains the graph micro-bench primitive;
//! this type is the cross-wired selection signal for SovereignBrain.

use crate::genomic::sovereign_brain::{SovereignBrain, StructuralMetrics};
use super::evaluator::FitnessScore;

/// Four-axis fitness used for Rung 2 selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultiAxisFitness {
    /// Higher is better, expected in [0, 1].
    pub task_accuracy: f32,
    /// Lower is better, expected in [0, 1] after normalization.
    pub structural_cost: f32,
    /// Higher is better, expected in [0, 1].
    pub biological_consistency: f32,
    /// Higher is better, expected in [0, 1] (1 = fully safe).
    pub safety: f32,
}

impl Default for MultiAxisFitness {
    fn default() -> Self {
        Self {
            task_accuracy: 0.0,
            structural_cost: 1.0,
            biological_consistency: 0.0,
            safety: 1.0,
        }
    }
}

impl MultiAxisFitness {
    pub fn new(
        task_accuracy: f32,
        structural_cost: f32,
        biological_consistency: f32,
        safety: f32,
    ) -> Self {
        Self {
            task_accuracy: task_accuracy.clamp(0.0, 1.0),
            structural_cost: structural_cost.clamp(0.0, 1.0),
            biological_consistency: biological_consistency.clamp(0.0, 1.0),
            safety: safety.clamp(0.0, 1.0),
        }
    }

    /// Build from kernel graph micro-fitness + domain axes.
    ///
    /// Maps latency/memory into structural_cost via simple saturating norms.
    pub fn from_graph_and_domain(
        graph: FitnessScore,
        task_accuracy: f32,
        biological_consistency: f32,
        safety: f32,
    ) -> Self {
        // 10ms latency → cost 1.0; 8MB memory → cost 1.0 (soft caps).
        let lat_cost = (graph.latency_us as f32 / 10_000.0).min(1.0);
        let mem_cost = (graph.memory_bytes as f32 / 8_000_000.0).min(1.0);
        let structural_cost = 0.7 * lat_cost + 0.3 * mem_cost;
        Self::new(
            task_accuracy,
            structural_cost,
            biological_consistency,
            safety,
        )
    }

    /// Build from SovereignBrain structural metrics + external scores.
    pub fn from_sovereign(
        structure: &StructuralMetrics,
        task_accuracy: f32,
        biological_consistency: f32,
        safety: f32,
    ) -> Self {
        // Normalize structural cost: reward compact high-weight graphs.
        let mem_cost = (structure.approx_memory_bytes as f32 / 4_000_000.0).min(1.0);
        let size_cost = ((structure.n_synapses as f32) / 50_000.0).min(1.0);
        // Low mean weight ⇒ poorly integrated structure ⇒ higher cost.
        let integration_penalty = 1.0 - structure.mean_synapse_weight.clamp(0.0, 1.0);
        let structural_cost = (0.4 * mem_cost + 0.3 * size_cost + 0.3 * integration_penalty)
            .clamp(0.0, 1.0);
        Self::new(
            task_accuracy,
            structural_cost,
            biological_consistency,
            safety,
        )
    }

    /// Scalar utility in [0, 1], **higher is better**.
    ///
    /// Weights: task 35%, biology 25%, safety 25%, structure 15%
    /// (structure inverted so lower cost raises score).
    pub fn utility(&self) -> f32 {
        let structure_good = 1.0 - self.structural_cost;
        (0.35 * self.task_accuracy
            + 0.25 * self.biological_consistency
            + 0.25 * self.safety
            + 0.15 * structure_good)
            .clamp(0.0, 1.0)
    }

    /// Weak Pareto: self dominates other if every maximize-axis is ≥ and
    /// structural_cost is ≤, with at least one strict improvement.
    pub fn dominates(&self, other: &Self) -> bool {
        let ge_task = self.task_accuracy >= other.task_accuracy;
        let ge_bio = self.biological_consistency >= other.biological_consistency;
        let ge_safe = self.safety >= other.safety;
        let le_cost = self.structural_cost <= other.structural_cost;
        let all = ge_task && ge_bio && ge_safe && le_cost;
        let strict = self.task_accuracy > other.task_accuracy
            || self.biological_consistency > other.biological_consistency
            || self.safety > other.safety
            || self.structural_cost < other.structural_cost;
        all && strict
    }

    /// Accept candidate over baseline if utility improves by `min_delta`
    /// **and** safety does not drop, **and** biological_consistency does
    /// not drop by more than `bio_slack`.
    pub fn accept_candidate(&self, candidate: &Self, min_delta: f32, bio_slack: f32) -> bool {
        if candidate.safety + 1e-6 < self.safety {
            return false; // never accept safety regression
        }
        if candidate.biological_consistency + bio_slack + 1e-6 < self.biological_consistency {
            return false; // biology cannot collapse
        }
        candidate.utility() + 1e-6 >= self.utility() + min_delta
    }
}

/// Evaluates multi-axis fitness for sovereign brains and graph scores.
#[derive(Clone, Debug)]
pub struct MultiAxisEvaluator {
    /// Minimum utility gain to accept a mutant (default 0.01 = 1%).
    pub min_utility_delta: f32,
    /// Allowed biology drop when other axes improve (default 0.02).
    pub bio_slack: f32,
}

impl Default for MultiAxisEvaluator {
    fn default() -> Self {
        Self {
            min_utility_delta: 0.01,
            bio_slack: 0.02,
        }
    }
}

impl MultiAxisEvaluator {
    pub fn new(min_utility_delta: f32, bio_slack: f32) -> Self {
        Self {
            min_utility_delta,
            bio_slack,
        }
    }

    /// Score a sovereign brain given external task/biology/safety signals.
    pub fn score_sovereign(
        &self,
        brain: &SovereignBrain,
        task_accuracy: f32,
        biological_consistency: f32,
        safety: f32,
    ) -> MultiAxisFitness {
        let structure = brain.measure_structure();
        MultiAxisFitness::from_sovereign(
            &structure,
            task_accuracy,
            biological_consistency,
            safety,
        )
    }

    /// Decide acceptance.
    pub fn should_accept(&self, baseline: &MultiAxisFitness, candidate: &MultiAxisFitness) -> bool {
        baseline.accept_candidate(candidate, self.min_utility_delta, self.bio_slack)
    }

    /// Run one selection step: propose prune mutant, score both, accept if better.
    ///
    /// `task_fn` / `bio_fn` compute task and biology scores for a brain state
    /// (kept as closures so calib / Phase E can plug in later without coupling).
    pub fn select_prune_step<FTask, FBio>(
        &self,
        parent: &SovereignBrain,
        prune_frac: f32,
        safety: f32,
        mut task_fn: FTask,
        mut bio_fn: FBio,
    ) -> SelectionOutcome
    where
        FTask: FnMut(&SovereignBrain) -> f32,
        FBio: FnMut(&SovereignBrain) -> f32,
    {
        let baseline = self.score_sovereign(parent, task_fn(parent), bio_fn(parent), safety);
        let child = parent.propose_prune_mutant(prune_frac);
        let cand = self.score_sovereign(&child, task_fn(&child), bio_fn(&child), safety);
        let accepted = self.should_accept(&baseline, &cand);
        SelectionOutcome {
            accepted,
            baseline,
            candidate: cand,
            child: if accepted { Some(child) } else { None },
        }
    }
}

/// Result of a multi-axis selection step.
#[derive(Clone, Debug)]
pub struct SelectionOutcome {
    pub accepted: bool,
    pub baseline: MultiAxisFitness,
    pub candidate: MultiAxisFitness,
    pub child: Option<SovereignBrain>,
}

/// Proxy task score from structure: prefers high mean synapse weight and
/// non-empty working set potential (used when no external task is wired yet).
pub fn proxy_task_from_structure(brain: &SovereignBrain) -> f32 {
    let s = brain.measure_structure();
    if s.n_synapses == 0 {
        return 0.0;
    }
    let weight = s.mean_synapse_weight.clamp(0.0, 1.0);
    let coverage = ((s.n_neurons as f32).ln() / 12.0).clamp(0.0, 1.0);
    (0.7 * weight + 0.3 * coverage).clamp(0.0, 1.0)
}

/// Proxy biology score: mean block r² + LTM motif presence (structure fidelity).
pub fn proxy_biology_from_structure(brain: &SovereignBrain) -> f32 {
    let s = brain.measure_structure();
    let r2 = s.mean_block_r2.clamp(0.0, 1.0);
    let ltm = if s.n_ltm_motifs > 0 { 1.0 } else { 0.0 };
    (0.8 * r2 + 0.2 * ltm).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::sovereign_brain::synthetic_test_brain;

    #[test]
    fn utility_prefers_better_task() {
        let a = MultiAxisFitness::new(0.5, 0.4, 0.5, 1.0);
        let b = MultiAxisFitness::new(0.8, 0.4, 0.5, 1.0);
        assert!(b.utility() > a.utility());
    }

    #[test]
    fn safety_regression_never_accepted() {
        let base = MultiAxisFitness::new(0.5, 0.5, 0.5, 1.0);
        let bad = MultiAxisFitness::new(0.99, 0.1, 0.99, 0.5);
        assert!(!base.accept_candidate(&bad, 0.01, 0.05));
    }

    #[test]
    fn biology_collapse_rejected() {
        let base = MultiAxisFitness::new(0.5, 0.5, 0.9, 1.0);
        let bad = MultiAxisFitness::new(0.9, 0.2, 0.5, 1.0); // bio drop 0.4
        assert!(!base.accept_candidate(&bad, 0.01, 0.02));
    }

    #[test]
    fn dominates_requires_strict_improvement() {
        let a = MultiAxisFitness::new(0.5, 0.5, 0.5, 1.0);
        let b = MultiAxisFitness::new(0.5, 0.5, 0.5, 1.0);
        assert!(!a.dominates(&b));
        let c = MultiAxisFitness::new(0.6, 0.5, 0.5, 1.0);
        assert!(c.dominates(&a));
    }

    #[test]
    fn score_sovereign_is_finite() {
        let brain = synthetic_test_brain();
        let ev = MultiAxisEvaluator::default();
        let f = ev.score_sovereign(&brain, 0.7, 0.8, 1.0);
        assert!(f.utility().is_finite());
        assert!((0.0..=1.0).contains(&f.utility()));
    }

    #[test]
    fn select_prune_step_runs() {
        let parent = synthetic_test_brain();
        let ev = MultiAxisEvaluator::new(0.0, 0.1); // accept any non-regression
        let out = ev.select_prune_step(
            &parent,
            0.2,
            1.0,
            proxy_task_from_structure,
            proxy_biology_from_structure,
        );
        // Whether accepted depends on costs; both scores must be valid.
        assert!(out.baseline.utility().is_finite());
        assert!(out.candidate.utility().is_finite());
        if out.accepted {
            assert!(out.child.is_some());
        }
    }

    #[test]
    fn proxy_scores_in_unit_interval() {
        let brain = synthetic_test_brain();
        let t = proxy_task_from_structure(&brain);
        let b = proxy_biology_from_structure(&brain);
        assert!((0.0..=1.0).contains(&t));
        assert!((0.0..=1.0).contains(&b));
    }
}
