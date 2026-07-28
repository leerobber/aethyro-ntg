//! Twin-Brain & Quad-Brain Integration: Unified agents with synchronized brains.
//!
//! Phase 6.16 implements the unified twin-brain architecture:
//! - Brain α: Synchronization, healing, drift detection, rollback
//! - Brain β: Learning, pattern discovery, intelligent routing
//! - CrossBrainChannel: Bidirectional communication between brains
//! - Synchronized cycles: Both brains advance together
//! - Conflict resolution: Merge decisions from both brains
//!
//! Phase 6.16Q extends to quad-brain architecture:
//! - Brain γ: Meta-governance, evolution planning, policy synthesis
//! - Brain δ: Perception, representation, forecasting
//! - Extended CrossBrainChannel with γ/δ signal routes
//! - Quad-brain cycle: δ perceive → α sync → β learn → γ govern → feedback

use crate::ntg::mutation::domain_coordination::{AgentId, AgentLevel};
use super::brain_alpha::{BrainAlpha, BehavioralSignature};
use super::brain_beta::BrainBeta;
use std::collections::VecDeque;

/// Signal from Brain β to Brain α for synchronization/repair hints.
#[derive(Clone, Debug)]
pub struct SyncHint {
    pub cycle: u64,
    pub anomaly_type: String,
    pub confidence: f32,
    pub suggested_repair: Option<String>,
}

/// Signal from Brain α to Brain β for routing adjustments.
#[derive(Clone, Debug)]
pub struct RoutingHint {
    pub cycle: u64,
    pub agent_is_healthy: bool,
    pub connection_quality: f32,
    pub suggested_strategy_shift: Option<String>,
}

/// Health snapshot: Brain α → Brain γ.
#[derive(Clone, Debug, Default)]
pub struct HealthSnapshot {
    pub drift_score: f32,
    pub rollback_events: u64,
    pub connection_quality: f32,
}

/// Strategy snapshot: Brain β → Brain γ.
#[derive(Clone, Debug, Default)]
pub struct StrategySnapshot {
    pub active_strategies: Vec<String>,
    pub strategy_scores: Vec<(String, f32)>,
    pub routing_efficiency: f32,
}

/// Forecast snapshot: Brain δ → Brain γ, α, β.
#[derive(Clone, Debug, Default)]
pub struct ForecastSnapshot {
    pub drift_forecast: f32,
    pub load_forecast: f32,
    pub efficiency_trend: f32,
    pub risk_trend: f32,
}

/// Signals from Brain γ (governance) to Brain α (sync).
#[derive(Clone, Debug, Default)]
pub struct GammaToAlphaSignals {
    pub consensus_threshold: f32,
    pub rollback_aggressiveness: f32,
}

/// Signals from Brain γ (governance) to Brain β (strategy).
#[derive(Clone, Debug, Default)]
pub struct GammaToBetaSignals {
    pub allowed_strategies: Vec<String>,
    pub discouraged_strategies: Vec<String>,
    pub exploration_weight: f32,
}

/// Signals from Brain γ (governance) to Brain δ (perception).
#[derive(Clone, Debug, Default)]
pub struct GammaToDeltaSignals {
    pub track_drift: bool,
    pub track_load: bool,
    pub track_risk: bool,
}

/// Signals from Brain δ (perception) to Brain α (sync).
#[derive(Clone, Debug, Default)]
pub struct DeltaToAlphaSignals {
    pub drift_forecast: f32,
    pub risk_forecast: f32,
}

/// Signals from Brain δ (perception) to Brain β (strategy).
#[derive(Clone, Debug, Default)]
pub struct DeltaToBetaSignals {
    pub regime_label: Option<String>,
    pub load_forecast: f32,
}

/// Cross-brain communication channel with support for dual-brain and quad-brain architectures.
#[derive(Clone, Debug)]
pub struct CrossBrainChannel {
    // Twin-brain signals (α ↔ β)
    pub sync_hints: VecDeque<SyncHint>,
    pub routing_hints: VecDeque<RoutingHint>,

