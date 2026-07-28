//! Self-healing system: agents autonomously detect, diagnose, and fix errors
//!
//! Phase 8: Meta-programming for self-modification
//! - Error detection: Agents monitor their own health
//! - Root cause analysis: Trace back to bottleneck source
//! - Remediation: Auto-generate fixes (code rewrites, parameter tuning, etc)
//! - Validation: Test fix before applying
//! - Learning: Record what worked for future similar cases

use crate::ntg::error::NtgError;
use std::collections::HashMap;
use std::time::Instant;

/// Error severity levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    Info,       // Performance degradation < 5%
    Warning,    // Degradation 5-20%
    Error,      // Degradation 20-50%
    Critical,   // Degradation > 50%
    Fatal,      // System crash imminent
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
            Self::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Root cause classification
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RootCause {
    MemoryLeak,
    CacheInvalidation,
    DeadlockRisk,
    ThreadStarvation,
    DataRaceCondition,
    StackOverflow,
    OutOfMemory,
    FileHandleLeak,
    GPUMemoryLeak,
    AlgorithmicComplexity,
    ConfigurationMismatch,
    HardwareFailure,
}

impl std::fmt::Display for RootCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemoryLeak => write!(f, "memory_leak"),
            Self::CacheInvalidation => write!(f, "cache_invalidation"),
            Self::DeadlockRisk => write!(f, "deadlock_risk"),
            Self::ThreadStarvation => write!(f, "thread_starvation"),
            Self::DataRaceCondition => write!(f, "data_race"),
            Self::StackOverflow => write!(f, "stack_overflow"),
            Self::OutOfMemory => write!(f, "out_of_memory"),
            Self::FileHandleLeak => write!(f, "file_handle_leak"),
            Self::GPUMemoryLeak => write!(f, "gpu_memory_leak"),
            Self::AlgorithmicComplexity => write!(f, "algorithmic_complexity"),
            Self::ConfigurationMismatch => write!(f, "configuration_mismatch"),
            Self::HardwareFailure => write!(f, "hardware_failure"),
        }
    }
}

/// Detected error with diagnosis
#[derive(Clone, Debug)]
pub struct DiagnosedError {
    pub id: String,
    pub severity: ErrorSeverity,
    pub root_cause: Option<RootCause>,
    pub description: String,
    pub detection_time: Instant,
    pub metric_value: f32,
    pub threshold: f32,
    pub confidence: f32,  // 0.0-1.0, how confident in diagnosis
}

/// Proposed fix action
#[derive(Clone, Debug)]
pub struct RemediationAction {
    pub id: String,
    pub error_id: String,
    pub action_type: RemediationType,
    pub description: String,
    pub expected_recovery: f32,  // How much performance improvement expected
    pub rollback_plan: String,   // How to undo if something goes wrong
    pub estimated_duration_ms: u64,
}

/// Type of remediation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemediationType {
    // Memory fixes
    FreeLeakedMemory,
    CompactHeap,
    ReduceBufferSize,

    // Concurrency fixes
    RemoveDeadlock,
    RebalanceThreadPool,
    BackoffRetryLogic,

    // Cache fixes
    InvalidateCache,
    PrefetchData,
    ReorderAccess,

    // Algorithm fixes
    ReduceComplexity,
    BatchOperations,
    EnableParallelization,

    // Configuration fixes
    AdjustThreadCount,
    TuneMemoryLimit,
    EnableGPUAcceleration,

    // System fixes
    RestartComponent,
    FailoverToBackup,
    GracefulShutdown,
}

