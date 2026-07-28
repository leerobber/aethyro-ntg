//! CUDA FFI bindings for quad-brain GPU acceleration
//!
//! Bridges Rust/CPU code to CUDA kernels:
//! - Brain δ (perception): agent embeddings, forecasting, regimes
//! - Brain γ (governance): policy alignment, Bayesian tracking, coherence

use std::os::raw::c_int;
use std::os::raw::c_float;

// ============================================================================
// FFI declarations: Brain δ (Perception)
// ============================================================================

extern "C" {
    /// Compute 16-dim agent embeddings via matrix multiplication on GPU
    pub fn cuda_compute_agent_embeddings(
        metrics: *const c_float,
        W: *const c_float,
        embeddings: *mut c_float,
        n_agents: c_int,
        metric_dim: c_int,
        embedding_dim: c_int,
    );

    /// Aggregate agent embeddings into cluster-level metrics
    pub fn cuda_aggregate_cluster_metrics(
        agent_embeddings: *const c_float,
        cluster_assignment: *const c_int,
        cluster_embeddings: *mut c_float,
        cluster_counts: *mut c_float,
        n_agents: c_int,
        embedding_dim: c_int,
    );

    /// Normalize cluster embeddings by agent count
    pub fn cuda_normalize_cluster_embeddings(
        cluster_embeddings: *mut c_float,
        cluster_counts: *const c_float,
        n_clusters: c_int,
        embedding_dim: c_int,
    );

    /// Forecast next-cycle metrics using linear regression on history
    pub fn cuda_forecast_agent_metrics(
        history: *const c_float,
        forecast: *mut c_float,
        n_agents: c_int,
        n_history: c_int,
        n_metrics: c_int,
    );

    /// Compute swarm-level aggregates from cluster metrics
    pub fn cuda_compute_swarm_metrics(
        cluster_metrics: *const c_float,
        swarm_metrics: *mut c_float,
        n_clusters: c_int,
        metric_dim: c_int,
    );

    /// Assign regime labels (normal/stressed/improving/exploring) based on metrics
    pub fn cuda_compute_regimes(
        agent_metrics: *const c_float,
        thresholds: *const c_float,
        regimes: *mut c_int,
        n_agents: c_int,
    );
}

// ============================================================================
// FFI declarations: Brain γ (Governance)
// ============================================================================

extern "C" {
    /// Compute policy alignment scores: agents × policies matrix
    pub fn cuda_compute_policy_alignment(
        agent_behaviors: *const c_float,
        policy_targets: *const c_float,
        policy_priorities: *const c_float,
        alignment_scores: *mut c_float,
        n_agents: c_int,
        n_policies: c_int,
        behavior_dim: c_int,
    );

    /// Update Bayesian success tracking for policies (Thompson sampling)
    pub fn cuda_update_policy_bayesian_stats(
        alignment_scores: *const c_float,
        policy_successes: *mut c_float,
        policy_total_trials: *mut c_float,
        policy_success_rates: *mut c_float,
        n_agents: c_int,
        n_policies: c_int,
        acceptance_threshold: c_float,
    );

    /// Rank policies by alignment score
    pub fn cuda_rank_policies_bitonic(
        policy_scores: *const c_float,
        ranking: *mut c_int,
        n_policies: c_int,
    );

    /// Compute weighted policy coherence (agreement metric)
    pub fn cuda_compute_policy_coherence(
        agent_policy_selections: *const c_int,
        agent_weights: *const c_float,
        policy_coherence: *mut c_float,
        n_agents: c_int,
        n_policies: c_int,
    );

    /// Compute policy scope conflicts based on Scope (Agent/Cluster/Swarm)
    pub fn cuda_compute_scope_conflicts(
        policy_scopes: *const c_int,
        agent_cluster_id: *const c_int,
        alignment_scores: *const c_float,
        scope_conflicts: *mut c_float,
        n_agents: c_int,
        n_policies: c_int,
    );
}

// ============================================================================
// Safe Rust wrappers
// ============================================================================