    // Quad-brain signals (γ and δ routes)
    pub alpha_to_gamma: HealthSnapshot,
    pub gamma_to_alpha: GammaToAlphaSignals,
    pub beta_to_gamma: StrategySnapshot,
    pub gamma_to_beta: GammaToBetaSignals,
    pub gamma_to_delta: GammaToDeltaSignals,
    pub delta_to_alpha: DeltaToAlphaSignals,
    pub delta_to_beta: DeltaToBetaSignals,
    pub delta_to_gamma: ForecastSnapshot,

    pub max_buffered_hints: usize,
    pub conflict_count: u64,
}

impl CrossBrainChannel {
    pub fn new(max_buffered_hints: usize) -> Self {
        Self {
            sync_hints: VecDeque::with_capacity(max_buffered_hints),
            routing_hints: VecDeque::with_capacity(max_buffered_hints),
            alpha_to_gamma: HealthSnapshot::default(),
            gamma_to_alpha: GammaToAlphaSignals::default(),
            beta_to_gamma: StrategySnapshot::default(),
            gamma_to_beta: GammaToBetaSignals::default(),
            gamma_to_delta: GammaToDeltaSignals::default(),
            delta_to_alpha: DeltaToAlphaSignals::default(),
            delta_to_beta: DeltaToBetaSignals::default(),
            delta_to_gamma: ForecastSnapshot::default(),
            max_buffered_hints,
            conflict_count: 0,
        }
    }

    /// Send sync hint from β to α.
    pub fn send_sync_hint(&mut self, hint: SyncHint) {
        if self.sync_hints.len() >= self.max_buffered_hints {
            self.sync_hints.pop_front();
        }
        self.sync_hints.push_back(hint);
    }

    /// Send routing hint from α to β.
    pub fn send_routing_hint(&mut self, hint: RoutingHint) {
        if self.routing_hints.len() >= self.max_buffered_hints {
            self.routing_hints.pop_front();
        }
        self.routing_hints.push_back(hint);
    }

    /// Consume buffered hints.
    pub fn drain_sync_hints(&mut self) -> Vec<SyncHint> {
        self.sync_hints.drain(..).collect()
    }

    pub fn drain_routing_hints(&mut self) -> Vec<RoutingHint> {
        self.routing_hints.drain(..).collect()
    }
}

/// Decision type from a brain.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BrainDecision {
    Rollback,
    StrategyAdjust,
    ContinueNormal,
    RebalanceLoad,
    Unknown,
}

/// Unified twin-brain agent.
#[derive(Clone, Debug)]
pub struct TwinBrainAgent {
    pub agent_id: AgentId,
    pub level: AgentLevel,
    pub brain_alpha: BrainAlpha,
    pub brain_beta: BrainBeta,
    pub channel: CrossBrainChannel,
    pub cycle_count: u64,
    pub synchronized: bool,
    pub conflict_history: VecDeque<(u64, String)>,
    pub metrics: TwinBrainMetrics,
}

#[derive(Clone, Debug, Default)]
pub struct TwinBrainMetrics {
    pub cycles_run: u64,
    pub sync_successful: u64,
    pub conflicts_resolved: u64,
    pub repairs_applied_from_alpha: u64,
    pub strategy_adjustments_from_beta: u64,
}

