//! Brain γ: Meta-Governance & Evolution Engine
//!
//! Phase 6.17 implements the governance layer:
//! - Policy directive synthesis from α/β/δ snapshots
//! - Evolution plan adaptation based on mutation outcomes
//! - Alignment scoring and constraint enforcement
//! - Lineage tracking and specialization management
//! - Policy priority levels and scope-based governance

use crate::ntg::mutation::domain_coordination::{AgentId, AgentLevel};
use crate::ntg::error::NtgError;
use crate::ntg::mutation::brain_integration::{HealthSnapshot, StrategySnapshot};
use std::collections::HashMap;

/// Policy priority level.
#[derive(Clone, Debug, PartialEq, Copy, Eq, Hash)]
pub enum PolicyPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Scope of policy application.
#[derive(Clone, Debug, PartialEq)]
pub enum PolicyScope {
    Agent(AgentId),
    Cluster(String),
    Swarm,
}

/// Enforcement mode for constraints.
#[derive(Clone, Debug, PartialEq, Copy, Eq)]
pub enum EnforcementMode {
    Advisory,
    Restrictive,
    Adaptive,
}

/// Constraint rule for policy enforcement.
#[derive(Clone, Debug)]
pub struct ConstraintRule {
    pub rule_type: String,
    pub threshold: f32,
    pub enforcement_mode: EnforcementMode,
}

/// Policy directive: high-level governance rule.
#[derive(Clone, Debug)]
pub struct PolicyDirective {
    pub priority: PolicyPriority,
    pub target_scope: PolicyScope,
    pub constraints: Vec<ConstraintRule>,
    pub mutation_budget_delta: i32,
    pub consensus_threshold_delta: f32,
    pub exploration_weight_delta: f32,
    pub notes: String,
}

/// Mutation type for evolution planning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MutationType {
    Point,
    Duplication,
    Deletion,
    Regulatory,
    Structural,
}

/// Evolution plan: guides mutation selection and weighting.
#[derive(Clone, Debug)]
pub struct EvolutionPlan {
    pub enabled_mutation_types: Vec<MutationType>,
    pub suppressed_mutation_types: Vec<MutationType>,
    pub lineage_objectives: Vec<LineageObjective>,
    pub specialization_pressure: f32,
    pub exploration_phase: bool,
}

/// Lineage objective for specialization.
#[derive(Clone, Debug)]
pub struct LineageObjective {
    pub domain: String,
    pub specialization_score_target: f32,
    pub risk_tolerance: f32,
}

/// Outcome of a mutation attempt.
#[derive(Clone, Debug)]
pub struct MutationOutcome {
    pub mutation_type: MutationType,
    pub success: bool,
    pub fitness_delta: f32,
    pub alignment_delta: f32,
    pub lineage_value: f32,
}

/// Lineage record for evolutionary tracking.
#[derive(Clone, Debug)]
pub struct LineageRecord {
    pub ancestor_id: AgentId,
    pub generation: u32,
    pub specialization: String,
    pub fitness: f32,
    pub traits: Vec<String>,
}

/// Snapshot of forecasts from Brain δ.
#[derive(Clone, Debug, Default)]
pub struct ForecastSnapshot {
    pub drift_forecast: f32,
    pub load_forecast: f32,
    pub efficiency_trend: f32,
    pub risk_trend: f32,
}

/// Governance metrics and state.
#[derive(Clone, Debug, Default)]
pub struct GovernanceMetrics {
    pub long_term_fitness: f32,
    pub alignment_score: f32,
    pub policy_conflict_count: u64,
    pub evolution_success_rate: f32,
    pub mutation_acceptance_rate: f32,
}

/// Brain γ: Meta-Governance Engine
#[derive(Clone, Debug)]
pub struct BrainGamma {
    pub agent_id: AgentId,
    pub level: AgentLevel,

    // Policy management
    pub active_policies: Vec<PolicyDirective>,
    pub global_constraints: Vec<ConstraintRule>,

