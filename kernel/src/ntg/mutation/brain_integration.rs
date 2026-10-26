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
use super::brain_delta::{BrainDelta, ClusterMetrics, LocalMetrics, PerceptionSnapshot, SwarmMetrics};
use super::brain_gamma::{BrainGamma, ForecastSnapshot as GammaForecast};
use std::collections::{HashMap, VecDeque};

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
            level,
            brain_alpha: BrainAlpha::new(agent_id, level, 0.8),
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
            BrainDecision::ContinueNormal
                // Follow β's suggestion if α is normal
                if beta_decision != BrainDecision::ContinueNormal => {
                    self.metrics.strategy_adjustments_from_beta += 1;
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

/// Cycles used to learn α's efficiency baseline before drift detection is meaningful.
const BASELINE_WARMUP_CYCLES: u64 = 5;

/// Unified quad-brain agent with Brain α, β, γ, and δ.
///
/// One cycle runs δ (perceive) → α (sync/heal) → β (learn) → γ (govern), and
/// γ's output is applied to α on the *next* cycle, closing the loop. Every
/// signal on the [`CrossBrainChannel`] is computed from the observed
/// behaviour; nothing is a constant.
///
/// Limits (single-agent view): cluster and swarm aggregates equal this agent's
/// own observation unless the caller supplies real peer data through
/// [`QuadBrainAgent::advance_cycle_with_perception`]. γ's perception-focus
/// signal to δ is published on the channel but δ does not act on it yet.
#[derive(Clone, Debug)]
pub struct QuadBrainAgent {
    pub agent_id: AgentId,
    pub level: AgentLevel,
    pub brain_alpha: BrainAlpha,
    pub brain_beta: BrainBeta,
    pub brain_gamma: BrainGamma,
    pub brain_delta: BrainDelta,
    pub channel: CrossBrainChannel,
    pub cycle_count: u64,
    pub synchronized: bool,
    pub conflict_history: VecDeque<(u64, String)>,
    pub metrics: QuadBrainMetrics,
    /// Last observed drift ratio from α (0.0 until α has history).
    pub last_drift: f32,
    prev_efficiency: Option<f32>,
    prev_drift_forecast: f32,
    warmup_sum: f32,
    strategy_ids: HashMap<String, u64>,
}

impl QuadBrainAgent {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level,
            brain_alpha: BrainAlpha::new(agent_id, level, 0.8),
            brain_beta: BrainBeta::new(agent_id, level),
            brain_gamma: BrainGamma::new(agent_id, level),
            brain_delta: BrainDelta::new(agent_id, level),
            channel: CrossBrainChannel::new(20),
            cycle_count: 0,
            synchronized: true,
            conflict_history: VecDeque::with_capacity(100),
            metrics: QuadBrainMetrics::default(),
            last_drift: 0.0,
            prev_efficiency: None,
            prev_drift_forecast: 0.0,
            warmup_sum: 0.0,
            strategy_ids: HashMap::new(),
        }
    }

    /// Advance one cycle from a behavioural observation alone. Cluster and
    /// swarm metrics are this agent's own view (see the type docs).
    pub fn advance_cycle(&mut self, behavioral_sig: BehavioralSignature) {
        let load = ((self.channel.sync_hints.len() + self.channel.routing_hints.len()) as f32 / 100.0)
            .clamp(0.0, 1.0);
        let connection = self.brain_alpha.health_monitor.parent_connection_quality;
        let perception = PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: behavioral_sig.efficiency,
                queue_depth: self.channel.sync_hints.len() + self.channel.routing_hints.len(),
                mutation_rate: behavioral_sig.mutation_acceptance_rate,
                connection_quality: connection,
            },
            cluster_metrics: ClusterMetrics {
                avg_efficiency: behavioral_sig.efficiency,
                drift_distribution: vec![self.last_drift],
                load_distribution: vec![load],
                consensus_health: (1.0 - self.last_drift).clamp(0.0, 1.0),
            },
            swarm_metrics: SwarmMetrics {
                global_efficiency: behavioral_sig.efficiency,
                global_drift: self.last_drift,
                global_load: load,
                global_risk: (self.last_drift * 0.5).clamp(0.0, 1.0),
            },
            hormone_levels: Default::default(),
            timestamp_us: self.cycle_count,
        };
        self.advance_cycle_with_perception(behavioral_sig, perception);
    }

    /// Advance one cycle with caller-supplied perception (use this when real
    /// cluster/swarm aggregates are available).
    pub fn advance_cycle_with_perception(
        &mut self,
        behavioral_sig: BehavioralSignature,
        perception: PerceptionSnapshot,
    ) {
        self.cycle_count += 1;
        self.metrics.cycles_run += 1;

        // γ's policy from the previous cycle takes effect on α now.
        if self.channel.gamma_to_alpha.consensus_threshold > 0.0 {
            self.brain_alpha.consensus_threshold = self.channel.gamma_to_alpha.consensus_threshold;
        }

        // α's baseline is learned from the first cycles rather than assumed.
        if self.cycle_count <= BASELINE_WARMUP_CYCLES {
            self.warmup_sum += behavioral_sig.efficiency;
            self.brain_alpha.drift_detector.baseline_efficiency =
                self.warmup_sum / self.cycle_count as f32;
        }

        // δ: perceive → hormones → embeddings → forecast.
        self.brain_delta.ingest_signals(perception);
        self.brain_delta.update_hormones();
        self.brain_delta.update_representation();
        self.brain_delta.update_forecast();
        let (drift_f, load_f, eff_trend, risk_trend) = self.brain_delta.export_forecast_snapshot();
        self.channel.delta_to_alpha.drift_forecast = drift_f;
        self.channel.delta_to_alpha.risk_forecast = risk_trend;
        self.channel.delta_to_beta.regime_label = self.brain_delta.get_regime_label();
        self.channel.delta_to_beta.load_forecast = load_f;
        self.channel.delta_to_gamma = ForecastSnapshot {
            drift_forecast: drift_f,
            load_forecast: load_f,
            efficiency_trend: eff_trend,
            risk_trend,
        };
        self.metrics.forecasts_generated += 1;

        // α: observe, detect, repair.
        let (is_drifting, _reason) = self.brain_alpha.observe_behavior(behavioral_sig.clone());
        let (_, drift_amount, _) = self.brain_alpha.drift_detector.detect_drift();
        self.last_drift = drift_amount;
        if self.cycle_count > BASELINE_WARMUP_CYCLES && is_drifting {
            if let Some(repair) = self.brain_alpha.propose_repair() {
                let hint = SyncHint {
                    cycle: self.cycle_count,
                    anomaly_type: "behavioral_drift".to_string(),
                    confidence: drift_amount.clamp(0.0, 1.0),
                    suggested_repair: Some(format!("{:?}", repair)),
                };
                self.channel.send_sync_hint(hint);
                let _ = self.brain_alpha.apply_repair(repair);
            }
        }

        // How well did last cycle's drift forecast match what α then saw?
        self.metrics.perception_quality =
            1.0 - (self.prev_drift_forecast - drift_amount).abs().clamp(0.0, 1.0);
        self.prev_drift_forecast = drift_f;

        self.channel.alpha_to_gamma = HealthSnapshot {
            drift_score: drift_amount,
            rollback_events: self.brain_alpha.rollbacks_triggered,
            connection_quality: self.brain_alpha.health_monitor.parent_connection_quality,
        };

        // β: learn from the efficiency change attributed to the active strategies.
        let gain = self.prev_efficiency.map_or(0.0, |prev| behavioral_sig.efficiency - prev);
        self.prev_efficiency = Some(behavioral_sig.efficiency);
        for name in &behavioral_sig.active_strategies {
            let id = match self.strategy_ids.get(name) {
                Some(id) => *id,
                None => {
                    let id = self.brain_beta.strategy_optimizer.create_strategy(
                        name.clone(),
                        "observed".to_string(),
                        Vec::new(),
                    );
                    self.strategy_ids.insert(name.clone(), id);
                    id
                }
            };
            self.brain_beta.strategy_optimizer.record_return(id, gain);
            if gain >= 0.0 {
                self.brain_beta.learn_success("observed".to_string(), name.clone(), gain);
            } else if let Some(pid) = self
                .brain_beta
                .pattern_learner
                .patterns
                .values()
                .find(|p| p.domain == "observed" && &p.mutation_type == name)
                .map(|p| p.pattern_id)
            {
                self.brain_beta.learn_failure(pid);
            }
        }
        let sync_hints = self.channel.drain_sync_hints();
        let routing_hints = self.channel.drain_routing_hints();
        self.process_sync_hints(sync_hints);
        self.process_routing_hints(routing_hints);

        let optimizer = &self.brain_beta.strategy_optimizer;
        let mut named: Vec<(String, f32)> = optimizer
            .strategies
            .values()
            .filter(|st| st.active)
            .map(|st| (st.name.clone(), optimizer.compute_sharpe_ratio(st.strategy_id)))
            .collect();
        named.sort_by(|a, b| a.0.cmp(&b.0));
        let routed = self.brain_beta.mutations_routed;
        self.channel.beta_to_gamma = StrategySnapshot {
            active_strategies: named.iter().map(|(n, _)| n.clone()).collect(),
            strategy_scores: named,
            // 0.0 means "no routing observed", not "routing failed".
            routing_efficiency: if routed > 0 {
                self.brain_beta.successful_routes as f32 / routed as f32
            } else {
                0.0
            },
        };

        // γ: synthesize policy from the three real snapshots.
        self.brain_gamma.update_inputs(
            self.channel.alpha_to_gamma.clone(),
            self.channel.beta_to_gamma.clone(),
            GammaForecast {
                drift_forecast: drift_f,
                load_forecast: load_f,
                efficiency_trend: eff_trend,
                risk_trend,
            },
        );
        self.brain_gamma.evaluate_alignment();
        if let Ok(policies) = self.brain_gamma.synthesize_policies() {
            self.metrics.policy_directives_issued += policies.len() as u64;
        }
        self.brain_gamma.update_evolution_plan();
        let (consensus_threshold, rollback_aggressiveness) = self.brain_gamma.get_sync_policy();
        self.channel.gamma_to_alpha = GammaToAlphaSignals { consensus_threshold, rollback_aggressiveness };
        let (allowed, exploration_weight) = self.brain_gamma.get_strategy_policy();
        self.channel.gamma_to_beta = GammaToBetaSignals {
            allowed_strategies: allowed,
            discouraged_strategies: Vec::new(),
            exploration_weight,
        };
        let (track_drift, track_load, track_risk) = self.brain_gamma.get_perception_focus();
        self.channel.gamma_to_delta = GammaToDeltaSignals { track_drift, track_load, track_risk };

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
            BrainDecision::ContinueNormal
                if beta_decision != BrainDecision::ContinueNormal => {
                    self.metrics.evolution_mutations_tracked += 1;
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
            forecast_quality: self.metrics.perception_quality,
            drift_score: self.last_drift,
            gamma_policies_active: self.brain_gamma.active_policies.len(),
            gamma_alignment: self.brain_gamma.metrics.alignment_score,
            delta_regime: self.brain_delta.get_regime_label(),
            consensus_threshold: self.brain_alpha.consensus_threshold,
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
    /// 1 - |previous drift forecast - drift α then observed|, in [0,1].
    pub forecast_quality: f32,
    pub drift_score: f32,
    pub gamma_policies_active: usize,
    pub gamma_alignment: f32,
    pub delta_regime: Option<String>,
    /// α's consensus threshold as currently set by γ.
    pub consensus_threshold: f32,
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
        // One calm observation gives γ nothing to react to.
        assert_eq!(agent.metrics.policy_directives_issued, 0);
        assert!((agent.channel.gamma_to_alpha.consensus_threshold - 0.80).abs() < 1e-6);
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
        assert_eq!(agent.channel.beta_to_gamma.active_strategies, vec!["s4".to_string()]);
        // No mutations were routed, so there is no routing efficiency to report.
        assert_eq!(agent.channel.beta_to_gamma.routing_efficiency, 0.0);
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
        // δ needs two observations before it can forecast anything.
        assert_eq!(agent.channel.delta_to_alpha.drift_forecast, 0.0);
        assert_eq!(agent.channel.delta_to_alpha.risk_forecast, 0.0);
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

    fn sig(cycle: u64, efficiency: f32, strategy: &str) -> BehavioralSignature {
        BehavioralSignature {
            cycle,
            efficiency,
            mutation_acceptance_rate: 0.7,
            active_strategies: vec![strategy.to_string()],
            behavior_hash: cycle.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        }
    }

    #[test]
    fn quad_brain_stable_behaviour_is_quiet() {
        let mut agent = QuadBrainAgent::new(AgentId::new(20), AgentLevel::Nano);
        for i in 1..=30 {
            agent.advance_cycle(sig(i, 0.9, "steady"));
        }
        let r = agent.report();
        assert_eq!(r.alpha_drifts, 0, "constant behaviour must not register as drift");
        assert_eq!(r.quad_metrics.policy_directives_issued, 0);
        assert_eq!(r.gamma_policies_active, 0);
        assert!((r.consensus_threshold - 0.80).abs() < 1e-6);
        assert_ne!(r.delta_regime.as_deref(), Some("stressed"));
    }

    #[test]
    fn quad_brain_learns_baseline_instead_of_assuming_one() {
        // Efficiency 1.0 is 25% above the old hard-coded 0.8 baseline; after
        // warm-up the baseline must follow the agent, so no drift is reported.
        let mut agent = QuadBrainAgent::new(AgentId::new(21), AgentLevel::Micro);
        for i in 1..=20 {
            agent.advance_cycle(sig(i, 1.0, "perfect"));
        }
        assert!((agent.brain_alpha.drift_detector.baseline_efficiency - 1.0).abs() < 1e-6);
        assert_eq!(agent.report().alpha_drifts, 0);
    }

    #[test]
    fn quad_brain_closed_loop_drift_tightens_consensus() {
        let mut agent = QuadBrainAgent::new(AgentId::new(22), AgentLevel::Sub);
        for i in 1..=8 {
            agent.advance_cycle(sig(i, 0.9, "steady"));
        }
        assert_eq!(agent.report().alpha_drifts, 0);
        assert!((agent.brain_alpha.consensus_threshold - 0.80).abs() < 1e-6);

        // Sudden regression: efficiency collapses.
        for i in 9..=24 {
            agent.advance_cycle(sig(i, 0.3, "regressed"));
        }
        let r = agent.report();
        assert!(r.alpha_drifts > 0, "α must notice the collapse");
        assert!(agent.channel.alpha_to_gamma.drift_score > 0.4, "drift reaches γ as a real value");
        assert!(r.quad_metrics.policy_directives_issued > 0, "γ must react");
        // γ's tightening is applied by α on a following cycle (the loop closes).
        assert!(
            agent.brain_alpha.consensus_threshold > 0.80,
            "consensus threshold was {}",
            agent.brain_alpha.consensus_threshold
        );
    }

    #[test]
    fn quad_brain_beta_credits_strategies_with_observed_gain() {
        let mut agent = QuadBrainAgent::new(AgentId::new(23), AgentLevel::Nano);
        agent.advance_cycle(sig(1, 0.5, "a"));
        agent.advance_cycle(sig(2, 0.6, "a"));
        agent.advance_cycle(sig(3, 0.7, "a"));
        let scores = &agent.channel.beta_to_gamma.strategy_scores;
        assert_eq!(scores.len(), 1);
        assert!(scores[0].1 > 0.0, "improving strategy should have positive Sharpe");
        assert!(agent.brain_beta.pattern_discoveries >= 3);
    }

    #[test]
    fn quad_brain_is_deterministic() {
        let run = || {
            let mut a = QuadBrainAgent::new(AgentId::new(24), AgentLevel::Micro);
            for i in 1..=20 {
                let eff = if i < 10 { 0.9 } else { 0.4 };
                a.advance_cycle(sig(i, eff, "x"));
            }
            let r = a.report();
            (r.alpha_drifts, r.quad_metrics.policy_directives_issued, r.drift_score.to_bits(), r.delta_regime)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn quad_brain_forecast_quality_is_in_range() {
        let mut agent = QuadBrainAgent::new(AgentId::new(25), AgentLevel::Super);
        for i in 1..=15 {
            agent.advance_cycle(sig(i, if i % 2 == 0 { 0.9 } else { 0.5 }, "osc"));
            let q = agent.report().forecast_quality;
            assert!((0.0..=1.0).contains(&q), "forecast_quality {q}");
        }
    }
}
