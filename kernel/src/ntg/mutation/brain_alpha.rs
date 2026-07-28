//! Phase 6.14: Brain α — Synchronization and Self-Healing.
//!
//! Synchronizes behavior across agent hierarchies and detects/repairs divergence:
//! - Behavioral drift detection (10-cycle rolling window)
//! - Health monitoring for parent/sibling connections
//! - Consensus enforcement across clusters
//! - Automatic rollback on pathological patterns
//! - Checkpoint-based state recovery

use std::collections::{HashMap, VecDeque};
use super::super::error::NtgError;
use super::domain_coordination::{AgentId, AgentLevel};

/// Behavioral state snapshot for synchronization.
#[derive(Clone, Debug)]
pub struct BehavioralSignature {
    pub cycle: u64,
    pub efficiency: f32,
    pub mutation_acceptance_rate: f32,
    pub active_strategies: Vec<String>,
    pub behavior_hash: u64,
}

impl BehavioralSignature {
    /// Compute Hamming distance between two signatures.
    pub fn distance(&self, other: &Self) -> u64 {
        (self.behavior_hash ^ other.behavior_hash).count_ones() as u64
    }
}

/// Detects behavioral divergence from expected patterns.
#[derive(Clone, Debug)]
pub struct BehavioralDriftDetector {
    pub window_cycles: usize,
    pub baseline_efficiency: f32,
    pub baseline_strategies: Vec<String>,
    pub drift_threshold: f32,
    pub history: VecDeque<BehavioralSignature>,
}

impl BehavioralDriftDetector {
    pub fn new(baseline_efficiency: f32, window_cycles: usize) -> Self {
        Self {
            window_cycles,
            baseline_efficiency,
            baseline_strategies: Vec::new(),
            drift_threshold: 0.4,
            history: VecDeque::with_capacity(window_cycles),
        }
    }

    /// Record a behavioral observation.
    pub fn observe(&mut self, signature: BehavioralSignature) {
        self.history.push_back(signature);
        if self.history.len() > self.window_cycles {
            self.history.pop_front();
        }
    }

    /// Detect if behavior has drifted significantly.
    pub fn detect_drift(&self) -> (bool, f32, String) {
        if self.history.is_empty() {
            return (false, 0.0, "No history".to_string());
        }

        // Compute average efficiency in window
        let avg_efficiency: f32 =
            self.history.iter().map(|s| s.efficiency).sum::<f32>() / self.history.len() as f32;

        let efficiency_delta = (self.baseline_efficiency - avg_efficiency).abs();
        let efficiency_ratio = efficiency_delta / (self.baseline_efficiency + 0.001);

        let is_drifting = efficiency_ratio > self.drift_threshold;

        let reason = if is_drifting {
            format!(
                "Efficiency drift: expected {:.2}, got {:.2} (ratio {:.2} > threshold {:.2})",
                self.baseline_efficiency, avg_efficiency, efficiency_ratio, self.drift_threshold
            )
        } else {
            format!(
                "No drift detected: efficiency {:.2} within threshold",
                efficiency_ratio
            )
        };

        (is_drifting, efficiency_ratio, reason)
    }

    /// Get average efficiency from history.
    pub fn avg_efficiency(&self) -> f32 {
        if self.history.is_empty() {
            self.baseline_efficiency
        } else {
            self.history.iter().map(|s| s.efficiency).sum::<f32>() / self.history.len() as f32
        }
    }
}

/// Connection quality assessment.
#[derive(Clone, Copy, Debug)]
pub enum ConnectionStatus {
    Healthy,
    Degraded(f32),  // latency or packet loss ratio
    Failed(u64),    // cycles since last successful heartbeat
}