    // Evolution management
    pub evolution_plan: EvolutionPlan,
    pub lineage_history: Vec<LineageRecord>,
    pub mutation_outcomes: Vec<MutationOutcome>,

    // Metrics & state
    pub metrics: GovernanceMetrics,
    pub cycle_count: u64,
    pub last_policy_update: u64,

    // Input snapshots
    pub alpha_health: Option<HealthSnapshot>,
    pub beta_strategy: Option<StrategySnapshot>,
    pub delta_forecast: Option<ForecastSnapshot>,
}

impl BrainGamma {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level,
            active_policies: Vec::new(),
            global_constraints: Vec::new(),
            evolution_plan: EvolutionPlan {
                enabled_mutation_types: vec![MutationType::Point, MutationType::Regulatory],
                suppressed_mutation_types: Vec::new(),
                lineage_objectives: Vec::new(),
                specialization_pressure: 0.5,
                exploration_phase: true,
            },
            lineage_history: Vec::new(),
            mutation_outcomes: Vec::new(),
            metrics: GovernanceMetrics::default(),
            cycle_count: 0,
            last_policy_update: 0,
            alpha_health: None,
            beta_strategy: None,
            delta_forecast: None,
        }
    }

    /// Ingest snapshots from all three brains.
    pub fn update_inputs(
        &mut self,
        alpha: HealthSnapshot,
        beta: StrategySnapshot,
        delta: ForecastSnapshot,
    ) {
        self.alpha_health = Some(alpha);
        self.beta_strategy = Some(beta);
        self.delta_forecast = Some(delta);
    }

    /// Evaluate alignment: how well α/β/δ follow γ policies.
    pub fn evaluate_alignment(&mut self) -> f32 {
        let mut score = 1.0f32;

        if let Some(health) = &self.alpha_health {
            if health.rollback_events > 0 {
                score *= 0.95;
            }
        }

        if let Some(strategy) = &self.beta_strategy {
            if strategy.routing_efficiency < 0.7 {
                score *= 0.90;
            }
        }

        if let Some(forecast) = &self.delta_forecast {
            if forecast.risk_trend > 0.1 {
                score *= 0.92;
            }
        }

        self.metrics.alignment_score = score;
        score
    }

    /// Synthesize new policies based on current state and snapshots.
    pub fn synthesize_policies(&mut self) -> Result<Vec<PolicyDirective>, NtgError> {
        self.cycle_count += 1;
        let mut policies = Vec::new();

        // If drift detected in α, tighten consensus
        if let Some(health) = &self.alpha_health {
            if health.drift_score > 0.4 {
                policies.push(PolicyDirective {
                    priority: PolicyPriority::High,
                    target_scope: PolicyScope::Agent(self.agent_id),
                    constraints: vec![ConstraintRule {
                        rule_type: "tighten_consensus".to_string(),
                        threshold: 0.85,
                        enforcement_mode: EnforcementMode::Restrictive,
                    }],
                    consensus_threshold_delta: 0.05,
                    mutation_budget_delta: -2,
                    exploration_weight_delta: -0.1,
                    notes: "High drift detected, tightening consensus".to_string(),
                });
            }
        }

        // If load forecast high in δ, reduce exploration in β
        if let Some(forecast) = &self.delta_forecast {
            if forecast.load_forecast > 0.8 {
                policies.push(PolicyDirective {
                    priority: PolicyPriority::Normal,
                    target_scope: PolicyScope::Cluster("compute".to_string()),
                    constraints: Vec::new(),
                    consensus_threshold_delta: 0.0,
                    mutation_budget_delta: -1,
                    exploration_weight_delta: -0.2,
                    notes: "High load forecast, reducing exploration".to_string(),
                });
            }
        }

        // If efficiency trend positive, enable structural mutations
        if let Some(forecast) = &self.delta_forecast {
            if forecast.efficiency_trend > 0.05
                && !self.evolution_plan.enabled_mutation_types.contains(&MutationType::Structural) {
                    policies.push(PolicyDirective {
                        priority: PolicyPriority::Normal,
                        target_scope: PolicyScope::Swarm,
                        constraints: Vec::new(),
                        consensus_threshold_delta: 0.0,
                        mutation_budget_delta: 2,
                        exploration_weight_delta: 0.15,
                        notes: "Efficiency improving, enabling structural mutations".to_string(),
                    });
                }
        }

        self.active_policies = policies.clone();
        self.last_policy_update = self.cycle_count;
        Ok(policies)
    }

    /// Record mutation outcome for evolutionary learning.
    pub fn record_mutation_outcome(&mut self, outcome: MutationOutcome) {
        self.mutation_outcomes.push(outcome.clone());

        let successful = self.mutation_outcomes.iter().filter(|m| m.success).count();
        self.metrics.evolution_success_rate =
            successful as f32 / (self.mutation_outcomes.len() as f32).max(1.0);

        if self.mutation_outcomes.len() > 1000 {
            self.mutation_outcomes.remove(0);
        }
    }

    /// Update evolution plan based on mutation outcomes.
    pub fn update_evolution_plan(&mut self) {
        if self.mutation_outcomes.is_empty() {
            return;
        }

        let mut success_by_type: HashMap<MutationType, (u32, u32)> = HashMap::new();
        for outcome in &self.mutation_outcomes {
            let (successes, total) = success_by_type
                .entry(outcome.mutation_type)
                .or_insert((0, 0));
            *total += 1;
            if outcome.success {
                *successes += 1;
            }
        }

        for (mutation_type, (successes, total)) in success_by_type {
            let success_rate = successes as f32 / (total as f32).max(1.0);

            // High success rate: enable if suppressed
            if success_rate > 0.7 && self.evolution_plan.suppressed_mutation_types.contains(&mutation_type) {
                self.evolution_plan.suppressed_mutation_types.retain(|t| t != &mutation_type);
                self.evolution_plan.enabled_mutation_types.push(mutation_type);
            }

            // Low success rate: suppress if enabled
            if success_rate < 0.3 && self.evolution_plan.enabled_mutation_types.contains(&mutation_type) {
                self.evolution_plan.enabled_mutation_types.retain(|t| t != &mutation_type);
                self.evolution_plan.suppressed_mutation_types.push(mutation_type);
            }
        }
    }

    /// Get sync policy for Brain α.
    pub fn get_sync_policy(&self) -> (f32, f32) {
        let consensus_delta = self.active_policies.iter()
            .find(|p| p.consensus_threshold_delta != 0.0)
            .map(|p| p.consensus_threshold_delta)
            .unwrap_or(0.0);

        let rollback_aggressiveness = if consensus_delta > 0.0 { 1.5 } else { 0.8 };
        (0.80 + consensus_delta, rollback_aggressiveness)
    }

    /// Get strategy policy for Brain β.
    pub fn get_strategy_policy(&self) -> (Vec<String>, f32) {
        let exploration_delta = self.active_policies.iter()
            .find(|p| p.exploration_weight_delta != 0.0)
            .map(|p| p.exploration_weight_delta)
            .unwrap_or(0.0);

        let allowed = self.active_policies.iter()
            .flat_map(|p| p.constraints.iter()
                .filter(|c| c.rule_type == "allowed_strategy")
                .map(|c| c.rule_type.clone()))
            .collect();

        (allowed, 0.3 + exploration_delta)
    }

    /// Get perception focus for Brain δ.
    pub fn get_perception_focus(&self) -> (bool, bool, bool) {
        let track_drift = self.active_policies.iter()
            .any(|p| p.priority == PolicyPriority::High || p.priority == PolicyPriority::Critical);

        let track_risk = self.active_policies.iter()
            .any(|p| p.priority == PolicyPriority::Critical);

        (track_drift, true, track_risk)
    }

    /// Add lineage record for specialization tracking.
    pub fn record_lineage(&mut self, record: LineageRecord) {
        self.lineage_history.push(record);
        if self.lineage_history.len() > 500 {
            self.lineage_history.remove(0);
        }
    }

    /// Get report of governance state.
    pub fn report(&self) -> GammaReport {
        GammaReport {
            agent_id: self.agent_id,
            cycle: self.cycle_count,
            active_policies_count: self.active_policies.len() as u64,
            evolution_success_rate: self.metrics.evolution_success_rate,
            alignment_score: self.metrics.alignment_score,
            mutation_outcomes_count: self.mutation_outcomes.len() as u64,
            lineage_depth: self.lineage_history.len() as u32,
            exploration_phase: self.evolution_plan.exploration_phase,
        }
    }
}

