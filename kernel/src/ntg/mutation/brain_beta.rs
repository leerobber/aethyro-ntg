//! Phase 6.15: Brain β — Learning and Intelligent Routing.
//!
//! Discovers patterns, optimizes strategies, and routes mutations efficiently:
//! - Pattern learning from successful mutations
//! - Domain expertise tracking and strategy portfolio
//! - Intelligent mutation routing based on affinity
//! - Load prediction and bottleneck detection
//! - Adaptive strategy selection

use std::collections::{HashMap, VecDeque};
use super::super::error::NtgError;
use super::domain_coordination::{AgentId, AgentLevel, MutationProposal};

/// Learned mutation pattern.
#[derive(Clone, Debug)]
pub struct MutationPattern {
    pub pattern_id: u64,
    pub domain: String,
    pub mutation_type: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_efficiency_gain: f32,
    pub confidence: f32,
    pub last_seen: u64,
}

impl MutationPattern {
    /// Success rate for this pattern.
    pub fn success_rate(&self) -> f32 {
        let total = (self.success_count + self.failure_count) as f32;
        if total == 0.0 {
            0.0
        } else {
            self.success_count as f32 / total
        }
    }
}

/// Pattern discovery and learning engine.
#[derive(Clone, Debug)]
pub struct PatternLearner {
    pub patterns: HashMap<u64, MutationPattern>,
    pub domain_expertise: HashMap<String, f32>,
    pub next_pattern_id: u64,
    pub history_window: usize,
}

