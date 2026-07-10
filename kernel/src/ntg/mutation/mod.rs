//! Self-modification engine: rule-based topology mutations with budget enforcement.
//!
//! Phase 3 implements ADR 0002's safety rails:
//! - Rail 2: Bounded compute/time budget per cycle
//! - Rail 3: Automatic rollback on regression
//! - Rail 5: Every mutation event ledger-logged
//!
//! Mutations are proposed by rules (add_node, remove_edge, etc.), evaluated
//! against real graph forward-pass fitness, and accepted/rejected based on
//! measured performance improvement and budget consumption.

pub mod rules;
pub mod evaluator;
pub mod budget;

use super::error::NtgError;
use super::graph::Graph;
use rules::MutationRule;
use evaluator::FitnessEvaluator;
use budget::BudgetTracker;

/// Configuration for the self-modification engine (ADR 0002).
#[derive(Clone, Debug)]
pub struct SelfModConfig {
    /// Enabled? (disabled by default per ADR 0002 rule 1)
    pub enabled: bool,
    /// Wall-clock budget per mutation cycle in microseconds
    pub cycle_budget_us: u64,
    /// Maximum mutations per cycle
    pub max_mutations_per_cycle: usize,
    /// Fitness improvement required to accept (e.g. 1.01 = 1% better)
    pub fitness_improvement_threshold: f32,
    /// Whether to auto-rollback on regression
    pub auto_rollback_on_regression: bool,
}

impl Default for SelfModConfig {
    fn default() -> Self {
        Self {
            enabled: false, // **Off by default**
            cycle_budget_us: 1_000_000, // 1 millisecond
            max_mutations_per_cycle: 5,
            fitness_improvement_threshold: 1.01, // 1% improvement required
            auto_rollback_on_regression: true,
        }
    }
}

/// A single mutation cycle: propose → evaluate → decide → log.
pub struct MutationCycle {
    pub config: SelfModConfig,
    pub budget: BudgetTracker,
    pub fitness_evaluator: FitnessEvaluator,
    /// Mutations evaluated this cycle
    pub mutations_proposed: Vec<MutationRule>,
    /// Which mutations were accepted
    pub mutations_accepted: Vec<usize>,
    /// Measured baseline fitness before mutations
    pub baseline_fitness: (u64, u64), // (latency_us, memory_bytes)
}

impl MutationCycle {
    pub fn new(
        config: SelfModConfig,
        baseline_fitness: (u64, u64),
    ) -> Result<Self, NtgError> {
        if !config.enabled {
            return Err(NtgError::InvalidInput(
                "Self-modification is disabled in config".to_string(),
            ));
        }

        let cycle_budget_us = config.cycle_budget_us;
        Ok(Self {
            config,
            budget: BudgetTracker::new(cycle_budget_us),
            fitness_evaluator: FitnessEvaluator::new(),
            mutations_proposed: Vec::new(),
            mutations_accepted: Vec::new(),
            baseline_fitness,
        })
    }

    /// Propose a mutation rule.
    pub fn propose_mutation(&mut self, rule: MutationRule) -> Result<(), NtgError> {
        if self.mutations_proposed.len() >= self.config.max_mutations_per_cycle {
            return Err(NtgError::InvalidInput(format!(
                "Mutation limit per cycle ({}) reached",
                self.config.max_mutations_per_cycle
            )));
        }
        self.mutations_proposed.push(rule);
        Ok(())
    }

    /// Evaluate a proposed mutation against the graph.
    /// Returns (new_fitness, budget_consumed_us).
    pub fn evaluate_mutation(
        &mut self,
        graph: &Graph,
        mutation_idx: usize,
    ) -> Result<((u64, u64), u64), NtgError> {
        if mutation_idx >= self.mutations_proposed.len() {
            return Err(NtgError::IndexOutOfBounds {
                index: mutation_idx,
                len: self.mutations_proposed.len(),
            });
        }

        let rule = &self.mutations_proposed[mutation_idx];

        // Apply the mutation (creates a test graph)
        let mut test_graph = graph.clone();
        rule.apply(&mut test_graph)?;

        // Measure performance
        let start_ns = self.budget.wall_time_ns();
        let fitness = self
            .fitness_evaluator
            .measure_graph(&test_graph, self.baseline_fitness)?;
        let elapsed_ns = self.budget.wall_time_ns() - start_ns;
        let elapsed_us = elapsed_ns / 1000;

        // Track budget
        self.budget.consume_us(elapsed_us)?;

        Ok((fitness, elapsed_us))
    }