/// Report from Brain γ.
#[derive(Clone, Debug)]
pub struct GammaReport {
    pub agent_id: AgentId,
    pub cycle: u64,
    pub active_policies_count: u64,
    pub evolution_success_rate: f32,
    pub alignment_score: f32,
    pub mutation_outcomes_count: u64,
    pub lineage_depth: u32,
    pub exploration_phase: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::mutation::domain_coordination::AgentLevel;

    #[test]
    fn brain_gamma_creation() {
        let gamma = BrainGamma::new(AgentId::new(1), AgentLevel::Nano);
        assert_eq!(gamma.agent_id, AgentId::new(1));
        assert_eq!(gamma.cycle_count, 0);
        assert!(gamma.evolution_plan.exploration_phase);
    }

    #[test]
    fn brain_gamma_initial_evolution_plan() {
        let gamma = BrainGamma::new(AgentId::new(1), AgentLevel::Micro);
        assert!(gamma.evolution_plan.enabled_mutation_types.contains(&MutationType::Point));
        assert!(gamma.evolution_plan.enabled_mutation_types.contains(&MutationType::Regulatory));
        assert_eq!(gamma.evolution_plan.specialization_pressure, 0.5);
    }

    #[test]
    fn brain_gamma_evaluate_alignment() {
        let mut gamma = BrainGamma::new(AgentId::new(2), AgentLevel::Sub);
        let health = HealthSnapshot {
            drift_score: 0.2,
            rollback_events: 0,
            connection_quality: 0.95,
        };
        let strategy = StrategySnapshot {
            active_strategies: vec!["route".to_string()],
            strategy_scores: vec![("route".to_string(), 0.9)],
            routing_efficiency: 0.85,
        };
        let forecast = ForecastSnapshot {
            drift_forecast: 0.1,
            load_forecast: 0.3,
            efficiency_trend: 0.05,
            risk_trend: -0.02,
        };

        gamma.update_inputs(health, strategy, forecast);
        let alignment = gamma.evaluate_alignment();
        assert!(alignment > 0.8);
    }

