//! Brain δ: Perception, Representation & Forecasting Engine
//!
//! Phase 6.18 implements the perception and prediction layer:
//! - Local, cluster, and swarm metrics aggregation
//! - Hormone-like system signals (stress, growth, repair, coordination)
//! - Embedding-based representation learning
//! - Regime detection and classification
//! - Forward-looking forecasting with linear extrapolation
//! - Signal export for α, β, γ consumption

use crate::ntg::mutation::domain_coordination::{AgentId, AgentLevel};
use std::collections::VecDeque;

/// Local metrics from this agent.
#[derive(Clone, Debug, Default)]
pub struct LocalMetrics {
    pub efficiency: f32,
    pub queue_depth: usize,
    pub mutation_rate: f32,
    pub connection_quality: f32,
}

/// Aggregate metrics from cluster.
#[derive(Clone, Debug, Default)]
pub struct ClusterMetrics {
    pub avg_efficiency: f32,
    pub drift_distribution: Vec<f32>,
    pub load_distribution: Vec<f32>,
    pub consensus_health: f32,
}

/// Aggregate metrics from swarm.
#[derive(Clone, Debug, Default)]
pub struct SwarmMetrics {
    pub global_efficiency: f32,
    pub global_drift: f32,
    pub global_load: f32,
    pub global_risk: f32,
}

/// Hormone-like system signals (0.0-1.0 range).
#[derive(Clone, Debug, Default)]
pub struct HormoneLevels {
    pub stress: f32,
    pub growth: f32,
    pub repair: f32,
    pub coordination: f32,
}

/// Unified perception snapshot combining all metric levels.
#[derive(Clone, Debug)]
#[derive(Default)]
pub struct PerceptionSnapshot {
    pub local_metrics: LocalMetrics,
    pub cluster_metrics: ClusterMetrics,
    pub swarm_metrics: SwarmMetrics,
    pub hormone_levels: HormoneLevels,
    pub timestamp_us: u64,
}


/// Embedding representation of agent + cluster + swarm state.
#[derive(Clone, Debug)]
pub struct RepresentationState {
    pub agent_embedding: Vec<f32>,
    pub cluster_embedding: Vec<f32>,
    pub swarm_embedding: Vec<f32>,
    pub regime_label: Option<String>,
}

impl Default for RepresentationState {
    fn default() -> Self {
        Self {
            agent_embedding: vec![0.5; 16],
            cluster_embedding: vec![0.5; 8],
            swarm_embedding: vec![0.5; 8],
            regime_label: Some("normal".to_string()),
        }
    }
}

/// Forward-looking forecast state.
#[derive(Clone, Debug, Default)]
pub struct ForecastState {
    pub drift_forecast: f32,
    pub load_forecast: f32,
    pub efficiency_trend: f32,
    pub risk_trend: f32,
}

/// Complete Brain δ: Perception and Forecasting Engine.
#[derive(Clone, Debug)]
pub struct BrainDelta {
    pub agent_id: AgentId,
    pub level: AgentLevel,

    pub perception_snapshot: PerceptionSnapshot,
    pub representation_state: RepresentationState,
    pub forecast_state: ForecastState,

    pub cycle_count: u64,
    pub history_window: usize,
    pub metrics_history: VecDeque<LocalMetrics>,
    pub embedding_history: VecDeque<Vec<f32>>,
}

impl BrainDelta {
    pub fn new(agent_id: AgentId, level: AgentLevel) -> Self {
        Self {
            agent_id,
            level,
            perception_snapshot: PerceptionSnapshot::default(),
            representation_state: RepresentationState::default(),
            forecast_state: ForecastState::default(),
            cycle_count: 0,
            history_window: 20,
            metrics_history: VecDeque::with_capacity(20),
            embedding_history: VecDeque::with_capacity(20),
        }
    }

