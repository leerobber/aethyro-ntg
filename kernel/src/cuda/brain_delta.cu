// CUDA kernels for Brain δ (Perception) — embedding computation and forecasting
// Accelerates: agent embeddings, cluster metrics, swarm metrics, trend forecasting

#include <cuda_runtime.h>
#include <math.h>

// ============================================================================
// Kernel 1: Compute 16-dimensional agent embeddings from local metrics
// ============================================================================
// Input: metrics[n_agents][8] (LocalMetrics: stress, growth, repair, coordination, etc.)
// Output: embeddings[n_agents][16] (16-dim agent embeddings via linear projection)
// Weights: W[8][16] (learned embedding matrix)
__global__ void compute_agent_embeddings(
    const float* __restrict__ metrics,
    const float* __restrict__ W,
    float* __restrict__ embeddings,
    int n_agents,
    int metric_dim,
    int embedding_dim
) {
    int agent_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (agent_id < n_agents) {
        // Compute embedding = metrics @ W (linear projection)
        for (int d = 0; d < embedding_dim; d++) {
            float val = 0.0f;
            for (int m = 0; m < metric_dim; m++) {
                val += metrics[agent_id * metric_dim + m] * W[m * embedding_dim + d];
            }
            // Apply ReLU activation
            embeddings[agent_id * embedding_dim + d] = fmaxf(val, 0.0f);
        }
    }
}

// ============================================================================
// Kernel 2: Aggregate local embeddings → cluster embeddings (tree reduction)
// ============================================================================
// Input: agent_embeddings[n_agents][8] (per-agent 8-dim state)
// Output: cluster_embeddings[n_clusters][8] (aggregated per cluster)
// cluster_assignment[n_agents] (which cluster each agent belongs to)
__global__ void aggregate_cluster_metrics(
    const float* __restrict__ agent_embeddings,
    const int* __restrict__ cluster_assignment,
    float* __restrict__ cluster_embeddings,
    float* __restrict__ cluster_counts,
    int n_agents,
    int embedding_dim
) {
    int agent_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (agent_id < n_agents) {
        int cluster_id = cluster_assignment[agent_id];

        // Atomic add: accumulate agent embedding into cluster
        for (int d = 0; d < embedding_dim; d++) {
            atomicAdd(
                &cluster_embeddings[cluster_id * embedding_dim + d],
                agent_embeddings[agent_id * embedding_dim + d]
            );
        }
        if (d == 0) {
            atomicAdd(&cluster_counts[cluster_id], 1.0f);
        }
    }
}

// Normalize cluster embeddings by count
__global__ void normalize_cluster_embeddings(
    float* __restrict__ cluster_embeddings,
    const float* __restrict__ cluster_counts,
    int n_clusters,
    int embedding_dim
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;

    if (idx < n_clusters * embedding_dim) {
        int cluster_id = idx / embedding_dim;
        float count = cluster_counts[cluster_id];
        if (count > 0.0f) {
            cluster_embeddings[idx] /= count;
        }
    }
}

// ============================================================================
// Kernel 3: Compute forecasted values (linear regression on history)
// ============================================================================
// Input: history[n_agents][20][4] (20-cycle history, 4 metrics: latency, memory, stress, growth)
// Output: forecast[n_agents][4] (predicted next cycle values)
//
// Uses least-squares linear regression: y = slope*t + intercept
__global__ void forecast_agent_metrics(
    const float* __restrict__ history,
    float* __restrict__ forecast,
    int n_agents,
    int n_history,  // 20 cycles
    int n_metrics   // 4: latency, memory, stress, growth
) {
    int agent_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (agent_id < n_agents) {
        // For each metric, fit a line and extrapolate
        for (int m = 0; m < n_metrics; m++) {
            float sum_t = 0.0f, sum_y = 0.0f, sum_ty = 0.0f, sum_t2 = 0.0f;

            // Compute regression coefficients
            for (int t = 0; t < n_history; t++) {
                float y = history[agent_id * n_history * n_metrics + t * n_metrics + m];
                sum_t += (float)t;
                sum_y += y;
                sum_ty += (float)t * y;
                sum_t2 += (float)t * (float)t;
            }

            float n = (float)n_history;
            float denom = n * sum_t2 - sum_t * sum_t;

            if (fabsf(denom) < 1e-6f) {
                // Degenerate case: use mean
                forecast[agent_id * n_metrics + m] = sum_y / n;
            } else {
                // Slope: (n*sum_ty - sum_t*sum_y) / denom
                float slope = (n * sum_ty - sum_t * sum_y) / denom;
                // Intercept: (sum_y - slope*sum_t) / n
                float intercept = (sum_y - slope * sum_t) / n;
                // Predict at t=n_history (next cycle)
                forecast[agent_id * n_metrics + m] = slope * (float)n_history + intercept;
            }
        }
    }
}