impl std::fmt::Display for RemediationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FreeLeakedMemory => write!(f, "free_leaked_memory"),
            Self::CompactHeap => write!(f, "compact_heap"),
            Self::ReduceBufferSize => write!(f, "reduce_buffer_size"),
            Self::RemoveDeadlock => write!(f, "remove_deadlock"),
            Self::RebalanceThreadPool => write!(f, "rebalance_thread_pool"),
            Self::BackoffRetryLogic => write!(f, "backoff_retry_logic"),
            Self::InvalidateCache => write!(f, "invalidate_cache"),
            Self::PrefetchData => write!(f, "prefetch_data"),
            Self::ReorderAccess => write!(f, "reorder_access"),
            Self::ReduceComplexity => write!(f, "reduce_complexity"),
            Self::BatchOperations => write!(f, "batch_operations"),
            Self::EnableParallelization => write!(f, "enable_parallelization"),
            Self::AdjustThreadCount => write!(f, "adjust_thread_count"),
            Self::TuneMemoryLimit => write!(f, "tune_memory_limit"),
            Self::EnableGPUAcceleration => write!(f, "enable_gpu_acceleration"),
            Self::RestartComponent => write!(f, "restart_component"),
            Self::FailoverToBackup => write!(f, "failover_to_backup"),
            Self::GracefulShutdown => write!(f, "graceful_shutdown"),
        }
    }
}

/// Self-healing engine: autonomous error detection & remediation
pub struct SelfHealer {
    pub config: SelfHealerConfig,
    pub detected_errors: Vec<DiagnosedError>,
    pub remediation_history: Vec<RemediationHistory>,
    pub error_patterns: HashMap<RootCause, u32>,  // Track error frequency
    pub successful_remediations: u32,
    pub failed_remediations: u32,
}

/// Configuration for self-healing
#[derive(Clone, Debug)]
pub struct SelfHealerConfig {
    /// Enable automatic remediation (vs. alerting only)
    pub autonomous_remediation: bool,
    /// Maximum severity to auto-fix (anything higher → manual review)
    pub auto_fix_max_severity: ErrorSeverity,
    /// Minimum confidence threshold for diagnosis (0.0-1.0)
    pub min_diagnosis_confidence: f32,
    /// Recovery time budget (milliseconds)
    pub max_recovery_time_ms: u64,
    /// Historical window for pattern detection
    pub pattern_history_cycles: usize,
}

impl Default for SelfHealerConfig {
    fn default() -> Self {
        Self {
            autonomous_remediation: true,
            auto_fix_max_severity: ErrorSeverity::Error,  // Auto-fix up to Error, escalate Critical/Fatal
            min_diagnosis_confidence: 0.7,
            max_recovery_time_ms: 100,
            pattern_history_cycles: 100,
        }
    }
}

/// Record of a remediation attempt
#[derive(Clone, Debug)]
pub struct RemediationHistory {
    pub timestamp: Instant,
    pub error: DiagnosedError,
    pub action: RemediationAction,
    pub success: bool,
    pub performance_before: f32,
    pub performance_after: f32,
    pub recovery_time_ms: u64,
}

impl SelfHealer {
    pub fn new(config: SelfHealerConfig) -> Self {
        Self {
            config,
            detected_errors: Vec::new(),
            remediation_history: Vec::new(),
            error_patterns: HashMap::new(),
            successful_remediations: 0,
            failed_remediations: 0,
        }
    }

    /// Detect error by monitoring system metrics
    pub fn detect_error(
        &mut self,
        metric_name: &str,
        current_value: f32,
        baseline_value: f32,
        threshold_degradation: f32,  // e.g., 0.2 = 20% degradation
    ) -> Result<Option<DiagnosedError>, NtgError> {
        let degradation = match metric_name {
            "memory_usage" | "latency" | "lock_contention" | "thread_queue_depth" | "gpu_memory" => {
                (current_value - baseline_value).abs() / baseline_value
            }
            _ => {
                (baseline_value - current_value) / baseline_value
            }
        };

        if degradation < threshold_degradation {
            return Ok(None);  // No error
        }

        // Classify severity
        let severity = if degradation > 0.5 {
            ErrorSeverity::Critical
        } else if degradation > 0.3 {
            ErrorSeverity::Error
        } else if degradation > 0.15 {
            ErrorSeverity::Warning
        } else {
            ErrorSeverity::Info
        };

        // Diagnose root cause (simplified; real system would use ML)
        let root_cause = self.diagnose_root_cause(metric_name, current_value, baseline_value);

        let error = DiagnosedError {
            id: format!("err_{}", self.detected_errors.len()),
            severity,
            root_cause: root_cause.0,
            description: format!(
                "{}: {} degradation ({:.2} → {:.2})",
                metric_name, metric_name, baseline_value, current_value
            ),
            detection_time: Instant::now(),
            metric_value: current_value,
            threshold: baseline_value * (1.0 - threshold_degradation),
            confidence: root_cause.1,
        };

        self.detected_errors.push(error.clone());

        if let Some(ref cause) = error.root_cause {
            *self.error_patterns.entry(cause.clone()).or_insert(0) += 1;
        }

        Ok(Some(error))
    }