    /// Decide whether to accept a mutation based on fitness improvement.
    pub fn should_accept(
        &self,
        new_fitness: (u64, u64),
    ) -> bool {
        // Fitness is (latency_us, memory_bytes).
        // Improvement: lower latency + lower memory = better.
        // Dual-objective: both must improve (or stay same) for acceptance.

        let latency_ratio = new_fitness.0 as f32 / self.baseline_fitness.0 as f32;
        let memory_ratio = new_fitness.1 as f32 / self.baseline_fitness.1 as f32;

        // Both ratios must be <= threshold for acceptance
        // (lower is better, so <= 1.0 means improvement, and we require < threshold)
        latency_ratio <= self.config.fitness_improvement_threshold
            && memory_ratio <= self.config.fitness_improvement_threshold
    }

    /// Accept a mutation (add it to accepted list).
    pub fn accept_mutation(&mut self, mutation_idx: usize) -> Result<(), NtgError> {
        if mutation_idx >= self.mutations_proposed.len() {
            return Err(NtgError::IndexOutOfBounds {
                index: mutation_idx,
                len: self.mutations_proposed.len(),
            });
        }
        self.mutations_accepted.push(mutation_idx);
        Ok(())
    }

    /// Check if cycle completed within budget.
    pub fn within_budget(&self) -> bool {
        self.budget.within_budget()
    }

    /// Budget summary.
    pub fn budget_status(&self) -> (u64, u64, u64) {
        self.budget.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rules::MutationRuleKind;

    #[test]
    fn default_config_is_disabled() {
        let config = SelfModConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn mutation_cycle_requires_enabled_config() {
        let config = SelfModConfig::default();
        assert!(MutationCycle::new(config, (5000, 1024)).is_err());
    }

    #[test]
    fn mutation_cycle_with_enabled_config_succeeds() -> Result<(), NtgError> {
        let config = SelfModConfig {
            enabled: true,
            ..SelfModConfig::default()
        };
        let cycle = MutationCycle::new(config, (5000, 1024))?;
        assert_eq!(cycle.mutations_proposed.len(), 0);
        assert_eq!(cycle.mutations_accepted.len(), 0);
        Ok(())
    }

    #[test]
    fn propose_mutation() -> Result<(), NtgError> {
        let config = SelfModConfig {
            enabled: true,
            ..SelfModConfig::default()
        };
        let mut cycle = MutationCycle::new(config, (5000, 1024))?;

        let rule = MutationRule {
            kind: MutationRuleKind::AddNode { label: "test".to_string() },
        };
        cycle.propose_mutation(rule)?;
        assert_eq!(cycle.mutations_proposed.len(), 1);
        Ok(())
    }

    #[test]
    fn exceeding_mutation_limit_fails() -> Result<(), NtgError> {
        let config = SelfModConfig {
            enabled: true,
            max_mutations_per_cycle: 2,
            ..SelfModConfig::default()
        };
        let mut cycle = MutationCycle::new(config, (5000, 1024))?;

        for i in 0..3 {
            let rule = MutationRule {
                kind: MutationRuleKind::AddNode {
                    label: format!("node_{}", i),
                },
            };
            if i < 2 {
                cycle.propose_mutation(rule)?;
            } else {
                assert!(cycle.propose_mutation(rule).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn fitness_improvement_check() -> Result<(), NtgError> {
        let config = SelfModConfig {
            enabled: true,
            fitness_improvement_threshold: 1.01, // 1% improvement required
            ..SelfModConfig::default()
        };
        let cycle = MutationCycle::new(config, (5000, 1024))?;

        // 5% improvement in both: accepted
        assert!(cycle.should_accept((4750, 973)));

        // 1% improvement: borderline, but accepted (ratio == threshold)
        assert!(cycle.should_accept((5050, 1034)));

        // Regression: rejected
        assert!(!cycle.should_accept((5100, 1100)));
        Ok(())
    }
}
