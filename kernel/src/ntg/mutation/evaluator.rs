//! Fitness evaluation: measure real graph performance against target hardware.
//!
//! ADR 0002 rule 3 requires that fitness is "real and fast enough to run
//! inside the budget" — not a slow proxy. This module measures actual
//! forward-pass latency and memory usage.

use super::super::error::NtgError;
use super::super::graph::Graph;
use std::time::Instant;

/// Fitness score: dual-objective (latency + memory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FitnessScore {
    pub latency_us: u64,
    pub memory_bytes: u64,
}

impl FitnessScore {
    /// Pareto dominance: self is better if both metrics are <= other's.
    pub fn dominates(&self, other: &FitnessScore) -> bool {
        self.latency_us <= other.latency_us && self.memory_bytes <= other.memory_bytes
    }

    /// Weighted scalar score for easier comparison (80% latency, 20% memory).
    /// Lower is better.
    pub fn weighted_score(&self) -> f32 {
        0.8 * self.latency_us as f32 / 10_000.0 + 0.2 * self.memory_bytes as f32 / 1_000_000.0
    }
}

pub struct FitnessEvaluator {
    /// How many forward passes to run per evaluation (for averaging)
    pub runs_per_eval: usize,
}

impl FitnessEvaluator {
    pub fn new() -> Self {
        Self {
            runs_per_eval: 3, // 3 runs per mutation evaluation, take median
        }
    }

    /// Measure a graph's fitness: run forward pass, measure time + memory.
    pub fn measure_graph(
        &self,
        graph: &Graph,
        _baseline: (u64, u64),
    ) -> Result<(u64, u64), NtgError> {
        let mut latencies = Vec::new();
        let mut memories = Vec::new();

        for _ in 0..self.runs_per_eval {
            let start = Instant::now();

            // Run the forward pass
            let _result = graph.forward_pass()?;

            let elapsed = start.elapsed();
            let latency_us = (elapsed.as_micros() as u64).max(1); // At least 1us

            // Approximate memory as node_count * 256 bytes per node
            // (in reality, would use profiling tools like valgrind/heaptrack)
            let approx_memory = (graph.node_count() as u64) * 256;

            latencies.push(latency_us);
            memories.push(approx_memory);
        }

        // Return median latency, median memory
        latencies.sort();
        memories.sort();

        let median_latency = latencies[latencies.len() / 2];
        let median_memory = memories[memories.len() / 2];

        Ok((median_latency, median_memory))
    }

    /// Evaluate improvement from baseline to new measurement.
    /// Returns true if new is better (lower latency + memory).
    pub fn is_improvement(
        baseline: (u64, u64),
        new: (u64, u64),
        threshold: f32,
    ) -> bool {
        let baseline_score = FitnessScore {
            latency_us: baseline.0,
            memory_bytes: baseline.1,
        };
        let new_score = FitnessScore {
            latency_us: new.0,
            memory_bytes: new.1,
        };

        // Both must improve (within threshold) for acceptance
        new_score.latency_us as f32 / baseline_score.latency_us as f32 <= threshold
            && new_score.memory_bytes as f32 / baseline_score.memory_bytes as f32 <= threshold
    }
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitness_score_dominance() {
        let s1 = FitnessScore {
            latency_us: 4000,
            memory_bytes: 900,
        };
        let s2 = FitnessScore {
            latency_us: 5000,
            memory_bytes: 1000,
        };
        assert!(s1.dominates(&s2)); // s1 is better in both dimensions
        assert!(!s2.dominates(&s1));
    }

    #[test]
    fn fitness_score_no_dominance_mixed() {
        let s1 = FitnessScore {
            latency_us: 4000,
            memory_bytes: 1100,
        };
        let s2 = FitnessScore {
            latency_us: 5000,
            memory_bytes: 1000,
        };
        assert!(!s1.dominates(&s2)); // s1 better latency but worse memory
        assert!(!s2.dominates(&s1));
    }

    #[test]
    fn fitness_weighted_score() {
        let s1 = FitnessScore {
            latency_us: 5000,
            memory_bytes: 1_000_000,
        };
        let s2 = FitnessScore {
            latency_us: 4000,
            memory_bytes: 1_000_000,
        };
        assert!(s2.weighted_score() < s1.weighted_score()); // s2 is better
    }

    #[test]
    fn fitness_improvement_check_both_better() {
        let baseline = (5000, 1024);
        let improved = (4750, 973); // ~5% better
        assert!(FitnessEvaluator::is_improvement(
            baseline,
            improved,
            1.01 // 1% threshold
        ));
    }

    #[test]
    fn fitness_improvement_check_regression() {
        let baseline = (5000, 1024);
        let regressed = (5100, 1100); // Worse
        assert!(!FitnessEvaluator::is_improvement(
            baseline,
            regressed,
            1.01
        ));
    }

    #[test]
    fn fitness_improvement_check_mixed() {
        let baseline = (5000, 1024);
        let mixed = (4500, 1100); // Better latency, worse memory
        assert!(!FitnessEvaluator::is_improvement(
            baseline,
            mixed,
            1.01
        ));
    }

    #[test]
    fn evaluator_new() {
        let eval = FitnessEvaluator::new();
        assert_eq!(eval.runs_per_eval, 3);
    }
}