    #[test]
    fn brain_gamma_alignment_with_drift() {
        let mut gamma = BrainGamma::new(AgentId::new(3), AgentLevel::Super);
        let health = HealthSnapshot {
            drift_score: 0.7,
            rollback_events: 2,
            connection_quality: 0.85,
        };
        let strategy = StrategySnapshot::default();
        let forecast = ForecastSnapshot::default();

        gamma.update_inputs(health, strategy, forecast);
        let alignment = gamma.evaluate_alignment();
        assert!(alignment < 1.0);
    }

    #[test]
    fn brain_gamma_synthesize_policies_high_drift() {
        let mut gamma = BrainGamma::new(AgentId::new(4), AgentLevel::Nano);
        let health = HealthSnapshot {
            drift_score: 0.6,
            rollback_events: 1,
            connection_quality: 0.9,
        };
        let strategy = StrategySnapshot::default();
        let forecast = ForecastSnapshot::default();

        gamma.update_inputs(health, strategy, forecast);
        let policies = gamma.synthesize_policies().unwrap();
        assert!(!policies.is_empty());
        assert_eq!(policies[0].priority, PolicyPriority::High);
    }

    #[test]
    fn brain_gamma_synthesize_policies_high_load() {
        let mut gamma = BrainGamma::new(AgentId::new(5), AgentLevel::Micro);
        let health = HealthSnapshot::default();
        let strategy = StrategySnapshot::default();
        let forecast = ForecastSnapshot {
            drift_forecast: 0.1,
            load_forecast: 0.85,
            efficiency_trend: 0.0,
            risk_trend: 0.0,
        };

        gamma.update_inputs(health, strategy, forecast);
        let policies = gamma.synthesize_policies().unwrap();
        assert!(!policies.is_empty());
    }