impl PatternLearner {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            domain_expertise: HashMap::new(),
            next_pattern_id: 0,
            history_window: 100,
        }
    }

    /// Learn a successful mutation pattern.
    pub fn learn_success(
        &mut self,
        domain: String,
        mutation_type: String,
        efficiency_gain: f32,
    ) -> u64 {
        let pattern = self
            .patterns
            .values_mut()
            .find(|p| p.domain == domain && p.mutation_type == mutation_type);

        if let Some(pattern) = pattern {
            pattern.success_count += 1;
            pattern.avg_efficiency_gain =
                (pattern.avg_efficiency_gain + efficiency_gain) / 2.0;
            pattern.confidence = (pattern.confidence + 0.05).min(1.0);
            pattern.last_seen = 0; // Would be current cycle
            pattern.pattern_id
        } else {
            let pattern_id = self.next_pattern_id;
            self.next_pattern_id += 1;

            self.patterns.insert(
                pattern_id,
                MutationPattern {
                    pattern_id,
                    domain: domain.clone(),
                    mutation_type,
                    success_count: 1,
                    failure_count: 0,
                    avg_efficiency_gain: efficiency_gain,
                    confidence: 0.5,
                    last_seen: 0,
                },
            );

            // Update domain expertise
            let expertise = self.domain_expertise.entry(domain).or_insert(0.0);
            *expertise = (*expertise + 0.1).min(1.0);

            pattern_id
        }
    }

    /// Learn a failed mutation pattern.
    pub fn learn_failure(&mut self, pattern_id: u64) {
        if let Some(pattern) = self.patterns.get_mut(&pattern_id) {
            pattern.failure_count += 1;
            pattern.confidence = (pattern.confidence - 0.05).max(0.0);
        }
    }

    /// Get patterns for a domain.
    pub fn patterns_for_domain(&self, domain: &str) -> Vec<&MutationPattern> {
        self.patterns
            .values()
            .filter(|p| p.domain == domain)
            .collect()
    }

    /// Get top patterns by success rate.
    pub fn top_patterns(&self, limit: usize) -> Vec<&MutationPattern> {
        let mut patterns: Vec<_> = self.patterns.values().collect();
        patterns.sort_by(|a, b| {
            b.success_rate()
                .partial_cmp(&a.success_rate())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        patterns.into_iter().take(limit).collect()
    }
}

/// Active mutation strategy.
#[derive(Clone, Debug)]
pub struct Strategy {
    pub strategy_id: u64,
    pub name: String,
    pub domain: String,
    pub patterns: Vec<u64>, // Pattern IDs
    pub active: bool,
    pub portfolio_weight: f32,
    pub recent_returns: VecDeque<f32>,
}

/// Strategy optimization and portfolio management.
#[derive(Clone, Debug)]
pub struct StrategyOptimizer {
    pub strategies: HashMap<u64, Strategy>,
    pub next_strategy_id: u64,
    pub active_strategies: usize,
}

impl StrategyOptimizer {
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            next_strategy_id: 0,
            active_strategies: 5,
        }
    }

    /// Create a new strategy from patterns.
    pub fn create_strategy(&mut self, name: String, domain: String, patterns: Vec<u64>) -> u64 {
        let strategy_id = self.next_strategy_id;
        self.next_strategy_id += 1;

        self.strategies.insert(
            strategy_id,
            Strategy {
                strategy_id,
                name,
                domain,
                patterns,
                active: true,
                portfolio_weight: 0.2,
                recent_returns: VecDeque::with_capacity(20),
            },
        );

        strategy_id
    }

    /// Record return for a strategy.
    pub fn record_return(&mut self, strategy_id: u64, return_value: f32) {
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            strategy.recent_returns.push_back(return_value);
            if strategy.recent_returns.len() > 20 {
                strategy.recent_returns.pop_front();
            }
        }
    }

    /// Compute Sharpe ratio for a strategy (risk-adjusted return).
    pub fn compute_sharpe_ratio(&self, strategy_id: u64) -> f32 {
        if let Some(strategy) = self.strategies.get(&strategy_id) {
            if strategy.recent_returns.is_empty() {
                return 0.0;
            }

            let mean: f32 = strategy.recent_returns.iter().sum::<f32>() / strategy.recent_returns.len() as f32;
            let variance: f32 = strategy
                .recent_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f32>()
                / strategy.recent_returns.len() as f32;

            let std_dev = variance.sqrt();
            if std_dev < 0.001 {
                mean
            } else {
                mean / std_dev
            }
        } else {
            0.0
        }
    }

    /// Rebalance portfolio based on recent performance.
    pub fn rebalance_portfolio(&mut self) {
        let active: Vec<_> = self
            .strategies
            .values()
            .filter(|s| s.active)
            .collect();

        if active.is_empty() {
            return;
        }

        // Compute Sharpe ratios
        let mut ratios: Vec<(u64, f32)> = active
            .iter()
            .map(|s| (s.strategy_id, self.compute_sharpe_ratio(s.strategy_id)))
            .collect();

        ratios.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Collect weight updates
        let weight_updates: Vec<(u64, f32)> = self.strategies
            .keys()
            .filter_map(|strategy_id| {
                if let Some(pos) = ratios.iter().position(|(id, _)| id == strategy_id) {
                    let weight = if pos == 0 {
                        0.4
                    } else if pos == 1 {
                        0.3
                    } else if pos == 2 {
                        0.2
                    } else {
                        0.05
                    };
                    Some((*strategy_id, weight))
                } else {
                    None
                }
            })
            .collect();

        // Apply weight updates
        for (strategy_id, weight) in weight_updates {
            if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                strategy.portfolio_weight = weight;
            }
        }
    }
}

/// Routing decision for a mutation.
#[derive(Clone, Debug)]
pub struct RoutingDecision {
    pub target_agent: AgentId,
    pub routing_score: f32,
    pub affinity_score: f32,
    pub load_score: f32,
    pub estimated_queue_time: u64,
}

/// Intelligent mutation router.
#[derive(Clone, Debug)]
pub struct MutationRouter {
    pub routing_table: HashMap<String, Vec<AgentId>>,
    pub affinity_cache: HashMap<(AgentId, AgentId), f32>,
    pub queue_length_estimates: HashMap<AgentId, usize>,
    pub strategy_affinity: HashMap<String, Vec<AgentId>>,
}