    /// Diagnose root cause from metrics (heuristic-based)
    fn diagnose_root_cause(&self, metric_name: &str, current: f32, baseline: f32) -> (Option<RootCause>, f32) {
        // Simple heuristics; real system would use ML classifier
        match metric_name {
            "memory_usage" if current > baseline * 1.5 => {
                (Some(RootCause::MemoryLeak), 0.8)
            }
            "cache_hit_rate" if current < baseline * 0.7 => {
                (Some(RootCause::CacheInvalidation), 0.7)
            }
            "lock_contention" if current > baseline * 2.0 => {
                (Some(RootCause::DeadlockRisk), 0.6)
            }
            "thread_queue_depth" if current > baseline * 3.0 => {
                (Some(RootCause::ThreadStarvation), 0.75)
            }
            "gpu_memory" if current > baseline * 1.3 => {
                (Some(RootCause::GPUMemoryLeak), 0.75)
            }
            "latency" if current > baseline * 5.0 => {
                (Some(RootCause::AlgorithmicComplexity), 0.6)
            }
            _ => (None, 0.3),
        }
    }

    /// Generate remediation actions for diagnosed error
    pub fn generate_remediations(&self, error: &DiagnosedError) -> Result<Vec<RemediationAction>, NtgError> {
        let mut actions = Vec::new();

        if let Some(root_cause) = &error.root_cause {
            match root_cause {
                RootCause::MemoryLeak => {
                    actions.push(RemediationAction {
                        id: format!("rem_{}_1", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::FreeLeakedMemory,
                        description: "Scan and free unreferenced memory".to_string(),
                        expected_recovery: 0.8,
                        rollback_plan: "Restore memory snapshot from last known good state".to_string(),
                        estimated_duration_ms: 50,
                    });
                    actions.push(RemediationAction {
                        id: format!("rem_{}_2", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::CompactHeap,
                        description: "Compact heap to reduce fragmentation".to_string(),
                        expected_recovery: 0.4,
                        rollback_plan: "No rollback needed (garbage collection)".to_string(),
                        estimated_duration_ms: 100,
                    });
                }
                RootCause::CacheInvalidation => {
                    actions.push(RemediationAction {
                        id: format!("rem_{}_1", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::InvalidateCache,
                        description: "Clear and rebuild cache".to_string(),
                        expected_recovery: 0.9,
                        rollback_plan: "Restore cache from backup".to_string(),
                        estimated_duration_ms: 30,
                    });
                    actions.push(RemediationAction {
                        id: format!("rem_{}_2", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::PrefetchData,
                        description: "Prefetch frequently accessed data".to_string(),
                        expected_recovery: 0.5,
                        rollback_plan: "Disable prefetching".to_string(),
                        estimated_duration_ms: 20,
                    });
                }
                RootCause::ThreadStarvation => {
                    actions.push(RemediationAction {
                        id: format!("rem_{}_1", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::RebalanceThreadPool,
                        description: "Increase thread pool size from current to optimal".to_string(),
                        expected_recovery: 0.85,
                        rollback_plan: "Reduce thread pool to original size".to_string(),
                        estimated_duration_ms: 10,
                    });
                }
                RootCause::AlgorithmicComplexity => {
                    actions.push(RemediationAction {
                        id: format!("rem_{}_1", error.id),
                        error_id: error.id.clone(),
                        action_type: RemediationType::EnableParallelization,
                        description: "Enable GPU acceleration or multi-core parallelization".to_string(),
                        expected_recovery: 0.75,
                        rollback_plan: "Revert to single-threaded algorithm".to_string(),
                        estimated_duration_ms: 50,
                    });
                }
                _ => {}
            }
        }

        Ok(actions)
    }

    /// Execute remediation and track outcome
    pub fn execute_remediation(
        &mut self,
        error: DiagnosedError,
        action: RemediationAction,
        performance_before: f32,
    ) -> Result<RemediationHistory, NtgError> {
        let start = Instant::now();

        // Simulate remediation execution
        // In real system, would execute actual code changes
        let success = action.estimated_duration_ms < self.config.max_recovery_time_ms
            && error.severity <= self.config.auto_fix_max_severity;

        let recovery_time = start.elapsed().as_millis() as u64;
        let performance_after = if success {
            performance_before * (1.0 + action.expected_recovery)
        } else {
            performance_before
        };

        let history = RemediationHistory {
            timestamp: start,
            error: error.clone(),
            action: action.clone(),
            success,
            performance_before,
            performance_after,
            recovery_time_ms: recovery_time,
        };

        if success {
            self.successful_remediations += 1;
        } else {
            self.failed_remediations += 1;
        }

        self.remediation_history.push(history.clone());

        Ok(history)
    }

    /// Get health summary
    pub fn health_summary(&self) -> SelfHealerSummary {
        let total_remediations = self.successful_remediations + self.failed_remediations;
        let success_rate = if total_remediations > 0 {
            self.successful_remediations as f32 / total_remediations as f32
        } else {
            1.0
        };

        let avg_recovery_time = if !self.remediation_history.is_empty() {
            self.remediation_history.iter().map(|r| r.recovery_time_ms).sum::<u64>()
                / self.remediation_history.len() as u64
        } else {
            0
        };

        let most_common_error = self.error_patterns.iter()
            .max_by_key(|&(_, count)| count)
            .map(|(cause, count)| (cause.clone(), *count));

        SelfHealerSummary {
            total_errors_detected: self.detected_errors.len(),
            total_remediations: total_remediations,
            successful_remediations: self.successful_remediations,
            failed_remediations: self.failed_remediations,
            success_rate,
            avg_recovery_time_ms: avg_recovery_time,
            most_common_error,
        }
    }
}

/// Summary of self-healing activity
#[derive(Clone, Debug)]
pub struct SelfHealerSummary {
    pub total_errors_detected: usize,
    pub total_remediations: u32,
    pub successful_remediations: u32,
    pub failed_remediations: u32,
    pub success_rate: f32,
    pub avg_recovery_time_ms: u64,
    pub most_common_error: Option<(RootCause, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_memory_leak() -> Result<(), NtgError> {
        let mut healer = SelfHealer::new(SelfHealerConfig::default());

        let error = healer.detect_error(
            "memory_usage",
            1600.0,  // Current: 1.6GB (> 1.5x baseline)
            1000.0,  // Baseline: 1.0GB
            0.2,     // Alert if > 20% increase
        )?;

        assert!(error.is_some());
        assert_eq!(error.unwrap().root_cause, Some(RootCause::MemoryLeak));
        Ok(())
    }

    #[test]
    fn generate_remediations() -> Result<(), NtgError> {
        let healer = SelfHealer::new(SelfHealerConfig::default());

        let error = DiagnosedError {
            id: "test".to_string(),
            severity: ErrorSeverity::Error,
            root_cause: Some(RootCause::MemoryLeak),
            description: "Memory leak detected".to_string(),
            detection_time: Instant::now(),
            metric_value: 1500.0,
            threshold: 1000.0,
            confidence: 0.8,
        };

        let actions = healer.generate_remediations(&error)?;
        assert!(!actions.is_empty());
        Ok(())
    }
}
