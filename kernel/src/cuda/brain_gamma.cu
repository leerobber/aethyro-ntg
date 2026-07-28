// CUDA kernels for Brain γ (Governance) — policy alignment scoring
// Accelerates: policy alignment computation (agents × policies matrix operations)

#include <cuda_runtime.h>
#include <math.h>

// ============================================================================
// Kernel 1: Compute policy alignment scores (agents × policies)
// ============================================================================
// Input:
//   agent_behaviors[n_agents][8] (agent behavior vectors, 8-dim)
//   policy_targets[n_policies][8] (policy target vectors, 8-dim)
//   policy_priorities[n_policies] (priority weight: Low=0.5, Normal=1.0, High=2.0, Critical=4.0)
//
// Output:
//   alignment_scores[n_agents][n_policies] (dot product, scaled by priority)
//
// Formula: alignment[a, p] = (behavior[a] · target[p]) * priority[p]
__global__ void compute_policy_alignment(
    const float* __restrict__ agent_behaviors,
    const float* __restrict__ policy_targets,
    const float* __restrict__ policy_priorities,
    float* __restrict__ alignment_scores,
    int n_agents,
    int n_policies,
    int behavior_dim
) {
    int agent_id = blockIdx.x;
    int policy_id = blockIdx.y * blockDim.x + threadIdx.x;

    if (policy_id < n_policies) {
        float dot_product = 0.0f;

        // Compute dot product: behavior[agent_id] · target[policy_id]
        for (int d = 0; d < behavior_dim; d++) {
            dot_product += agent_behaviors[agent_id * behavior_dim + d] *
                          policy_targets[policy_id * behavior_dim + d];
        }

        // Scale by policy priority and store
        float priority = policy_priorities[policy_id];
        alignment_scores[agent_id * n_policies + policy_id] = dot_product * priority;
    }
}

// ============================================================================
// Kernel 2: Update Bayesian success tracking (Thompson sampling)
// ============================================================================
// Input:
//   alignment_scores[n_agents][n_policies] (from kernel 1)
//   policy_successes[n_policies] (Beta distribution alpha)
//   policy_total_trials[n_policies] (Beta distribution beta)
//   acceptance_threshold (default 0.6)
//
// Output:
//   policy_successes (updated alpha)
//   policy_total_trials (updated beta)
//   policy_success_rates[n_policies] (new success rate = alpha / (alpha + beta))
__global__ void update_policy_bayesian_stats(
    const float* __restrict__ alignment_scores,
    float* __restrict__ policy_successes,
    float* __restrict__ policy_total_trials,
    float* __restrict__ policy_success_rates,
    int n_agents,
    int n_policies,
    float acceptance_threshold
) {
    int policy_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (policy_id < n_policies) {
        float successes = 0.0f;
        float trials = 0.0f;

        // Count successes across all agents for this policy
        for (int a = 0; a < n_agents; a++) {
            float score = alignment_scores[a * n_policies + policy_id];
            trials += 1.0f;
            if (score >= acceptance_threshold) {
                successes += 1.0f;
            }
        }

        // Update Beta distribution (Bayesian conjugate prior)
        float alpha = policy_successes[policy_id];
        float beta = policy_total_trials[policy_id];

        alpha += successes;
        beta += (trials - successes);

        policy_successes[policy_id] = alpha;
        policy_total_trials[policy_id] = beta;
        policy_success_rates[policy_id] = alpha / (alpha + beta);
    }
}

// ============================================================================
// Kernel 3: Rank policies by alignment (parallel sort via bitonic network)
// ============================================================================
// Input: policy_scores[n_policies] (alignment scores aggregated per policy)
// Output: ranking[n_policies] (sorted indices, highest first)
//
// Note: For small n_policies (< 512), bitonic sort is efficient.
// For larger counts, use this on the CPU or employ GPU merge-sort.
__global__ void rank_policies_bitonic(
    const float* __restrict__ policy_scores,
    int* __restrict__ ranking,
    int n_policies
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;

    if (idx == 0) {
        // Simple implementation: count how many scores are higher than this one
        for (int i = 0; i < n_policies; i++) {
            int rank = 0;
            for (int j = 0; j < n_policies; j++) {
                if (policy_scores[j] > policy_scores[i] ||
                    (policy_scores[j] == policy_scores[i] && j < i)) {
                    rank++;
                }
            }
            ranking[rank] = i;
        }
    }
}

// ============================================================================
// Kernel 4: Compute policy coherence (mutual agreement across agents)
// ============================================================================
// Input:
//   agent_policy_selections[n_agents][n_policies] (binary: 1 if agent selects policy, 0 otherwise)
//   agent_weights[n_agents] (influence weight per agent, proportional to fitness)
//
// Output:
//   policy_coherence[n_policies] (weighted agreement metric, range [0, 1])
//
// Formula: coherence[p] = (sum of weights for agents selecting p) / (total weight)
__global__ void compute_policy_coherence(
    const int* __restrict__ agent_policy_selections,
    const float* __restrict__ agent_weights,
    float* __restrict__ policy_coherence,
    int n_agents,
    int n_policies
) {
    int policy_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (policy_id < n_policies) {
        float weighted_support = 0.0f;
        float total_weight = 0.0f;

        for (int a = 0; a < n_agents; a++) {
            float weight = agent_weights[a];
            total_weight += weight;

            if (agent_policy_selections[a * n_policies + policy_id] == 1) {
                weighted_support += weight;
            }
        }

        if (total_weight > 0.0f) {
            policy_coherence[policy_id] = weighted_support / total_weight;
        } else {
            policy_coherence[policy_id] = 0.0f;
        }
    }
}