impl MutationRouter {
    pub fn new() -> Self {
        Self {
            routing_table: HashMap::new(),
            affinity_cache: HashMap::new(),
            queue_length_estimates: HashMap::new(),
            strategy_affinity: HashMap::new(),
        }
    }

    /// Route a mutation to the best agent.
    pub fn route_mutation(
        &self,
        mutation: &MutationProposal,
        candidate_agents: &[AgentId],
    ) -> Result<RoutingDecision, NtgError> {
        if candidate_agents.is_empty() {
            return Err(NtgError::InvalidInput("No candidate agents".to_string()));
        }

        let mut best_decision = None;
        let mut best_score = 0.0;

        for &agent_id in candidate_agents {
            let affinity_score = self
                .affinity_cache
                .get(&(mutation.agent_id, agent_id))
                .copied()
                .unwrap_or(0.5);

            let queue_len = *self.queue_length_estimates.get(&agent_id).unwrap_or(&0);
            let load_score = 1.0 / (1.0 + queue_len as f32 * 0.1);

            let routing_score = affinity_score * 0.6 + load_score * 0.4;

            if routing_score > best_score {
                best_score = routing_score;
                best_decision = Some(RoutingDecision {
                    target_agent: agent_id,
                    routing_score,
                    affinity_score,
                    load_score,
                    estimated_queue_time: (queue_len as u64) * 2,
                });
            }
        }

        best_decision.ok_or_else(|| NtgError::InvalidInput("Routing failed".to_string()))
    }

    /// Update affinity cache entry.
    pub fn update_affinity(&mut self, agent_a: AgentId, agent_b: AgentId, score: f32) {
        let key = if agent_a < agent_b {
            (agent_a, agent_b)
        } else {
            (agent_b, agent_a)
        };
        self.affinity_cache.insert(key, score);
    }

    /// Update queue length estimate.
    pub fn update_queue_estimate(&mut self, agent_id: AgentId, length: usize) {
        self.queue_length_estimates.insert(agent_id, length);
    }
}

/// Load prediction for bottleneck detection.
#[derive(Clone, Debug)]
pub struct LoadPredictor {
    pub queue_history: HashMap<AgentId, VecDeque<usize>>,
    pub prediction_window: usize,
}

impl LoadPredictor {
    pub fn new() -> Self {
        Self {
            queue_history: HashMap::new(),
            prediction_window: 10,
        }
    }

    /// Record queue depth observation.
    pub fn observe_queue(&mut self, agent_id: AgentId, depth: usize) {
        let history = self
            .queue_history
            .entry(agent_id)
            .or_insert_with(|| VecDeque::with_capacity(10));

        history.push_back(depth);
        if history.len() > 10 {
            history.pop_front();
        }
    }

    /// Predict queue depth N cycles ahead using linear extrapolation.
    pub fn predict_queue_depth(&self, agent_id: AgentId, cycles_ahead: usize) -> usize {
        if let Some(history) = self.queue_history.get(&agent_id) {
            if history.len() < 2 {
                return history.back().copied().unwrap_or(0);
            }

            // Simple trend: compare recent vs older
            let recent: f32 = history.iter().rev().take(3).map(|&x| x as f32).sum::<f32>() / 3.0;
            let older: f32 = history
                .iter()
                .rev()
                .skip(3)
                .take(3)
                .map(|&x| x as f32)
                .sum::<f32>()
                / 3.0;

            let trend = recent - older;
            let predicted = (recent + trend * (cycles_ahead as f32)).max(0.0) as usize;
            predicted
        } else {
            0
        }
    }