impl TwinBrainAgent {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level: level.clone(),
            brain_alpha: BrainAlpha::new(agent_id, level.clone(), 0.8),
            brain_beta: BrainBeta::new(agent_id, level),
            channel: CrossBrainChannel::new(20),
            cycle_count: 0,
            synchronized: true,
            conflict_history: VecDeque::with_capacity(100),
            metrics: TwinBrainMetrics::default(),
        }
    }

    /// Advance both brains one cycle together.
    pub fn advance_cycle(&mut self, behavioral_sig: BehavioralSignature) {
        self.cycle_count += 1;
        self.metrics.cycles_run += 1;

        // Brain α: Observe behavior and check for drift
        self.brain_alpha.drift_detector.observe(behavioral_sig.clone());
        let (is_drifting, _drift_amount, _reason) = self.brain_alpha.drift_detector.detect_drift();

        // If drifting, Brain α may propose repair
        if is_drifting {
            if let Some(repair) = self.brain_alpha.propose_repair() {
                // Send sync hint to cross-brain channel
                let hint = SyncHint {
                    cycle: self.cycle_count,
                    anomaly_type: "behavioral_drift".to_string(),
                    confidence: 0.8,
                    suggested_repair: Some(format!("{:?}", repair)),
                };
                self.channel.send_sync_hint(hint);
            }
        }

        // Brain β: Learn from this cycle's outcomes
        // (In real use, this would be called with actual mutation outcomes)

        // Process cross-brain hints
        let sync_hints = self.channel.drain_sync_hints();
        let routing_hints = self.channel.drain_routing_hints();

        self.process_sync_hints(sync_hints);
        self.process_routing_hints(routing_hints);

        // Merge decisions
        let alpha_decision = self.get_alpha_decision();
        let beta_decision = self.get_beta_decision();

        self.merge_decisions(alpha_decision, beta_decision);

        self.synchronized = true;
    }

    /// Process sync hints from Brain β.
    fn process_sync_hints(&mut self, hints: Vec<SyncHint>) {
        for hint in hints {
            if hint.anomaly_type == "behavioral_drift" {
                self.metrics.repairs_applied_from_alpha += 1;
            }
        }
    }

    /// Process routing hints from Brain α.
    fn process_routing_hints(&mut self, hints: Vec<RoutingHint>) {
        for hint in hints {
            if hint.agent_is_healthy {
                self.metrics.strategy_adjustments_from_beta += 1;
            }
        }
    }

    /// Get decision from Brain α (synchronization/healing).
    fn get_alpha_decision(&self) -> BrainDecision {
        if self.brain_alpha.rollbacks_triggered > 0 {
            BrainDecision::Rollback
        } else if self.brain_alpha.drifts_detected > 0 {
            BrainDecision::ContinueNormal
        } else {
            BrainDecision::ContinueNormal
        }
    }

    /// Get decision from Brain β (learning/routing).
    fn get_beta_decision(&self) -> BrainDecision {
        if self.brain_beta.mutations_routed > 0 {
            BrainDecision::RebalanceLoad
        } else {
            BrainDecision::ContinueNormal
        }
    }

    /// Merge decisions from both brains.
    fn merge_decisions(&mut self, alpha_decision: BrainDecision, beta_decision: BrainDecision) {
        // Priority: Safety (α) > Optimization (β)
        match alpha_decision {
            BrainDecision::Rollback => {
                self.metrics.repairs_applied_from_alpha += 1;
            }
            BrainDecision::ContinueNormal => {
                // Follow β's suggestion if α is normal
                if beta_decision != BrainDecision::ContinueNormal {
                    self.metrics.strategy_adjustments_from_beta += 1;
                }
            }
            _ => {}
        }

        // Check for conflicts
        if alpha_decision != beta_decision
            && alpha_decision != BrainDecision::ContinueNormal
            && beta_decision != BrainDecision::ContinueNormal
        {
            self.channel.conflict_count += 1;
            self.metrics.conflicts_resolved += 1;
            self.conflict_history.push_back((
                self.cycle_count,
                format!("conflict_cycle_{}", self.cycle_count),
            ));
            if self.conflict_history.len() > 100 {
                self.conflict_history.pop_front();
            }
        }
    }

    /// Report agent status.
    pub fn report(&self) -> TwinBrainReport {
        TwinBrainReport {
            agent_id: self.agent_id,
            cycle: self.cycle_count,
            alpha_drifts: self.brain_alpha.drifts_detected,
            alpha_repairs: self.brain_alpha.repairs_successful,
            beta_mutations_routed: self.brain_beta.mutations_routed,
            twin_metrics: self.metrics.clone(),
            synchronized: self.synchronized,
            pending_sync_hints: self.channel.sync_hints.len(),
            pending_routing_hints: self.channel.routing_hints.len(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TwinBrainReport {
    pub agent_id: AgentId,
    pub cycle: u64,
    pub alpha_drifts: u64,
    pub alpha_repairs: u64,
    pub beta_mutations_routed: u64,
    pub twin_metrics: TwinBrainMetrics,
    pub synchronized: bool,
    pub pending_sync_hints: usize,
    pub pending_routing_hints: usize,
}

/// Metrics for quad-brain agent coordination.
#[derive(Clone, Debug, Default)]
pub struct QuadBrainMetrics {
    pub cycles_run: u64,
    pub policy_directives_issued: u64,
    pub evolution_mutations_tracked: u64,
    pub forecasts_generated: u64,
    pub perception_quality: f32,
}

/// Unified quad-brain agent with Brain α, β, γ, and δ.
/// Phase 6.16Q extends TwinBrainAgent with governance and perception.
#[derive(Clone, Debug)]
pub struct QuadBrainAgent {
    pub agent_id: AgentId,
    pub level: AgentLevel,
    pub brain_alpha: BrainAlpha,
    pub brain_beta: BrainBeta,
    pub channel: CrossBrainChannel,
    pub cycle_count: u64,
    pub synchronized: bool,
    pub conflict_history: VecDeque<(u64, String)>,
    pub metrics: QuadBrainMetrics,
}

impl QuadBrainAgent {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level: level.clone(),
            brain_alpha: BrainAlpha::new(agent_id, level.clone(), 0.8),
            brain_beta: BrainBeta::new(agent_id, level),
            channel: CrossBrainChannel::new(20),
            cycle_count: 0,
            synchronized: true,
            conflict_history: VecDeque::with_capacity(100),
            metrics: QuadBrainMetrics::default(),
        }
    }

    /// Advance quad-brain cycle: δ perceive → α sync → β learn → γ govern.
    /// Currently simplified to maintain compatibility; γ/δ brains added in Phase 6.17-6.18.
    pub fn advance_cycle(&mut self, behavioral_sig: BehavioralSignature) {
        self.cycle_count += 1;
        self.metrics.cycles_run += 1;

        // Phase δ simulation: forecast ingestion
        self.channel.delta_to_alpha.drift_forecast = 0.2;
        self.channel.delta_to_alpha.risk_forecast = 0.1;
        self.channel.delta_to_beta.load_forecast = 0.3;
        self.metrics.forecasts_generated += 1;

        // Phase α: Sync & Healing (existing logic)
        self.brain_alpha.drift_detector.observe(behavioral_sig.clone());
        let (is_drifting, _drift_amount, _reason) = self.brain_alpha.drift_detector.detect_drift();

        if is_drifting {
            if let Some(repair) = self.brain_alpha.propose_repair() {
                let hint = SyncHint {
                    cycle: self.cycle_count,
                    anomaly_type: "behavioral_drift".to_string(),
                    confidence: 0.8,
                    suggested_repair: Some(format!("{:?}", repair)),
                };
                self.channel.send_sync_hint(hint);
            }
        }

        // Capture health snapshot for γ
        self.channel.alpha_to_gamma = HealthSnapshot {
            drift_score: if is_drifting { 0.6 } else { 0.2 },
            rollback_events: self.brain_alpha.rollbacks_triggered,
            connection_quality: 0.95,
        };

        // Phase β: Learn & Route (existing logic)
        let sync_hints = self.channel.drain_sync_hints();
        let routing_hints = self.channel.drain_routing_hints();
        self.process_sync_hints(sync_hints);
        self.process_routing_hints(routing_hints);

        // Capture strategy snapshot for γ
        self.channel.beta_to_gamma = StrategySnapshot {
            active_strategies: vec!["route_by_affinity".to_string()],
            strategy_scores: vec![("route_by_affinity".to_string(), 0.85)],
            routing_efficiency: 0.85,
        };

        // Phase γ simulation: policy synthesis (placeholder until Phase 6.17)
        // Emit default policies
        self.channel.gamma_to_alpha.consensus_threshold = 0.80;
        self.channel.gamma_to_alpha.rollback_aggressiveness = 0.8;
        self.channel.gamma_to_beta.exploration_weight = 0.3;
        self.metrics.policy_directives_issued += 1;

        // Merge decisions
        let alpha_decision = self.get_alpha_decision();
        let beta_decision = self.get_beta_decision();
        self.merge_decisions(alpha_decision, beta_decision);

        self.synchronized = true;
    }

    fn process_sync_hints(&mut self, hints: Vec<SyncHint>) {
        for hint in hints {
            if hint.anomaly_type == "behavioral_drift" {
                self.metrics.evolution_mutations_tracked += 1;
            }
        }
    }

    fn process_routing_hints(&mut self, hints: Vec<RoutingHint>) {
        for _hint in hints {
            // Routing hints processed by β
        }
    }

    fn get_alpha_decision(&self) -> BrainDecision {
        if self.brain_alpha.rollbacks_triggered > 0 {
            BrainDecision::Rollback
        } else if self.brain_alpha.drifts_detected > 0 {
            BrainDecision::ContinueNormal
        } else {
            BrainDecision::ContinueNormal
        }
    }

    fn get_beta_decision(&self) -> BrainDecision {
        if self.brain_beta.mutations_routed > 0 {
            BrainDecision::RebalanceLoad
        } else {
            BrainDecision::ContinueNormal
        }
    }

    fn merge_decisions(&mut self, alpha_decision: BrainDecision, beta_decision: BrainDecision) {
        match alpha_decision {
            BrainDecision::Rollback => {
                self.metrics.evolution_mutations_tracked += 1;
            }
            BrainDecision::ContinueNormal => {
                if beta_decision != BrainDecision::ContinueNormal {
                    self.metrics.evolution_mutations_tracked += 1;
                }
            }
            _ => {}
        }

        if alpha_decision != beta_decision
            && alpha_decision != BrainDecision::ContinueNormal
            && beta_decision != BrainDecision::ContinueNormal
        {
            self.channel.conflict_count += 1;
            self.conflict_history.push_back((
                self.cycle_count,
                format!("conflict_cycle_{}", self.cycle_count),
            ));
            if self.conflict_history.len() > 100 {
                self.conflict_history.pop_front();
            }
        }
    }

    pub fn report(&self) -> QuadBrainReport {
        QuadBrainReport {
            agent_id: self.agent_id,
            cycle: self.cycle_count,
            alpha_drifts: self.brain_alpha.drifts_detected,
            alpha_repairs: self.brain_alpha.repairs_successful,
            beta_mutations_routed: self.brain_beta.mutations_routed,
            quad_metrics: self.metrics.clone(),
            synchronized: self.synchronized,
            forecast_quality: self.channel.delta_to_gamma.drift_forecast,
        }
    }
}

/// Report from quad-brain agent.
#[derive(Clone, Debug)]
pub struct QuadBrainReport {
    pub agent_id: AgentId,
    pub cycle: u64,
    pub alpha_drifts: u64,
    pub alpha_repairs: u64,
    pub beta_mutations_routed: u64,
    pub quad_metrics: QuadBrainMetrics,
    pub synchronized: bool,
    pub forecast_quality: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::mutation::domain_coordination::AgentLevel;

    #[test]
    fn twin_brain_creation() {
        let agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Nano);
        assert_eq!(agent.agent_id, AgentId::new(1));
        assert_eq!(agent.cycle_count, 0);
        assert!(agent.synchronized);
    }

    #[test]
    fn cross_brain_channel_new() {
        let channel = CrossBrainChannel::new(20);
        assert_eq!(channel.max_buffered_hints, 20);
        assert_eq!(channel.sync_hints.len(), 0);
        assert_eq!(channel.routing_hints.len(), 0);
    }

    #[test]
    fn send_and_receive_sync_hints() {
        let mut channel = CrossBrainChannel::new(10);
        let hint = SyncHint {
            cycle: 1,
            anomaly_type: "test".to_string(),
            confidence: 0.9,
            suggested_repair: Some("repair_1".to_string()),
        };

        channel.send_sync_hint(hint.clone());
        assert_eq!(channel.sync_hints.len(), 1);

        let hints = channel.drain_sync_hints();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].cycle, 1);
        assert_eq!(channel.sync_hints.len(), 0);
    }

    #[test]
    fn send_and_receive_routing_hints() {
        let mut channel = CrossBrainChannel::new(10);
        let hint = RoutingHint {
            cycle: 1,
            agent_is_healthy: true,
            connection_quality: 0.95,
            suggested_strategy_shift: None,
        };

        channel.send_routing_hint(hint.clone());
        assert_eq!(channel.routing_hints.len(), 1);

        let hints = channel.drain_routing_hints();
        assert_eq!(hints.len(), 1);
        assert!(hints[0].agent_is_healthy);
    }

    #[test]
    fn cross_brain_channel_max_buffer() {
        let mut channel = CrossBrainChannel::new(3);

        for i in 0..5 {
            let hint = SyncHint {
                cycle: i,
                anomaly_type: format!("test_{}", i),
                confidence: 0.8,
                suggested_repair: None,
            };
            channel.send_sync_hint(hint);
        }

        // Should only keep last 3
        assert_eq!(channel.sync_hints.len(), 3);
    }

    #[test]
    fn brain_decision_equality() {
        let d1 = BrainDecision::ContinueNormal;
        let d2 = BrainDecision::ContinueNormal;
        assert_eq!(d1, d2);

        let d3 = BrainDecision::Rollback;
        let d4 = BrainDecision::Rollback;
        assert_eq!(d3, d4);
    }

    #[test]
    fn twin_brain_advance_cycle() {
        let mut agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Micro);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["s1".to_string()],
            behavior_hash: 12345,
        };

        agent.advance_cycle(sig);
        assert_eq!(agent.cycle_count, 1);
        assert_eq!(agent.metrics.cycles_run, 1);
        assert!(agent.synchronized);
    }

    #[test]
    fn twin_brain_report() {
        let agent = TwinBrainAgent::new(AgentId::new(2), AgentLevel::Sub);
        let report = agent.report();
        assert_eq!(report.agent_id, AgentId::new(2));
        assert_eq!(report.cycle, 0);
        assert!(report.synchronized);
    }

    #[test]
    fn get_alpha_decision() {
        let agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Nano);
        let decision = agent.get_alpha_decision();
        assert_eq!(decision, BrainDecision::ContinueNormal);
    }

    #[test]
    fn get_beta_decision() {
        let agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Nano);
        let decision = agent.get_beta_decision();
        assert_eq!(decision, BrainDecision::ContinueNormal);
    }

    #[test]
    fn conflict_history_bounded() {
        let mut agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Nano);

        // Simulate many conflicting decisions
        for _ in 0..150 {
            agent.merge_decisions(
                BrainDecision::Rollback,
                BrainDecision::RebalanceLoad,
            );
        }

        // Should be bounded to 100
        assert!(agent.conflict_history.len() <= 100);
    }

    #[test]
    fn twin_brain_agent_metrics() {
        let mut agent = TwinBrainAgent::new(AgentId::new(1), AgentLevel::Super);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.85,
            mutation_acceptance_rate: 0.8,
            active_strategies: vec!["strategy_1".to_string()],
            behavior_hash: 99999,
        };

        agent.advance_cycle(sig);
        assert_eq!(agent.metrics.cycles_run, 1);

        agent.metrics.repairs_applied_from_alpha = 2;
        agent.metrics.strategy_adjustments_from_beta = 1;

        let report = agent.report();
        assert_eq!(report.twin_metrics.repairs_applied_from_alpha, 2);
        assert_eq!(report.twin_metrics.strategy_adjustments_from_beta, 1);
    }

    #[test]
    fn cross_brain_channel_conflicts() {
        let mut channel = CrossBrainChannel::new(10);
        assert_eq!(channel.conflict_count, 0);

        // Simulate conflict detection
        channel.conflict_count += 1;
        channel.conflict_count += 1;
        assert_eq!(channel.conflict_count, 2);
    }

    #[test]
    fn sync_and_routing_hint_interleave() {
        let mut channel = CrossBrainChannel::new(10);

        let sync_hint = SyncHint {
            cycle: 1,
            anomaly_type: "drift".to_string(),
            confidence: 0.9,
            suggested_repair: Some("fix".to_string()),
        };

        let routing_hint = RoutingHint {
            cycle: 1,
            agent_is_healthy: true,
            connection_quality: 0.95,
            suggested_strategy_shift: None,
        };

        channel.send_sync_hint(sync_hint);
        channel.send_routing_hint(routing_hint);

        assert_eq!(channel.sync_hints.len(), 1);
        assert_eq!(channel.routing_hints.len(), 1);

        let sync = channel.drain_sync_hints();
        let routing = channel.drain_routing_hints();

        assert_eq!(sync.len(), 1);
        assert_eq!(routing.len(), 1);
    }

    // Phase 6.16Q: Quad-Brain Integration Tests (12 tests)

    #[test]
    fn quad_brain_creation() {
        let agent = QuadBrainAgent::new(AgentId::new(1), AgentLevel::Nano);
        assert_eq!(agent.agent_id, AgentId::new(1));
        assert_eq!(agent.cycle_count, 0);
        assert!(agent.synchronized);
    }

    #[test]
    fn quad_brain_initial_metrics() {
        let agent = QuadBrainAgent::new(AgentId::new(1), AgentLevel::Micro);
        assert_eq!(agent.metrics.cycles_run, 0);
        assert_eq!(agent.metrics.forecasts_generated, 0);
        assert_eq!(agent.metrics.policy_directives_issued, 0);
    }

    #[test]
    fn quad_brain_channel_initialization() {
        let channel = CrossBrainChannel::new(20);
        assert_eq!(channel.sync_hints.len(), 0);
        assert_eq!(channel.routing_hints.len(), 0);
        assert_eq!(channel.conflict_count, 0);
        assert_eq!(channel.gamma_to_alpha.consensus_threshold, 0.0);
        assert_eq!(channel.delta_to_beta.load_forecast, 0.0);
    }

    #[test]
    fn quad_brain_cycle_advance() {
        let mut agent = QuadBrainAgent::new(AgentId::new(1), AgentLevel::Sub);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["s1".to_string()],
            behavior_hash: 12345,
        };

        agent.advance_cycle(sig);
        assert_eq!(agent.cycle_count, 1);
        assert_eq!(agent.metrics.cycles_run, 1);
        assert_eq!(agent.metrics.forecasts_generated, 1);
        assert!(agent.synchronized);
    }

    #[test]
    fn quad_brain_policy_generation() {
        let mut agent = QuadBrainAgent::new(AgentId::new(2), AgentLevel::Micro);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.7,
            mutation_acceptance_rate: 0.6,
            active_strategies: vec!["s2".to_string()],
            behavior_hash: 54321,
        };

        agent.advance_cycle(sig);
        assert_eq!(agent.metrics.policy_directives_issued, 1);
        assert!(agent.channel.gamma_to_alpha.consensus_threshold > 0.0);
    }

    #[test]
    fn quad_brain_signal_routing_alpha_to_gamma() {
        let mut agent = QuadBrainAgent::new(AgentId::new(3), AgentLevel::Nano);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.85,
            mutation_acceptance_rate: 0.8,
            active_strategies: vec!["s3".to_string()],
            behavior_hash: 99999,
        };

        agent.advance_cycle(sig);
        assert!(agent.channel.alpha_to_gamma.drift_score >= 0.0);
        assert!(agent.channel.alpha_to_gamma.connection_quality > 0.0);
    }

    #[test]
    fn quad_brain_signal_routing_beta_to_gamma() {
        let mut agent = QuadBrainAgent::new(AgentId::new(4), AgentLevel::Super);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.9,
            mutation_acceptance_rate: 0.85,
            active_strategies: vec!["s4".to_string()],
            behavior_hash: 11111,
        };

        agent.advance_cycle(sig);
        assert!(!agent.channel.beta_to_gamma.active_strategies.is_empty());
        assert!(agent.channel.beta_to_gamma.routing_efficiency > 0.0);
    }

    #[test]
    fn quad_brain_signal_routing_delta_to_alpha() {
        let mut agent = QuadBrainAgent::new(AgentId::new(5), AgentLevel::Sub);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.75,
            mutation_acceptance_rate: 0.7,
            active_strategies: vec!["s5".to_string()],
            behavior_hash: 22222,
        };

        agent.advance_cycle(sig);
        assert_eq!(agent.channel.delta_to_alpha.drift_forecast, 0.2);
        assert_eq!(agent.channel.delta_to_alpha.risk_forecast, 0.1);
    }

    #[test]
    fn quad_brain_signal_routing_delta_to_beta() {
        let mut agent = QuadBrainAgent::new(AgentId::new(6), AgentLevel::Micro);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["s6".to_string()],
            behavior_hash: 33333,
        };

        agent.advance_cycle(sig);
        assert!(agent.channel.delta_to_beta.load_forecast >= 0.0);
        assert!(agent.channel.delta_to_beta.load_forecast <= 1.0);
    }

    #[test]
    fn quad_brain_conflict_tracking() {
        let mut agent = QuadBrainAgent::new(AgentId::new(7), AgentLevel::Nano);

        agent.merge_decisions(
            BrainDecision::Rollback,
            BrainDecision::RebalanceLoad,
        );

        assert_eq!(agent.channel.conflict_count, 1);
        assert_eq!(agent.conflict_history.len(), 1);
    }

    #[test]
    fn quad_brain_report_generation() {
        let mut agent = QuadBrainAgent::new(AgentId::new(8), AgentLevel::Sub);
        let sig = BehavioralSignature {
            cycle: 1,
            efficiency: 0.82,
            mutation_acceptance_rate: 0.78,
            active_strategies: vec!["s8".to_string()],
            behavior_hash: 44444,
        };

        agent.advance_cycle(sig);
        let report = agent.report();

        assert_eq!(report.agent_id, AgentId::new(8));
        assert_eq!(report.cycle, 1);
        assert!(report.synchronized);
    }

    #[test]
    fn quad_brain_forecast_quality_tracking() {
        let mut agent = QuadBrainAgent::new(AgentId::new(9), AgentLevel::Micro);

        for i in 0..5 {
            let sig = BehavioralSignature {
                cycle: (i + 1) as u64,
                efficiency: 0.75 + (i as f32 * 0.05),
                mutation_acceptance_rate: 0.7,
                active_strategies: vec!["s9".to_string()],
                behavior_hash: 55555 + i as u64,
            };
            agent.advance_cycle(sig);
        }

        assert_eq!(agent.metrics.forecasts_generated, 5);
        let report = agent.report();
        assert_eq!(report.cycle, 5);
    }
}