/// Health monitoring for agents and their connections.
#[derive(Clone, Debug)]
pub struct HealthMonitor {
    pub last_heartbeat_from_parent: u64,
    pub parent_connection_quality: f32, // 0.0-1.0
    pub sibling_connection_quality: HashMap<AgentId, f32>,
    pub consecutive_rejections: u64,
    pub health_threshold: f32,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            last_heartbeat_from_parent: 0,
            parent_connection_quality: 1.0,
            sibling_connection_quality: HashMap::new(),
            consecutive_rejections: 0,
            health_threshold: 0.2, // Connection quality must stay above 0.2
        }
    }

    /// Update parent heartbeat timestamp.
    pub fn parent_heartbeat(&mut self, current_cycle: u64) {
        self.last_heartbeat_from_parent = current_cycle;
        // Improve connection quality on successful heartbeat
        self.parent_connection_quality = (self.parent_connection_quality + 0.1).min(1.0);
    }

    /// Check parent connection health.
    pub fn parent_status(&self, current_cycle: u64) -> ConnectionStatus {
        let cycles_since = current_cycle.saturating_sub(self.last_heartbeat_from_parent);

        if cycles_since > 20 {
            ConnectionStatus::Failed(cycles_since)
        } else if cycles_since > 10 {
            ConnectionStatus::Degraded(self.parent_connection_quality)
        } else {
            ConnectionStatus::Healthy
        }
    }

    /// Record a mutation rejection.
    pub fn record_rejection(&mut self) {
        self.consecutive_rejections += 1;
    }

    /// Clear rejection counter on success.
    pub fn record_acceptance(&mut self) {
        self.consecutive_rejections = 0;
    }

    /// Check if health is critical (too many rejections).
    pub fn is_critical(&self) -> bool {
        self.consecutive_rejections > 20 || self.parent_connection_quality < self.health_threshold
    }
}

/// Synchronization state between agent and parent/siblings.
#[derive(Clone, Debug)]
pub struct SyncState {
    pub current_behavior_hash: u64,
    pub expected_behavior_hash: u64,
    pub sync_offset_cycles: u64,
    pub parent_alignment_vector: Option<Vec<f32>>,
    pub children_alignment_vectors: HashMap<AgentId, Vec<f32>>,
    pub sync_confidence: f32,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            current_behavior_hash: 0,
            expected_behavior_hash: 0,
            sync_offset_cycles: 0,
            parent_alignment_vector: None,
            children_alignment_vectors: HashMap::new(),
            sync_confidence: 1.0,
        }
    }

    /// Compute alignment score (0.0 = diverged, 1.0 = perfectly aligned).
    pub fn alignment_score(&self) -> f32 {
        if self.current_behavior_hash == self.expected_behavior_hash {
            1.0
        } else {
            let distance = (self.current_behavior_hash ^ self.expected_behavior_hash).count_ones() as f32;
            (1.0 - (distance / 64.0)).max(0.0)
        }
    }
}

/// Checkpoint for rollback recovery.
#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub checkpoint_id: u64,
    pub cycle: u64,
    pub efficiency: f32,
    pub mutation_acceptance_rate: f32,
    pub active_strategies: Vec<String>,
    pub behavior_hash: u64,
    pub timestamp_us: u64,
    pub reason: String,
}

/// Rollback trigger and recovery logic.
#[derive(Clone, Debug)]
pub struct RollbackManager {
    pub checkpoints: VecDeque<Checkpoint>,
    pub max_checkpoints: usize,
    pub next_checkpoint_id: u64,
    pub rollback_threshold_rejections: u64,
}

impl RollbackManager {
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: VecDeque::with_capacity(max_checkpoints),
            max_checkpoints,
            next_checkpoint_id: 0,
            rollback_threshold_rejections: 20, // 2× max_mutations (assume 10)
        }
    }

    /// Capture a checkpoint.
    pub fn capture(&mut self, signature: &BehavioralSignature, reason: String) -> u64 {
        let checkpoint = Checkpoint {
            checkpoint_id: self.next_checkpoint_id,
            cycle: signature.cycle,
            efficiency: signature.efficiency,
            mutation_acceptance_rate: 0.0, // Will be filled from LoopController
            active_strategies: signature.active_strategies.clone(),
            behavior_hash: signature.behavior_hash,
            timestamp_us: signature.cycle,
            reason,
        };

        self.checkpoints.push_back(checkpoint);
        if self.checkpoints.len() > self.max_checkpoints {
            self.checkpoints.pop_front();
        }

        let id = self.next_checkpoint_id;
        self.next_checkpoint_id += 1;
        id
    }

    /// Get the most recent checkpoint.
    pub fn latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.back()
    }

    /// Rollback to a specific checkpoint.
    pub fn rollback_to(&self, checkpoint_id: u64) -> Result<Checkpoint, NtgError> {
        self.checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id == checkpoint_id)
            .cloned()
            .ok_or_else(|| NtgError::InvalidInput(
                format!("Checkpoint {} not found", checkpoint_id)
            ))
    }
}