/// Safe wrapper for Brain δ embeddings computation
pub fn compute_agent_embeddings_gpu(
    metrics: &[f32],
    weights: &[f32],
    n_agents: usize,
    metric_dim: usize,
    embedding_dim: usize,
) -> Vec<f32> {
    let mut embeddings = vec![0.0f32; n_agents * embedding_dim];

    unsafe {
        cuda_compute_agent_embeddings(
            metrics.as_ptr(),
            weights.as_ptr(),
            embeddings.as_mut_ptr(),
            n_agents as c_int,
            metric_dim as c_int,
            embedding_dim as c_int,
        );
    }

    embeddings
}

/// Safe wrapper for forecast computation
pub fn forecast_agent_metrics_gpu(
    history: &[f32],
    n_agents: usize,
    n_history: usize,
    n_metrics: usize,
) -> Vec<f32> {
    let mut forecast = vec![0.0f32; n_agents * n_metrics];

    unsafe {
        cuda_forecast_agent_metrics(
            history.as_ptr(),
            forecast.as_mut_ptr(),
            n_agents as c_int,
            n_history as c_int,
            n_metrics as c_int,
        );
    }

    forecast
}

/// Safe wrapper for policy alignment computation
pub fn compute_policy_alignment_gpu(
    agent_behaviors: &[f32],
    policy_targets: &[f32],
    policy_priorities: &[f32],
    n_agents: usize,
    n_policies: usize,
    behavior_dim: usize,
) -> Vec<f32> {
    let mut alignment_scores = vec![0.0f32; n_agents * n_policies];

    unsafe {
        cuda_compute_policy_alignment(
            agent_behaviors.as_ptr(),
            policy_targets.as_ptr(),
            policy_priorities.as_ptr(),
            alignment_scores.as_mut_ptr(),
            n_agents as c_int,
            n_policies as c_int,
            behavior_dim as c_int,
        );
    }

    alignment_scores
}

/// Safe wrapper for Bayesian policy tracking
pub fn update_policy_bayesian_stats_gpu(
    alignment_scores: &[f32],
    n_agents: usize,
    n_policies: usize,
    acceptance_threshold: f32,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut policy_successes = vec![1.0f32; n_policies];
    let mut policy_total_trials = vec![1.0f32; n_policies];
    let mut policy_success_rates = vec![0.5f32; n_policies];

    unsafe {
        cuda_update_policy_bayesian_stats(
            alignment_scores.as_ptr(),
            policy_successes.as_mut_ptr(),
            policy_total_trials.as_mut_ptr(),
            policy_success_rates.as_mut_ptr(),
            n_agents as c_int,
            n_policies as c_int,
            acceptance_threshold,
        );
    }

    (policy_successes, policy_total_trials, policy_success_rates)
}

/// Safe wrapper for regime computation
pub fn compute_regimes_gpu(
    agent_metrics: &[f32],
    thresholds: &[f32],  // [stress_threshold, improvement_threshold, exploration_threshold]
    n_agents: usize,
) -> Vec<i32> {
    let mut regimes = vec![0i32; n_agents];

    unsafe {
        cuda_compute_regimes(
            agent_metrics.as_ptr(),
            thresholds.as_ptr(),
            regimes.as_mut_ptr(),
            n_agents as c_int,
        );
    }

    regimes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]  // Requires CUDA-capable GPU
    fn test_embedding_computation_gpu() {
        let n_agents = 100;
        let metric_dim = 8;
        let embedding_dim = 16;

        let metrics = vec![0.5f32; n_agents * metric_dim];
        let weights = vec![0.1f32; metric_dim * embedding_dim];

        let embeddings = compute_agent_embeddings_gpu(&metrics, &weights, n_agents, metric_dim, embedding_dim);

        assert_eq!(embeddings.len(), n_agents * embedding_dim);
        assert!(embeddings.iter().all(|&x| x >= 0.0));  // ReLU applied
    }

    #[test]
    #[ignore]  // Requires CUDA-capable GPU
    fn test_policy_alignment_gpu() {
        let n_agents = 100;
        let n_policies = 10;
        let behavior_dim = 8;

        let agent_behaviors = vec![0.5f32; n_agents * behavior_dim];
        let policy_targets = vec![0.3f32; n_policies * behavior_dim];
        let policy_priorities = vec![1.0f32; n_policies];

        let scores = compute_policy_alignment_gpu(
            &agent_behaviors,
            &policy_targets,
            &policy_priorities,
            n_agents,
            n_policies,
            behavior_dim,
        );

        assert_eq!(scores.len(), n_agents * n_policies);
    }
}