    /// Detect bottleneck agents (consistently high queues).
    pub fn detect_bottlenecks(&self, threshold: usize) -> Vec<AgentId> {
        self.queue_history
            .iter()
            .filter_map(|(agent_id, history)| {
                let avg: f32 = history.iter().map(|&x| x as f32).sum::<f32>() / history.len() as f32;
                if avg as usize >= threshold {
                    Some(*agent_id)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Brain β: Learning and Routing Engine.
#[derive(Clone, Debug)]
pub struct BrainBeta {
    pub agent_id: AgentId,
    pub level: AgentLevel,

    // Learning systems
    pub pattern_learner: PatternLearner,
    pub strategy_optimizer: StrategyOptimizer,

    // Routing
    pub mutation_router: MutationRouter,
    pub load_predictor: LoadPredictor,

    // Metrics
    pub mutations_routed: u64,
    pub successful_routes: u64,
    pub pattern_discoveries: u64,
}

impl BrainBeta {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level,
            pattern_learner: PatternLearner::new(),
            strategy_optimizer: StrategyOptimizer::new(),
            mutation_router: MutationRouter::new(),
            load_predictor: LoadPredictor::new(),
            mutations_routed: 0,
            successful_routes: 0,
            pattern_discoveries: 0,
        }
    }

    /// Learn from a successful mutation.
    pub fn learn_success(&mut self, domain: String, mutation_type: String, gain: f32) {
        let _pattern_id = self.pattern_learner.learn_success(domain, mutation_type, gain);
        self.pattern_discoveries += 1;
    }

    /// Learn from a failed mutation.
    pub fn learn_failure(&mut self, pattern_id: u64) {
        self.pattern_learner.learn_failure(pattern_id);
    }

    /// Route a mutation to the best agent.
    pub fn route_mutation(
        &mut self,
        mutation: &MutationProposal,
        candidates: &[AgentId],
    ) -> Result<RoutingDecision, NtgError> {
        self.mutations_routed += 1;
        let decision = self.mutation_router.route_mutation(mutation, candidates)?;
        self.successful_routes += 1;
        Ok(decision)
    }

    /// Predict load for an agent.
    pub fn predict_load(&self, agent_id: AgentId, cycles_ahead: usize) -> usize {
        self.load_predictor.predict_queue_depth(agent_id, cycles_ahead)
    }

    /// Report brain β status.
    pub fn report(&self) -> String {
        format!(
            "Brain β [{:?}] — Patterns: {}, Routes: {}/{}, Patterns discovered: {}",
            self.level,
            self.pattern_learner.patterns.len(),
            self.successful_routes,
            self.mutations_routed,
            self.pattern_discoveries
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_success_rate() {
        let pattern = MutationPattern {
            pattern_id: 1,
            domain: "learning".to_string(),
            mutation_type: "add_node".to_string(),
            success_count: 8,
            failure_count: 2,
            avg_efficiency_gain: 0.15,
            confidence: 0.8,
            last_seen: 0,
        };

        assert_eq!(pattern.success_rate(), 0.8);
    }

    #[test]
    fn pattern_learner_new() {
        let learner = PatternLearner::new();
        assert_eq!(learner.patterns.len(), 0);
        assert_eq!(learner.domain_expertise.len(), 0);
    }

    #[test]
    fn pattern_learner_success() {
        let mut learner = PatternLearner::new();

        let pattern_id = learner.learn_success(
            "learning".to_string(),
            "add_node".to_string(),
            0.15,
        );

        assert_eq!(learner.patterns.len(), 1);
        assert!(learner.domain_expertise.contains_key("learning"));
    }

    #[test]
    fn pattern_learner_reinforces_success() {
        let mut learner = PatternLearner::new();

        let id1 = learner.learn_success(
            "learning".to_string(),
            "add_node".to_string(),
            0.15,
        );

        let id2 = learner.learn_success(
            "learning".to_string(),
            "add_node".to_string(),
            0.12,
        );

        assert_eq!(id1, id2); // Same pattern
        assert_eq!(learner.patterns[&id1].success_count, 2);
    }

    #[test]
    fn strategy_optimizer_creation() {
        let mut optimizer = StrategyOptimizer::new();

        let strategy_id = optimizer.create_strategy(
            "explore".to_string(),
            "learning".to_string(),
            vec![0, 1, 2],
        );

        assert_eq!(optimizer.strategies.len(), 1);
        assert!(optimizer.strategies.contains_key(&strategy_id));
    }

    #[test]
    fn strategy_optimizer_record_returns() {
        let mut optimizer = StrategyOptimizer::new();
        let strategy_id = optimizer.create_strategy(
            "explore".to_string(),
            "learning".to_string(),
            vec![],
        );

        for return_val in &[0.1, 0.15, 0.12, 0.18] {
            optimizer.record_return(strategy_id, *return_val);
        }

        assert_eq!(optimizer.strategies[&strategy_id].recent_returns.len(), 4);
    }

    #[test]
    fn strategy_optimizer_sharpe_ratio() {
        let mut optimizer = StrategyOptimizer::new();
        let strategy_id = optimizer.create_strategy(
            "explore".to_string(),
            "learning".to_string(),
            vec![],
        );

        for return_val in &[0.1, 0.1, 0.1, 0.1] {
            optimizer.record_return(strategy_id, *return_val);
        }

        let sharpe = optimizer.compute_sharpe_ratio(strategy_id);
        assert!(sharpe > 0.0); // Consistent returns = positive Sharpe
    }

    #[test]
    fn mutation_router_empty_candidates() {
        let router = MutationRouter::new();
        let proposal = MutationProposal {
            proposal_id: 1,
            agent_id: AgentId::new(1),
            agent_level: AgentLevel::Micro,
            cycle: 0,
            mutation_description: "test".to_string(),
            estimated_efficiency_gain: 0.1,
            confidence: 0.8,
            domain: "learning".to_string(),
            timestamp_us: 0,
        };

        let result = router.route_mutation(&proposal, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn mutation_router_single_candidate() {
        let mut router = MutationRouter::new();
        let target = AgentId::new(2);

        router.update_affinity(AgentId::new(1), target, 0.8);
        router.update_queue_estimate(target, 0);

        let proposal = MutationProposal {
            proposal_id: 1,
            agent_id: AgentId::new(1),
            agent_level: AgentLevel::Micro,
            cycle: 0,
            mutation_description: "test".to_string(),
            estimated_efficiency_gain: 0.1,
            confidence: 0.8,
            domain: "learning".to_string(),
            timestamp_us: 0,
        };

        let decision = router.route_mutation(&proposal, &[target]).unwrap();
        assert_eq!(decision.target_agent, target);
    }

    #[test]
    fn load_predictor_observe() {
        let mut predictor = LoadPredictor::new();
        let agent_id = AgentId::new(1);

        predictor.observe_queue(agent_id, 5);
        predictor.observe_queue(agent_id, 6);
        predictor.observe_queue(agent_id, 7);

        assert_eq!(predictor.queue_history[&agent_id].len(), 3);
    }

    #[test]
    fn load_predictor_detect_bottleneck() {
        let mut predictor = LoadPredictor::new();
        let agent_id = AgentId::new(1);

        for _ in 0..5 {
            predictor.observe_queue(agent_id, 50);
        }

        let bottlenecks = predictor.detect_bottlenecks(40);
        assert!(bottlenecks.contains(&agent_id));
    }

    #[test]
    fn brain_beta_creation() {
        let brain = BrainBeta::new(AgentId::new(1), AgentLevel::Micro);
        assert_eq!(brain.mutations_routed, 0);
        assert_eq!(brain.pattern_discoveries, 0);
    }

    #[test]
    fn brain_beta_learn_success() {
        let mut brain = BrainBeta::new(AgentId::new(1), AgentLevel::Micro);

        brain.learn_success("learning".to_string(), "add_node".to_string(), 0.15);

        assert_eq!(brain.pattern_learner.patterns.len(), 1);
        assert_eq!(brain.pattern_discoveries, 1);
    }

    #[test]
    fn brain_beta_report() {
        let brain = BrainBeta::new(AgentId::new(1), AgentLevel::Micro);
        let report = brain.report();
        assert!(report.contains("Brain β"));
        assert!(report.contains("Micro"));
    }
}