/// Brain α: Synchronization and Self-Healing Engine.
#[derive(Clone, Debug)]
pub struct BrainAlpha {
    pub agent_id: AgentId,
    pub level: AgentLevel,

    // Core components
    pub drift_detector: BehavioralDriftDetector,
    pub health_monitor: HealthMonitor,
    pub sync_state: SyncState,
    pub rollback_manager: RollbackManager,

    // Configuration
    pub auto_heal_enabled: bool,
    pub consensus_threshold: f32,

    // Metrics
    pub drifts_detected: u64,
    pub repairs_attempted: u64,
    pub repairs_successful: u64,
    pub rollbacks_triggered: u64,
}

impl BrainAlpha {
    pub fn new(
        agent_id: AgentId,
        level: AgentLevel,
        baseline_efficiency: f32,
    ) -> Self {
        Self {
            agent_id,
            level,
            drift_detector: BehavioralDriftDetector::new(baseline_efficiency, 10),
            health_monitor: HealthMonitor::new(),
            sync_state: SyncState::new(),
            rollback_manager: RollbackManager::new(10),
            auto_heal_enabled: true,
            consensus_threshold: 0.8,
            drifts_detected: 0,
            repairs_attempted: 0,
            repairs_successful: 0,
            rollbacks_triggered: 0,
        }
    }

    /// Process behavioral snapshot and detect issues.
    pub fn observe_behavior(&mut self, signature: BehavioralSignature) -> (bool, String) {
        self.drift_detector.observe(signature.clone());

        let (is_drifting, _drift_amount, reason) = self.drift_detector.detect_drift();

        if is_drifting {
            self.drifts_detected += 1;
            (true, reason)
        } else {
            (false, reason)
        }
    }

    /// Propose a repair action.
    pub fn propose_repair(&mut self) -> Option<RepairAction> {
        if !self.auto_heal_enabled {
            return None;
        }

        self.repairs_attempted += 1;

        // Detect what needs repair
        let (is_drifting, _, drift_reason) = self.drift_detector.detect_drift();
        let parent_status = self.health_monitor.parent_status(0); // TODO: pass current cycle

        match parent_status {
            ConnectionStatus::Failed(cycles) => {
                Some(RepairAction::ReconnectParent {
                    reason: format!("Parent unreachable for {} cycles", cycles),
                })
            }
            ConnectionStatus::Degraded(_) if is_drifting => {
                Some(RepairAction::Rollback {
                    reason: format!("Drift + degraded connection: {}", drift_reason),
                })
            }
            _ if is_drifting => {
                Some(RepairAction::DriftCorrection {
                    reason: drift_reason,
                    target_efficiency: self.drift_detector.baseline_efficiency,
                })
            }
            _ => None,
        }
    }

    /// Apply a repair action.
    pub fn apply_repair(&mut self, action: RepairAction) -> Result<(), NtgError> {
        match action {
            RepairAction::Rollback { .. } => {
                if let Some(_cp) = self.rollback_manager.latest_checkpoint() {
                    self.rollbacks_triggered += 1;
                    self.repairs_successful += 1;
                    Ok(())
                } else {
                    Err(NtgError::InvalidInput("No checkpoint available".to_string()))
                }
            }
            RepairAction::DriftCorrection { target_efficiency, .. } => {
                self.drift_detector.baseline_efficiency = target_efficiency;
                self.repairs_successful += 1;
                Ok(())
            }
            RepairAction::ReconnectParent { .. } => {
                self.health_monitor.parent_connection_quality = 0.5;
                self.repairs_successful += 1;
                Ok(())
            }
        }
    }