    /// Ingest new perception data.
    pub fn ingest_signals(&mut self, snapshot: PerceptionSnapshot) {
        self.perception_snapshot = snapshot;
        self.cycle_count += 1;

        self.metrics_history.push_back(self.perception_snapshot.local_metrics.clone());
        if self.metrics_history.len() > self.history_window {
            self.metrics_history.pop_front();
        }
    }

    /// Update embeddings from current perception.
    pub fn update_representation(&mut self) {
        // Agent embedding (16-dim)
        self.representation_state.agent_embedding = vec![
            self.perception_snapshot.local_metrics.efficiency,
            (self.perception_snapshot.local_metrics.queue_depth as f32) / 100.0,
            self.perception_snapshot.local_metrics.mutation_rate,
            self.perception_snapshot.local_metrics.connection_quality,
            self.perception_snapshot.hormone_levels.stress,
            self.perception_snapshot.hormone_levels.growth,
            self.perception_snapshot.hormone_levels.repair,
            self.perception_snapshot.hormone_levels.coordination,
            self.forecast_state.efficiency_trend,
            self.forecast_state.risk_trend,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        // Cluster embedding (8-dim)
        self.representation_state.cluster_embedding = vec![
            self.perception_snapshot.cluster_metrics.avg_efficiency,
            if self.perception_snapshot.cluster_metrics.drift_distribution.is_empty() {
                0.0
            } else {
                self.perception_snapshot.cluster_metrics.drift_distribution.iter().sum::<f32>()
                    / self.perception_snapshot.cluster_metrics.drift_distribution.len() as f32
            },
            self.perception_snapshot.cluster_metrics.consensus_health,
            0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        // Swarm embedding (8-dim)
        self.representation_state.swarm_embedding = vec![
            self.perception_snapshot.swarm_metrics.global_efficiency,
            self.perception_snapshot.swarm_metrics.global_drift,
            self.perception_snapshot.swarm_metrics.global_load,
            self.perception_snapshot.swarm_metrics.global_risk,
            0.0, 0.0, 0.0, 0.0,
        ];

        // Regime labeling
        let stress = self.perception_snapshot.hormone_levels.stress;
        let growth = self.perception_snapshot.hormone_levels.growth;

        self.representation_state.regime_label = Some(match (stress > 0.7, growth > 0.6, stress > 0.4 && growth < 0.3) {
            (true, _, _) => "stressed".to_string(),
            (_, true, _) => "improving".to_string(),
            (_, _, true) => "exploring".to_string(),
            _ => "normal".to_string(),
        });

        self.embedding_history.push_back(self.representation_state.agent_embedding.clone());
        if self.embedding_history.len() > self.history_window {
            self.embedding_history.pop_front();
        }
    }

    /// Compute forward forecasts from current state + history.
    pub fn update_forecast(&mut self) {
        if self.metrics_history.len() < 2 {
            self.forecast_state = ForecastState::default();
            return;
        }

        let recent = &self.metrics_history[self.metrics_history.len() - 1];
        let older = &self.metrics_history[0];
        let eff_trend = (recent.efficiency - older.efficiency) / (self.metrics_history.len() as f32).max(1.0);
        let predicted_eff = (recent.efficiency + eff_trend * 10.0).clamp(0.0, 1.0);

        let queue_trend = (recent.queue_depth as f32 - older.queue_depth as f32) / (self.metrics_history.len() as f32).max(1.0);
        let predicted_queue = (recent.queue_depth as f32 + queue_trend * 10.0).max(0.0);

        let stress = self.perception_snapshot.hormone_levels.stress;
        let drift_forecast = if stress > 0.6 {
            (stress - 0.6) * 0.5
        } else {
            -0.1
        };

        let risk_trend = if stress > 0.5 {
            0.1
        } else if predicted_eff > 0.9 {
            -0.05
        } else {
            0.0
        };

        self.forecast_state = ForecastState {
            drift_forecast: drift_forecast.clamp(0.0, 1.0),
            load_forecast: (predicted_queue / 100.0).clamp(0.0, 1.0),
            efficiency_trend: eff_trend,
            risk_trend,
        };
    }

    /// Export forecast snapshot for Brain γ consumption.
    pub fn export_forecast_snapshot(&self) -> (f32, f32, f32, f32) {
        (
            self.forecast_state.drift_forecast,
            self.forecast_state.load_forecast,
            self.forecast_state.efficiency_trend,
            self.forecast_state.risk_trend,
        )
    }

    /// Get regime label.
    pub fn get_regime_label(&self) -> Option<String> {
        self.representation_state.regime_label.clone()
    }

    /// Get hormone levels.
    pub fn get_hormone_levels(&self) -> (f32, f32, f32, f32) {
        (
            self.perception_snapshot.hormone_levels.stress,
            self.perception_snapshot.hormone_levels.growth,
            self.perception_snapshot.hormone_levels.repair,
            self.perception_snapshot.hormone_levels.coordination,
        )
    }

    /// Compute hormone levels from current perception.
    pub fn update_hormones(&mut self) {
        let efficiency = self.perception_snapshot.local_metrics.efficiency;
        let drift_avg = if self.perception_snapshot.cluster_metrics.drift_distribution.is_empty() {
            0.0
        } else {
            self.perception_snapshot.cluster_metrics.drift_distribution.iter().sum::<f32>()
                / self.perception_snapshot.cluster_metrics.drift_distribution.len() as f32
        };
        let load_avg = if self.perception_snapshot.cluster_metrics.load_distribution.is_empty() {
            0.0
        } else {
            self.perception_snapshot.cluster_metrics.load_distribution.iter().sum::<f32>()
                / self.perception_snapshot.cluster_metrics.load_distribution.len() as f32
        };

        // Stress increases with high drift/load
        let stress = (drift_avg * 0.6 + load_avg * 0.4).clamp(0.0, 1.0);

        // Growth increases with high efficiency
        let growth = efficiency.clamp(0.0, 1.0);

        // Repair is high if stress is high
        let repair = if stress > 0.5 { stress * 0.8 } else { 0.1 };

        // Coordination from consensus
        let coordination = self.perception_snapshot.cluster_metrics.consensus_health.clamp(0.0, 1.0);

        self.perception_snapshot.hormone_levels = HormoneLevels {
            stress,
            growth,
            repair,
            coordination,
        };
    }

    /// Get report of perception state.
    pub fn report(&self) -> DeltaReport {
        DeltaReport {
            agent_id: self.agent_id,
            cycle: self.cycle_count,
            regime_label: self.representation_state.regime_label.clone(),
            drift_forecast: self.forecast_state.drift_forecast,
            load_forecast: self.forecast_state.load_forecast,
            efficiency_trend: self.forecast_state.efficiency_trend,
            risk_trend: self.forecast_state.risk_trend,
            stress: self.perception_snapshot.hormone_levels.stress,
            growth: self.perception_snapshot.hormone_levels.growth,
        }
    }
}

/// Report from Brain δ.
#[derive(Clone, Debug)]
pub struct DeltaReport {
    pub agent_id: AgentId,
    pub cycle: u64,
    pub regime_label: Option<String>,
    pub drift_forecast: f32,
    pub load_forecast: f32,
    pub efficiency_trend: f32,
    pub risk_trend: f32,
    pub stress: f32,
    pub growth: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::mutation::domain_coordination::AgentLevel;

    #[test]
    fn brain_delta_creation() {
        let delta = BrainDelta::new(AgentId::new(1), AgentLevel::Nano);
        assert_eq!(delta.agent_id, AgentId::new(1));
        assert_eq!(delta.cycle_count, 0);
        assert_eq!(delta.history_window, 20);
    }

    #[test]
    fn brain_delta_initial_embeddings() {
        let delta = BrainDelta::new(AgentId::new(1), AgentLevel::Micro);
        assert_eq!(delta.representation_state.agent_embedding.len(), 16);
        assert_eq!(delta.representation_state.cluster_embedding.len(), 8);
        assert_eq!(delta.representation_state.swarm_embedding.len(), 8);
    }

    #[test]
    fn brain_delta_ingest_signals() {
        let mut delta = BrainDelta::new(AgentId::new(2), AgentLevel::Sub);
        let snapshot = PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: 0.85,
                queue_depth: 5,
                mutation_rate: 0.3,
                connection_quality: 0.95,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        assert_eq!(delta.cycle_count, 1);
        assert_eq!(delta.metrics_history.len(), 1);
    }

    #[test]
    fn brain_delta_update_representation() {
        let mut delta = BrainDelta::new(AgentId::new(3), AgentLevel::Super);
        let snapshot = PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: 0.8,
                queue_depth: 3,
                mutation_rate: 0.2,
                connection_quality: 0.9,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_representation();

        assert_eq!(delta.representation_state.agent_embedding[0], 0.8);
        assert!(delta.representation_state.regime_label.is_some());
    }

    #[test]
    fn brain_delta_regime_labeling_normal() {
        let mut delta = BrainDelta::new(AgentId::new(4), AgentLevel::Nano);
        let snapshot = PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.2,
                growth: 0.3,
                repair: 0.1,
                coordination: 0.8,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_representation();

        assert_eq!(delta.representation_state.regime_label, Some("normal".to_string()));
    }

    #[test]
    fn brain_delta_regime_labeling_stressed() {
        let mut delta = BrainDelta::new(AgentId::new(5), AgentLevel::Micro);
        let snapshot = PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.8,
                growth: 0.2,
                repair: 0.5,
                coordination: 0.6,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_representation();

        assert_eq!(delta.representation_state.regime_label, Some("stressed".to_string()));
    }

    #[test]
    fn brain_delta_regime_labeling_improving() {
        let mut delta = BrainDelta::new(AgentId::new(6), AgentLevel::Sub);
        let snapshot = PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.2,
                growth: 0.7,
                repair: 0.1,
                coordination: 0.8,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_representation();

        assert_eq!(delta.representation_state.regime_label, Some("improving".to_string()));
    }

    #[test]
    fn brain_delta_update_forecast_short_history() {
        let mut delta = BrainDelta::new(AgentId::new(7), AgentLevel::Super);
        let snapshot = PerceptionSnapshot::default();

        delta.ingest_signals(snapshot);
        delta.update_forecast();

        assert_eq!(delta.forecast_state.drift_forecast, 0.0);
    }

    #[test]
    fn brain_delta_update_forecast_with_history() {
        let mut delta = BrainDelta::new(AgentId::new(8), AgentLevel::Nano);

        // Ingest two different efficiency snapshots
        delta.ingest_signals(PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: 0.7,
                queue_depth: 5,
                mutation_rate: 0.1,
                connection_quality: 0.9,
            },
            ..Default::default()
        });

        delta.ingest_signals(PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: 0.8,
                queue_depth: 4,
                mutation_rate: 0.15,
                connection_quality: 0.92,
            },
            ..Default::default()
        });

        delta.update_forecast();
        assert!(delta.forecast_state.efficiency_trend > 0.0);
    }

    #[test]
    fn brain_delta_update_forecast_with_stress() {
        let mut delta = BrainDelta::new(AgentId::new(9), AgentLevel::Micro);

        delta.ingest_signals(PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.7,
                growth: 0.2,
                repair: 0.3,
                coordination: 0.6,
            },
            ..Default::default()
        });

        delta.ingest_signals(PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.75,
                growth: 0.15,
                repair: 0.4,
                coordination: 0.5,
            },
            ..Default::default()
        });

        delta.update_forecast();
        assert!(delta.forecast_state.drift_forecast > 0.0);
    }

    #[test]
    fn brain_delta_metrics_history_bounded() {
        let mut delta = BrainDelta::new(AgentId::new(10), AgentLevel::Sub);

        for i in 0..30 {
            let snapshot = PerceptionSnapshot {
                local_metrics: LocalMetrics {
                    efficiency: 0.7 + (i as f32 * 0.01),
                    queue_depth: i as usize,
                    mutation_rate: 0.1,
                    connection_quality: 0.9,
                },
                ..Default::default()
            };
            delta.ingest_signals(snapshot);
        }

        assert_eq!(delta.metrics_history.len(), 20);
    }

    #[test]
    fn brain_delta_embedding_history_bounded() {
        let mut delta = BrainDelta::new(AgentId::new(11), AgentLevel::Super);

        for i in 0..30 {
            let snapshot = PerceptionSnapshot {
                local_metrics: LocalMetrics {
                    efficiency: 0.7 + (i as f32 * 0.01),
                    queue_depth: i as usize,
                    mutation_rate: 0.1,
                    connection_quality: 0.9,
                },
                ..Default::default()
            };
            delta.ingest_signals(snapshot);
            delta.update_representation();
        }

        assert_eq!(delta.embedding_history.len(), 20);
    }

    #[test]
    fn brain_delta_export_forecast() {
        let mut delta = BrainDelta::new(AgentId::new(12), AgentLevel::Nano);
        delta.ingest_signals(PerceptionSnapshot::default());
        delta.update_forecast();

        let (drift, load, _eff_trend, _risk) = delta.export_forecast_snapshot();
        assert!((0.0..=1.0).contains(&drift));
        assert!((0.0..=1.0).contains(&load));
    }

    #[test]
    fn brain_delta_get_regime_label() {
        let mut delta = BrainDelta::new(AgentId::new(13), AgentLevel::Micro);
        let snapshot = PerceptionSnapshot {
            hormone_levels: HormoneLevels {
                stress: 0.3,
                growth: 0.5,
                repair: 0.1,
                coordination: 0.8,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_representation();
        let label = delta.get_regime_label();
        assert!(label.is_some());
    }

    #[test]
    fn brain_delta_update_hormones() {
        let mut delta = BrainDelta::new(AgentId::new(14), AgentLevel::Sub);
        let snapshot = PerceptionSnapshot {
            local_metrics: LocalMetrics {
                efficiency: 0.85,
                queue_depth: 2,
                mutation_rate: 0.1,
                connection_quality: 0.95,
            },
            cluster_metrics: ClusterMetrics {
                avg_efficiency: 0.8,
                drift_distribution: vec![0.1, 0.15],
                load_distribution: vec![0.2, 0.25],
                consensus_health: 0.85,
            },
            ..Default::default()
        };

        delta.ingest_signals(snapshot);
        delta.update_hormones();

        let (stress, growth, repair, coord) = delta.get_hormone_levels();
        assert!((0.0..=1.0).contains(&stress));
        assert!((0.0..=1.0).contains(&growth));
        assert!((0.0..=1.0).contains(&repair));
        assert!((0.0..=1.0).contains(&coord));
    }

    #[test]
    fn brain_delta_report() {
        let mut delta = BrainDelta::new(AgentId::new(15), AgentLevel::Super);
        let snapshot = PerceptionSnapshot::default();

        delta.ingest_signals(snapshot);
        delta.update_forecast();
        let report = delta.report();

        assert_eq!(report.agent_id, AgentId::new(15));
        assert_eq!(report.cycle, 1);
    }

    #[test]
    fn brain_delta_cycle_advancement() {
        let mut delta = BrainDelta::new(AgentId::new(16), AgentLevel::Nano);

        for _ in 0..5 {
            delta.ingest_signals(PerceptionSnapshot::default());
        }

        assert_eq!(delta.cycle_count, 5);
    }
}