// ============================================================================
// Kernel 5: Compute policy scope conflicts (Scope: Agent/Cluster/Swarm)
// ============================================================================
// Input:
//   policy_scopes[n_policies] (0=Agent, 1=Cluster, 2=Swarm)
//   agent_cluster_id[n_agents] (which cluster each agent belongs to)
//   alignment_scores[n_agents][n_policies]
//
// Output:
//   scope_conflicts[n_policies] (conflict severity, 0=no conflict, 1=severe conflict)
//
// Logic:
//   - Agent-scoped policies: agents can disagree (low conflict)
//   - Cluster-scoped: agents within same cluster should align (medium conflict if they don't)
//   - Swarm-scoped: all agents should align (high conflict if they don't)
__global__ void compute_scope_conflicts(
    const int* __restrict__ policy_scopes,
    const int* __restrict__ agent_cluster_id,
    const float* __restrict__ alignment_scores,
    float* __restrict__ scope_conflicts,
    int n_agents,
    int n_policies
) {
    int policy_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (policy_id < n_policies) {
        int scope = policy_scopes[policy_id];

        if (scope == 0) {
            // Agent-scoped: no conflict expected
            scope_conflicts[policy_id] = 0.0f;
        } else if (scope == 1) {
            // Cluster-scoped: check alignment within clusters
            float max_variance = 0.0f;

            for (int c = 0; c < 256; c++) {  // Max 256 clusters (heuristic)
                float mean = 0.0f, count = 0.0f;

                for (int a = 0; a < n_agents; a++) {
                    if (agent_cluster_id[a] == c) {
                        mean += alignment_scores[a * n_policies + policy_id];
                        count += 1.0f;
                    }
                }

                if (count > 0.0f) {
                    mean /= count;

                    float variance = 0.0f;
                    for (int a = 0; a < n_agents; a++) {
                        if (agent_cluster_id[a] == c) {
                            float diff = alignment_scores[a * n_policies + policy_id] - mean;
                            variance += diff * diff;
                        }
                    }

                    if (count > 1.0f) {
                        variance /= (count - 1.0f);
                        max_variance = fmaxf(max_variance, sqrtf(variance));
                    }
                }
            }

            scope_conflicts[policy_id] = fminf(max_variance, 1.0f);
        } else {
            // Swarm-scoped: check global alignment
            float mean = 0.0f;
            for (int a = 0; a < n_agents; a++) {
                mean += alignment_scores[a * n_policies + policy_id];
            }
            mean /= (float)n_agents;

            float variance = 0.0f;
            for (int a = 0; a < n_agents; a++) {
                float diff = alignment_scores[a * n_policies + policy_id] - mean;
                variance += diff * diff;
            }
            variance /= ((float)n_agents - 1.0f);

            scope_conflicts[policy_id] = fminf(sqrtf(variance), 1.0f);
        }
    }
}

// ============================================================================
// Host wrapper functions (called from Rust)
// ============================================================================

extern "C" {

void cuda_compute_policy_alignment(
    const float* agent_behaviors,
    const float* policy_targets,
    const float* policy_priorities,
    float* alignment_scores,
    int n_agents,
    int n_policies,
    int behavior_dim
) {
    dim3 blocks(n_agents, (n_policies + 255) / 256);
    dim3 threads(256);

    compute_policy_alignment<<<blocks, threads>>>(
        agent_behaviors, policy_targets, policy_priorities, alignment_scores,
        n_agents, n_policies, behavior_dim
    );
    cudaDeviceSynchronize();
}

void cuda_update_policy_bayesian_stats(
    const float* alignment_scores,
    float* policy_successes,
    float* policy_total_trials,
    float* policy_success_rates,
    int n_agents,
    int n_policies,
    float acceptance_threshold
) {
    int threads_per_block = 256;
    int blocks = (n_policies + threads_per_block - 1) / threads_per_block;

    update_policy_bayesian_stats<<<blocks, threads_per_block>>>(
        alignment_scores, policy_successes, policy_total_trials,
        policy_success_rates, n_agents, n_policies, acceptance_threshold
    );
    cudaDeviceSynchronize();
}

void cuda_rank_policies_bitonic(
    const float* policy_scores,
    int* ranking,
    int n_policies
) {
    rank_policies_bitonic<<<1, 1>>>(policy_scores, ranking, n_policies);
    cudaDeviceSynchronize();
}

void cuda_compute_policy_coherence(
    const int* agent_policy_selections,
    const float* agent_weights,
    float* policy_coherence,
    int n_agents,
    int n_policies
) {
    int threads_per_block = 256;
    int blocks = (n_policies + threads_per_block - 1) / threads_per_block;

    compute_policy_coherence<<<blocks, threads_per_block>>>(
        agent_policy_selections, agent_weights, policy_coherence,
        n_agents, n_policies
    );
    cudaDeviceSynchronize();
}

void cuda_compute_scope_conflicts(
    const int* policy_scopes,
    const int* agent_cluster_id,
    const float* alignment_scores,
    float* scope_conflicts,
    int n_agents,
    int n_policies
) {
    int threads_per_block = 256;
    int blocks = (n_policies + threads_per_block - 1) / threads_per_block;

    compute_scope_conflicts<<<blocks, threads_per_block>>>(
        policy_scopes, agent_cluster_id, alignment_scores, scope_conflicts,
        n_agents, n_policies
    );
    cudaDeviceSynchronize();
}

}  // extern "C"