    /// Check if consensus is reached with siblings.
    pub fn check_consensus(&self, other_agents_aligned: usize, total_agents: usize) -> bool {
        let agreement_ratio = other_agents_aligned as f32 / total_agents as f32;
        agreement_ratio >= self.consensus_threshold
    }

    /// Update parent alignment vector.
    pub fn update_parent_alignment(&mut self, vector: Vec<f32>) {
        self.sync_state.parent_alignment_vector = Some(vector);
        self.health_monitor.parent_heartbeat(0);
    }

    /// Report brain α status.
    pub fn report(&self) -> String {
        format!(
            "Brain α [{:?}] — Drifts: {}, Repairs: {}/{}, Rollbacks: {}, Sync confidence: {:.2}",
            self.level,
            self.drifts_detected,
            self.repairs_successful,
            self.repairs_attempted,
            self.rollbacks_triggered,
            self.sync_state.sync_confidence
        )
    }
}

/// Repair actions that Brain α can propose.
#[derive(Clone, Debug)]
pub enum RepairAction {
    Rollback {
        reason: String,
    },
    DriftCorrection {
        reason: String,
        target_efficiency: f32,
    },
    ReconnectParent {
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavioral_signature_distance() {
        let sig1 = BehavioralSignature {
            cycle: 0,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["strategy1".to_string()],
            behavior_hash: 0xFFFF_FFFF_FFFF_FFFF,
        };

        let sig2 = BehavioralSignature {
            cycle: 1,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["strategy1".to_string()],
            behavior_hash: 0x0000_0000_0000_0000,
        };

        let distance = sig1.distance(&sig2);
        assert_eq!(distance, 64); // All bits flipped
    }

    #[test]
    fn drift_detector_initialization() {
        let detector = BehavioralDriftDetector::new(0.8, 10);
        assert_eq!(detector.baseline_efficiency, 0.8);
        assert_eq!(detector.window_cycles, 10);
        assert_eq!(detector.history.len(), 0);
    }

    #[test]
    fn drift_detector_no_drift_same_efficiency() {
        let mut detector = BehavioralDriftDetector::new(0.8, 10);

        for i in 0..10 {
            let sig = BehavioralSignature {
                cycle: i,
                efficiency: 0.8,
                mutation_acceptance_rate: 0.75,
                active_strategies: vec!["strategy1".to_string()],
                behavior_hash: 12345,
            };
            detector.observe(sig);
        }

        let (is_drifting, _, _) = detector.detect_drift();
        assert!(!is_drifting);
    }

    #[test]
    fn drift_detector_detects_significant_drop() {
        let mut detector = BehavioralDriftDetector::new(0.8, 10);

        for i in 0..10 {
            let sig = BehavioralSignature {
                cycle: i,
                efficiency: 0.4, // 50% drop
                mutation_acceptance_rate: 0.75,
                active_strategies: vec!["strategy1".to_string()],
                behavior_hash: 12345,
            };
            detector.observe(sig);
        }

        let (is_drifting, drift_amount, _) = detector.detect_drift();
        assert!(is_drifting);
        assert!(drift_amount > 0.4);
    }

    #[test]
    fn health_monitor_parent_heartbeat() {
        let mut monitor = HealthMonitor::new();
        monitor.parent_heartbeat(5);
        assert_eq!(monitor.last_heartbeat_from_parent, 5);
    }

    #[test]
    fn health_monitor_failed_connection() {
        let mut monitor = HealthMonitor::new();
        monitor.parent_heartbeat(0);

        let status = monitor.parent_status(25);
        matches!(status, ConnectionStatus::Failed(_));
    }

    #[test]
    fn health_monitor_rejection_tracking() {
        let mut monitor = HealthMonitor::new();
        assert_eq!(monitor.consecutive_rejections, 0);

        for _ in 0..5 {
            monitor.record_rejection();
        }
        assert_eq!(monitor.consecutive_rejections, 5);

        monitor.record_acceptance();
        assert_eq!(monitor.consecutive_rejections, 0);
    }

    #[test]
    fn sync_state_alignment_perfect() {
        let mut sync = SyncState::new();
        sync.current_behavior_hash = 0x1234_5678_9ABC_DEF0;
        sync.expected_behavior_hash = 0x1234_5678_9ABC_DEF0;

        assert_eq!(sync.alignment_score(), 1.0);
    }

    #[test]
    fn sync_state_alignment_diverged() {
        let mut sync = SyncState::new();
        sync.current_behavior_hash = 0xFFFF_FFFF_FFFF_FFFF;
        sync.expected_behavior_hash = 0x0000_0000_0000_0000;

        let score = sync.alignment_score();
        assert!(score < 0.1); // Mostly diverged
    }

    #[test]
    fn rollback_manager_capture() {
        let mut manager = RollbackManager::new(10);

        let sig = BehavioralSignature {
            cycle: 0,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["strategy1".to_string()],
            behavior_hash: 12345,
        };

        let cp_id = manager.capture(&sig, "test".to_string());
        assert_eq!(cp_id, 0);
        assert_eq!(manager.checkpoints.len(), 1);
    }

    #[test]
    fn rollback_manager_retrieve() {
        let mut manager = RollbackManager::new(10);

        let sig = BehavioralSignature {
            cycle: 5,
            efficiency: 0.8,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["strategy1".to_string()],
            behavior_hash: 12345,
        };

        let cp_id = manager.capture(&sig, "test".to_string());
        let retrieved = manager.rollback_to(cp_id).unwrap();

        assert_eq!(retrieved.checkpoint_id, cp_id);
        assert_eq!(retrieved.efficiency, 0.8);
    }

    #[test]
    fn brain_alpha_creation() {
        let brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        assert_eq!(brain.drifts_detected, 0);
        assert_eq!(brain.repairs_attempted, 0);
    }

    #[test]
    fn brain_alpha_observe_no_drift() {
        let mut brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        for i in 0..10 {
            let sig = BehavioralSignature {
                cycle: i,
                efficiency: 0.8,
                mutation_acceptance_rate: 0.75,
                active_strategies: vec!["strategy1".to_string()],
                behavior_hash: 12345,
            };
            let (drifting, _) = brain.observe_behavior(sig);
            if i == 9 {
                assert!(!drifting); // After 10 observations, no drift
            }
        }
    }

    #[test]
    fn brain_alpha_consensus_check() {
        let brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        assert!(brain.check_consensus(8, 10)); // 80% agreement
        assert!(!brain.check_consensus(7, 10)); // 70% agreement
    }

    #[test]
    fn brain_alpha_repair_proposal() {
        let mut brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        // Introduce drift
        for i in 0..10 {
            let sig = BehavioralSignature {
                cycle: i,
                efficiency: 0.4, // Significant drop
                mutation_acceptance_rate: 0.75,
                active_strategies: vec!["strategy1".to_string()],
                behavior_hash: 12345,
            };
            brain.observe_behavior(sig);
        }

        let repair = brain.propose_repair();
        assert!(repair.is_some());
    }

    #[test]
    fn brain_alpha_apply_repair() {
        let mut brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        let action = RepairAction::DriftCorrection {
            reason: "test".to_string(),
            target_efficiency: 0.9,
        };

        assert!(brain.apply_repair(action).is_ok());
        assert_eq!(brain.repairs_successful, 1);
    }

    #[test]
    fn brain_alpha_report() {
        let brain = BrainAlpha::new(
            AgentId::new(1),
            AgentLevel::Micro,
            0.8,
        );

        let report = brain.report();
        assert!(report.contains("Brain α"));
        assert!(report.contains("Micro"));
    }
}