    #[test]
    fn brain_gamma_record_mutation_outcome() {
        let mut gamma = BrainGamma::new(AgentId::new(6), AgentLevel::Sub);

        let outcome = MutationOutcome {
            mutation_type: MutationType::Point,
            success: true,
            fitness_delta: 0.1,
            alignment_delta: 0.05,
            lineage_value: 0.8,
        };

        gamma.record_mutation_outcome(outcome);
        assert_eq!(gamma.mutation_outcomes.len(), 1);
        assert!(gamma.metrics.evolution_success_rate > 0.0);
    }

    #[test]
    fn brain_gamma_evolution_success_rate() {
        let mut gamma = BrainGamma::new(AgentId::new(7), AgentLevel::Nano);

        for i in 0..10 {
            let outcome = MutationOutcome {
                mutation_type: MutationType::Point,
                success: i < 7,  // 7 successes out of 10
                fitness_delta: 0.1,
                alignment_delta: 0.05,
                lineage_value: 0.8,
            };
            gamma.record_mutation_outcome(outcome);
        }

        assert!(gamma.metrics.evolution_success_rate > 0.6 && gamma.metrics.evolution_success_rate < 0.8);
    }

    #[test]
    fn brain_gamma_update_evolution_plan_enable_type() {
        let mut gamma = BrainGamma::new(AgentId::new(8), AgentLevel::Micro);
        gamma.evolution_plan.suppressed_mutation_types = vec![MutationType::Structural];

        // Record 8 successes out of 10 for Structural
        for i in 0..10 {
            let outcome = MutationOutcome {
                mutation_type: MutationType::Structural,
                success: i < 8,
                fitness_delta: 0.1,
                alignment_delta: 0.0,
                lineage_value: 0.8,
            };
            gamma.record_mutation_outcome(outcome);
        }

        gamma.update_evolution_plan();
        assert!(!gamma.evolution_plan.suppressed_mutation_types.contains(&MutationType::Structural));
    }

    #[test]
    fn brain_gamma_update_evolution_plan_suppress_type() {
        let mut gamma = BrainGamma::new(AgentId::new(9), AgentLevel::Super);
        gamma.evolution_plan.enabled_mutation_types = vec![MutationType::Duplication];

        // Record 2 successes out of 10 for Duplication
        for i in 0..10 {
            let outcome = MutationOutcome {
                mutation_type: MutationType::Duplication,
                success: i < 2,
                fitness_delta: -0.05,
                alignment_delta: -0.1,
                lineage_value: 0.3,
            };
            gamma.record_mutation_outcome(outcome);
        }

        gamma.update_evolution_plan();
        assert!(gamma.evolution_plan.suppressed_mutation_types.contains(&MutationType::Duplication));
    }

    #[test]
    fn brain_gamma_get_sync_policy() {
        let mut gamma = BrainGamma::new(AgentId::new(10), AgentLevel::Nano);
        let health = HealthSnapshot {
            drift_score: 0.6,
            rollback_events: 1,
            connection_quality: 0.9,
        };
        gamma.update_inputs(health, StrategySnapshot::default(), ForecastSnapshot::default());
        gamma.synthesize_policies().unwrap();

        let (threshold, aggressiveness) = gamma.get_sync_policy();
        assert!(threshold > 0.8);
        assert!(aggressiveness > 1.0);  // More aggressive due to high drift
    }