// ============================================================================
// Kernel 4: Compute swarm-level statistics (reduction over clusters)
// ============================================================================
// Input: cluster_metrics[n_clusters][8]
// Output: swarm_metrics[8] (global aggregates)
__global__ void compute_swarm_metrics(
    const float* __restrict__ cluster_metrics,
    float* __restrict__ swarm_metrics,
    int n_clusters,
    int metric_dim
) {
    int metric_id = threadIdx.x;

    if (metric_id < metric_dim) {
        // Parallel reduction within thread block
        extern __shared__ float sdata[];

        float val = 0.0f;
        for (int c = threadIdx.x; c < n_clusters; c += blockDim.x) {
            val += cluster_metrics[c * metric_dim + metric_id];
        }
        sdata[threadIdx.x] = val;
        __syncthreads();

        // Tree reduction
        for (int s = blockDim.x / 2; s > 0; s >>= 1) {
            if (threadIdx.x < s) {
                sdata[threadIdx.x] += sdata[threadIdx.x + s];
            }
            __syncthreads();
        }

        if (threadIdx.x == 0) {
            swarm_metrics[metric_id] = sdata[0];
        }
    }
}

// ============================================================================
// Kernel 5: Compute perception regime (normal/stressed/improving/exploring)
// ============================================================================
// Input: agent_metrics[n_agents][4] (latency, memory, stress, growth)
// Output: regimes[n_agents] (regime label: 0=normal, 1=stressed, 2=improving, 3=exploring)
__global__ void compute_regimes(
    const float* __restrict__ agent_metrics,
    const float* __restrict__ thresholds,  // stress_threshold, improvement_threshold, exploration_threshold
    int* __restrict__ regimes,
    int n_agents
) {
    int agent_id = blockIdx.x * blockDim.x + threadIdx.x;

    if (agent_id < n_agents) {
        float stress = agent_metrics[agent_id * 4 + 2];
        float growth = agent_metrics[agent_id * 4 + 3];

        float stress_threshold = thresholds[0];
        float improvement_threshold = thresholds[1];
        float exploration_threshold = thresholds[2];

        int regime;
        if (stress > stress_threshold) {
            regime = 1;  // stressed
        } else if (growth > improvement_threshold) {
            regime = 2;  // improving
        } else if (growth > exploration_threshold) {
            regime = 3;  // exploring
        } else {
            regime = 0;  // normal
        }

        regimes[agent_id] = regime;
    }
}

// ============================================================================
// Host wrapper functions (called from Rust)
// ============================================================================

extern "C" {

void cuda_compute_agent_embeddings(
    const float* metrics,
    const float* W,
    float* embeddings,
    int n_agents,
    int metric_dim,
    int embedding_dim
) {
    int threads_per_block = 256;
    int blocks = (n_agents + threads_per_block - 1) / threads_per_block;

    compute_agent_embeddings<<<blocks, threads_per_block>>>(
        metrics, W, embeddings, n_agents, metric_dim, embedding_dim
    );
    cudaDeviceSynchronize();
}

void cuda_aggregate_cluster_metrics(
    const float* agent_embeddings,
    const int* cluster_assignment,
    float* cluster_embeddings,
    float* cluster_counts,
    int n_agents,
    int embedding_dim
) {
    int threads_per_block = 256;
    int blocks = (n_agents + threads_per_block - 1) / threads_per_block;

    aggregate_cluster_metrics<<<blocks, threads_per_block>>>(
        agent_embeddings, cluster_assignment, cluster_embeddings,
        cluster_counts, n_agents, embedding_dim
    );
    cudaDeviceSynchronize();
}

void cuda_normalize_cluster_embeddings(
    float* cluster_embeddings,
    const float* cluster_counts,
    int n_clusters,
    int embedding_dim
) {
    int threads_per_block = 256;
    int total_elements = n_clusters * embedding_dim;
    int blocks = (total_elements + threads_per_block - 1) / threads_per_block;

    normalize_cluster_embeddings<<<blocks, threads_per_block>>>(
        cluster_embeddings, cluster_counts, n_clusters, embedding_dim
    );
    cudaDeviceSynchronize();
}

void cuda_forecast_agent_metrics(
    const float* history,
    float* forecast,
    int n_agents,
    int n_history,
    int n_metrics
) {
    int threads_per_block = 256;
    int blocks = (n_agents + threads_per_block - 1) / threads_per_block;

    forecast_agent_metrics<<<blocks, threads_per_block>>>(
        history, forecast, n_agents, n_history, n_metrics
    );
    cudaDeviceSynchronize();
}

void cuda_compute_swarm_metrics(
    const float* cluster_metrics,
    float* swarm_metrics,
    int n_clusters,
    int metric_dim
) {
    int shared_mem = 256 * sizeof(float);

    compute_swarm_metrics<<<1, 256, shared_mem>>>(
        cluster_metrics, swarm_metrics, n_clusters, metric_dim
    );
    cudaDeviceSynchronize();
}

void cuda_compute_regimes(
    const float* agent_metrics,
    const float* thresholds,
    int* regimes,
    int n_agents
) {
    int threads_per_block = 256;
    int blocks = (n_agents + threads_per_block - 1) / threads_per_block;

    compute_regimes<<<blocks, threads_per_block>>>(
        agent_metrics, thresholds, regimes, n_agents
    );
    cudaDeviceSynchronize();
}

}  // extern "C"
