//! Benchmarking harness — measure throughput, latency, memory for large-scale populations.
//! ADR 0010 §6: Performance characterization at 100K agents, 10K ticks.

use std::time::Instant;
use super::agent_hierarchy::AgentHierarchy;

/// Benchmark configuration.
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    /// Total Nano agents to spawn
    pub total_nano_agents: usize,
    /// Number of ticks to simulate
    pub num_ticks: u64,
    /// Report granularity (report stats every N ticks)
    pub report_interval_ticks: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            total_nano_agents: 100_000,
            num_ticks: 10_000,
            report_interval_ticks: 1000,
        }
    }
}

/// Per-tick metrics.
#[derive(Clone, Debug)]
pub struct TickMetrics {
    pub tick: u64,
    pub elapsed_us: u64,
    pub total_agents: usize,
}

/// Aggregated benchmark results.
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub config: BenchmarkConfig,
    pub total_agents: usize,
    pub total_ticks_completed: u64,
    pub total_wall_time_us: u64,
    pub throughput_agents_per_us: f64,
    pub latency_per_tick_us: f64,
    pub ticks_per_second: f64,
    pub per_tick_metrics: Vec<TickMetrics>,
}

impl BenchmarkResult {
    /// Summary report as a formatted string.
    pub fn summary(&self) -> String {
        format!(
            "=== Benchmark Results ===\n\
             Agents: {}\n\
             Ticks: {}\n\
             Total Time: {} us ({:.2} sec)\n\
             Throughput: {:.2} agents/us ({:.2}M agents/sec)\n\
             Latency/tick: {:.2} us\n\
             Ticks/sec: {:.0}\n\
             Samples: {}",
            self.total_agents,
            self.total_ticks_completed,
            self.total_wall_time_us,
            self.total_wall_time_us as f64 / 1_000_000.0,
            self.throughput_agents_per_us,
            self.throughput_agents_per_us * 1_000_000.0 / 1_000_000.0,
            self.latency_per_tick_us,
            self.ticks_per_second,
            self.per_tick_metrics.len(),
        )
    }
}

/// Build a hierarchy for benchmarking.
fn build_benchmark_hierarchy(total_nano: usize) -> AgentHierarchy {
    let mut hier = AgentHierarchy::new();
    hier.spawn_super();
    hier.spawn_sub(50).expect("Failed to spawn Sub tier");

    // Distribute Nano agents across Micro leaders
    // Assume 20 Nano per Micro = 1000 Micro for 20K Nano, etc.
    let micro_count = (total_nano / 20).min(1000);
    hier.spawn_micro(micro_count as u32, 0)
        .expect("Failed to spawn Micro tier");

    // Spawn Nano in batches (max 50K per call)
    let mut spawned = 0;
    let mut micro_idx = 0u32;
    while spawned < total_nano {
        let batch = ((total_nano - spawned).min(50000)) as u32;
        hier.spawn_nano(batch, micro_idx)
            .expect("Failed to spawn Nano tier");
        spawned += batch as usize;
        micro_idx = (micro_idx + 1) % (micro_count as u32);
    }

    hier
}

/// Run a benchmark of the agent hierarchy.
pub fn run_benchmark(config: BenchmarkConfig) -> BenchmarkResult {
    let mut hierarchy = build_benchmark_hierarchy(config.total_nano_agents);
    let total_agents = hierarchy.total_agents();

    let start = Instant::now();
    let mut tick_metrics = Vec::new();

    for tick in 0..config.num_ticks {
        let tick_start = Instant::now();

        // Simulate tick: broadcast to all agents
        hierarchy.broadcast_tick(tick);

        let tick_elapsed = tick_start.elapsed().as_micros() as u64;

        if (tick + 1) % config.report_interval_ticks == 0 {
            tick_metrics.push(TickMetrics {
                tick: tick + 1,
                elapsed_us: tick_elapsed,
                total_agents,
            });
        }
    }

    let total_wall_time_us = start.elapsed().as_micros() as u64;

    let throughput_agents_per_us = if total_wall_time_us > 0 {
        (total_agents as u64 * config.num_ticks) as f64 / total_wall_time_us as f64
    } else {
        0.0
    };

    let latency_per_tick_us = if config.num_ticks > 0 {
        total_wall_time_us as f64 / config.num_ticks as f64
    } else {
        0.0
    };

    let ticks_per_second = if total_wall_time_us > 0 {
        config.num_ticks as f64 * 1_000_000.0 / total_wall_time_us as f64
    } else {
        0.0
    };

    let num_ticks = config.num_ticks;
    BenchmarkResult {
        config,
        total_agents,
        total_ticks_completed: num_ticks,
        total_wall_time_us,
        throughput_agents_per_us,
        latency_per_tick_us,
        ticks_per_second,
        per_tick_metrics: tick_metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_100k_agents_10k_ticks() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.total_nano_agents, 100_000);
        assert_eq!(config.num_ticks, 10_000);
    }

    #[test]
    fn build_hierarchy_respects_agent_count() {
        let hier = build_benchmark_hierarchy(50000);
        // 50K Nano + 1 Super + 50 Sub + ~2500 Micro (50K/20)
        let expected_hierarchy = 50000 + 1 + 50 + (50000 / 20).min(1000);
        assert_eq!(hier.total_agents(), expected_hierarchy);
    }

    #[test]
    fn benchmark_result_has_valid_metrics() {
        let config = BenchmarkConfig {
            total_nano_agents: 1000,
            num_ticks: 100,
            report_interval_ticks: 50,
        };
        let result = run_benchmark(config);
        assert!(result.total_wall_time_us > 0);
        assert!(result.throughput_agents_per_us > 0.0);
        assert!(result.latency_per_tick_us > 0.0);
        assert!(result.ticks_per_second > 0.0);
    }

    #[test]
    fn benchmark_completes_all_ticks() {
        let config = BenchmarkConfig {
            total_nano_agents: 100,
            num_ticks: 10,
            report_interval_ticks: 5,
        };
        let result = run_benchmark(config);
        assert_eq!(result.total_ticks_completed, 10);
    }

    #[test]
    fn benchmark_reports_per_tick_metrics() {
        let config = BenchmarkConfig {
            total_nano_agents: 500,
            num_ticks: 100,
            report_interval_ticks: 10,
        };
        let result = run_benchmark(config);
        assert!(!result.per_tick_metrics.is_empty());
        // 100 / 10 = 10 reports
        assert!(result.per_tick_metrics.len() >= 5);
    }

    #[test]
    fn benchmark_summary_contains_key_metrics() {
        let config = BenchmarkConfig {
            total_nano_agents: 1000,
            num_ticks: 50,
            report_interval_ticks: 25,
        };
        let result = run_benchmark(config);
        let summary = result.summary();
        assert!(summary.contains("Agents:"));
        assert!(summary.contains("Ticks:"));
        assert!(summary.contains("Throughput:"));
        assert!(summary.contains("Latency"));
    }

    #[test]
    fn benchmark_small_run() {
        let config = BenchmarkConfig {
            total_nano_agents: 100,
            num_ticks: 5,
            report_interval_ticks: 1,
        };
        let result = run_benchmark(config);
        assert_eq!(result.total_ticks_completed, 5);
        assert!(result.total_wall_time_us > 0);
    }
}