    #[test]
    fn brain_gamma_get_strategy_policy() {
        let gamma = BrainGamma::new(AgentId::new(11), AgentLevel::Micro);
        let (_strategies, exploration) = gamma.get_strategy_policy();
        assert!((0.0..=1.0).contains(&exploration));
    }

    #[test]
    fn brain_gamma_get_perception_focus() {
        let mut gamma = BrainGamma::new(AgentId::new(12), AgentLevel::Sub);
        let health = HealthSnapshot {
            drift_score: 0.7,
            rollback_events: 1,
            connection_quality: 0.9,
        };
        gamma.update_inputs(health, StrategySnapshot::default(), ForecastSnapshot::default());
        gamma.synthesize_policies().unwrap();

        let (track_drift, track_load, _track_risk) = gamma.get_perception_focus();
        assert!(track_drift);
        assert!(track_load);
    }

    #[test]
    fn brain_gamma_record_lineage() {
        let mut gamma = BrainGamma::new(AgentId::new(13), AgentLevel::Nano);

        let record = LineageRecord {
            ancestor_id: AgentId::new(1),
            generation: 1,
            specialization: "compute".to_string(),
            fitness: 0.85,
            traits: vec!["fast".to_string(), "accurate".to_string()],
        };

        gamma.record_lineage(record);
        assert_eq!(gamma.lineage_history.len(), 1);
    }

    #[test]
    fn brain_gamma_lineage_bounded() {
        let mut gamma = BrainGamma::new(AgentId::new(14), AgentLevel::Micro);

        for i in 0..600 {
            let record = LineageRecord {
                ancestor_id: AgentId::new((i % 10) as u64),
                generation: (i / 10) as u32,
                specialization: format!("domain_{}", i % 5),
                fitness: 0.7 + (i as f32 * 0.001),
                traits: vec!["trait1".to_string()],
            };
            gamma.record_lineage(record);
        }

        assert_eq!(gamma.lineage_history.len(), 500);
    }

    #[test]
    fn brain_gamma_cycle_advancement() {
        let mut gamma = BrainGamma::new(AgentId::new(15), AgentLevel::Super);

        for _ in 0..5 {
            let health = HealthSnapshot::default();
            let strategy = StrategySnapshot::default();
            let forecast = ForecastSnapshot::default();
            gamma.update_inputs(health, strategy, forecast);
            gamma.synthesize_policies().unwrap();
        }

        assert_eq!(gamma.cycle_count, 5);
    }

    #[test]
    fn brain_gamma_report() {
        let mut gamma = BrainGamma::new(AgentId::new(16), AgentLevel::Nano);
        let health = HealthSnapshot::default();
        let strategy = StrategySnapshot::default();
        let forecast = ForecastSnapshot::default();

        gamma.update_inputs(health, strategy, forecast);
        gamma.synthesize_policies().unwrap();

        let report = gamma.report();
        assert_eq!(report.agent_id, AgentId::new(16));
        assert_eq!(report.cycle, 1);
        assert!(report.exploration_phase);
    }

    #[test]
    fn brain_gamma_mutation_outcomes_bounded() {
        let mut gamma = BrainGamma::new(AgentId::new(17), AgentLevel::Micro);

        for i in 0..1100 {
            let outcome = MutationOutcome {
                mutation_type: if i % 2 == 0 { MutationType::Point } else { MutationType::Regulatory },
                success: i % 3 == 0,
                fitness_delta: 0.05,
                alignment_delta: 0.01,
                lineage_value: 0.75,
            };
            gamma.record_mutation_outcome(outcome);
        }

        assert_eq!(gamma.mutation_outcomes.len(), 1000);
    }
}
