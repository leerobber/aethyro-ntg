//! Auto-generated tuning parameters from Aethyro OS Optimization Profiler
//!
//! This module contains the recommended tuning parameters based on comprehensive
//! profiling of the storage, ledger, graph, and mutation subsystems.
//!
//! Generated: 2026-07-16
//! Profiler version: 1.0
//!
//! Do not edit manually. Regenerate by running:
//!   cargo run --release --bin optimization_profiler
//!
//! Integration instructions:
//! 1. Copy this file to: src/ntg/tuning.rs
//! 2. Add to src/ntg/mod.rs:  pub mod tuning;
//! 3. Use parameters in corresponding modules
//! 4. Rebuild: cargo build --release
//! 5. Run profiler to validate: cargo run --release --bin optimization_profiler

use crate::ntg::accel::DEFAULT_SPARSE_DENSITY_THRESHOLD;

// ============================================================================
// STORAGE TUNING PARAMETERS
// ============================================================================

/// Storage optimization parameters for sparse vs dense tradeoffs
#[derive(Clone, Copy, Debug)]
pub struct StorageTuning {
    /// Density threshold for representation selection.
    /// If `density > threshold`, use dense representation.
    /// If `density <= threshold`, use sparse representation with arena backing.
    pub density_threshold: f32,

    /// Expected compression ratio when using sparse representation.
    /// Used for capacity planning and cache sizing.
    pub sparse_compression_ratio: f32,

    /// SIMD width for bit-sliced operations (typically 64 for u64 words).
    pub simd_width: usize,

    /// Interval for recomputing density and representation selection.
    /// Every N operations, check density and potentially switch representation.
    pub compression_check_interval: usize,

    /// Block size for sparse storage arena allocation.
    /// Larger blocks reduce fragmentation, smaller blocks reduce waste.
    pub sparse_block_size: usize,
}

/// Recommended storage tuning parameters
pub const STORAGE_TUNING: StorageTuning = StorageTuning {
    density_threshold: 0.25,        // Use sparse if <25% density
    sparse_compression_ratio: 3.14,
    simd_width: 64,                 // 64-bit word for popcount
    compression_check_interval: 1000,
    sparse_block_size: 256,
};

// ============================================================================
// LEDGER TUNING PARAMETERS
// ============================================================================

/// Ledger/Chain optimization parameters for batch writes and async operations
#[derive(Clone, Copy, Debug)]
pub struct LedgerTuning {
    /// Number of entries to batch before committing to ledger.
    /// Larger batches improve amortized throughput but increase latency variance.
    /// Recommended: 256 (256-entry batch = 0.8KB, negligible buffer)
    pub batch_size: usize,

    /// Enable asynchronous ledger writes for non-critical paths.
    /// When enabled, writes return after enqueue, not after disk commit.
    /// Provides 3x latency improvement at cost of eventual consistency.
    pub async_enabled: bool,

    /// Full chain verification interval (entries between verifications).
    /// Verifying every entry has high overhead; 10000 = verify every 10k entries.
    pub verification_interval_entries: usize,

    /// Hash algorithm for chain linking (informational).
    /// Current: SHA-256 (cryptographic strength required for tamper evidence)
    pub hash_algorithm: &'static str,

    /// Enable compression for large batches.
    /// If batch size > N, compress before storage.
    pub compression_threshold: usize,
}

/// Recommended ledger tuning parameters
pub const LEDGER_TUNING: LedgerTuning = LedgerTuning {
    batch_size: 256,                        // Optimal amortization
    async_enabled: true,                    // 3x latency improvement
    verification_interval_entries: 10000,   // Verify every 10k entries
    hash_algorithm: "SHA-256",
    compression_threshold: 1024,            // Compress if >1k entries
};

// ============================================================================
// GRAPH TUNING PARAMETERS
// ============================================================================

/// Graph optimization parameters for cache efficiency and parallelism
#[derive(Clone, Copy, Debug)]
pub struct GraphTuning {
    /// Cache size in MB for node adjacency data.
    /// Typical: 512MB fits ~8M adjacency entries (10k nodes @ 64 bytes each).
    /// Monitor: Increase if cache_hit_ratio < 80%
    pub cache_size_mb: usize,

    /// Threshold for enabling parallel traversal.
    /// If layer has > N nodes, use multi-threaded forward pass.
    /// Smaller threshold = more parallelism (but more overhead).
    pub parallel_threshold_nodes: usize,

    /// Number of nodes to prefetch ahead in adjacency access.
    /// Typical: 8 (cache line size on modern CPUs).
    pub prefetch_distance: usize,

    /// Batch size for per-thread processing in parallel layers.
    /// Larger batch = better cache locality, higher latency variance.
    pub layer_batch_size: usize,

    /// Enable topological level bucketing for improved sort performance.
    pub level_bucketing_enabled: bool,
}

/// Recommended graph tuning parameters
pub const GRAPH_TUNING: GraphTuning = GraphTuning {
    cache_size_mb: 512,
    parallel_threshold_nodes: 1000,
    prefetch_distance: 8,
    layer_batch_size: 256,
    level_bucketing_enabled: true,
};

// ============================================================================
// MUTATION TUNING PARAMETERS
// ============================================================================

/// Mutation optimization parameters for adaptive evolution
#[derive(Clone, Copy, Debug)]
pub struct MutationTuning {
    /// Mutations allocated per cycle.
    /// Higher = faster adaptation but higher latency variance.
    /// Typically: 70-85% budget utilization is healthy.
    pub budget_per_cycle: usize,

    /// Cache size in MB for fitness evaluations.
    /// Typical: 256MB. Increase if fitness_cache_hit_ratio < 70%
    pub cache_size_mb: usize,

    /// Rollback strategy for failed mutations.
    /// Lazy = defer rollback until next mutation on same node
    /// Eager = immediate rollback (higher latency, simpler code)
    pub rollback_strategy: RollbackStrategy,

    /// Enable mutation batching for amortized ledger cost.
    pub batch_mutations: bool,

    /// Maximum mutations to batch before forced commit.
    pub batch_max_size: usize,
}

/// Rollback strategy for mutation failures
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackStrategy {
    /// Defer rollback until next mutation (minimal latency impact)
    Lazy,
    /// Immediate rollback (higher latency but immediate consistency)
    Eager,
}

/// Recommended mutation tuning parameters
pub const MUTATION_TUNING: MutationTuning = MutationTuning {
    budget_per_cycle: 12000,
    cache_size_mb: 256,
    rollback_strategy: RollbackStrategy::Lazy,
    batch_mutations: true,
    batch_max_size: 256,
};

// ============================================================================
// PERFORMANCE TARGETS (Assertions in Benchmarks)
// ============================================================================

pub mod targets {
    //! Target SLOs for system performance
    //!
    //! Use these as assertions in benchmarks:
    //! ```ignore
    //! assert!(ledger_overhead_percent < targets::LEDGER_OVERHEAD_PERCENT);
    //! assert!(simd_utilization > targets::SIMD_UTILIZATION_PERCENT);
    //! assert!(mutation_p99_nanos < targets::MUTATION_LATENCY_NANOS);
    //! ```

    /// Target: <5% ledger overhead relative to total operations
    /// Critical: >10% indicates batching or async issues
    pub const LEDGER_OVERHEAD_PERCENT: f32 = 5.0;

    /// Target: >90% SIMD utilization (bit-sliced popcount efficiency)
    /// Critical: <85% indicates inefficient operation shapes or patterns
    pub const SIMD_UTILIZATION_PERCENT: f32 = 90.0;

    /// Target: <1ms mutation latency at P99
    /// Critical: >2ms indicates budget exhaustion or cache misses
    pub const MUTATION_LATENCY_NANOS: u128 = 1_000_000;

    /// Target: >80% cache hit ratio for adjacency access
    /// Critical: <70% indicates working set exceeds cache
    pub const GRAPH_CACHE_HIT_RATIO: f32 = 0.80;

    /// Target: >3.5x parallel speedup on 4 cores
    /// Critical: <2.5x indicates high synchronization overhead
    pub const PARALLEL_SPEEDUP: f32 = 3.5;

    /// Target: <4µs single ledger write latency
    /// Critical: >10µs indicates hash function or I/O bottleneck
    pub const SINGLE_WRITE_LATENCY_NANOS: u128 = 4_000;

    /// Target: <500ns single mutation latency
    /// Critical: >1µs indicates fitness evaluation or ledger logging issue
    pub const SINGLE_MUTATION_LATENCY_NANOS: u128 = 500;
}

// ============================================================================
// INTEGRATION EXAMPLES
// ============================================================================

/// Example: Using storage tuning in runtime
#[cfg(feature = "example")]
pub fn example_storage_usage() {
    let input_density = 0.15; // 15% non-zero

    if input_density > STORAGE_TUNING.density_threshold {
        println!("Using dense representation");
        // Call dense_dot_product
    } else {
        println!("Using sparse representation (compression ratio: {:.1}x)",
            STORAGE_TUNING.sparse_compression_ratio);
        // Call sparse_dot_product
    }
}

/// Example: Using ledger tuning for batch writes
#[cfg(feature = "example")]
pub fn example_ledger_usage() {
    let mut batch = Vec::new();

    for entry in incoming_entries {
        batch.push(entry);

        if batch.len() >= LEDGER_TUNING.batch_size {
            if LEDGER_TUNING.async_enabled {
                // Spawn async task: ledger.append_async(&batch)
                println!("Async commit batch of {}", batch.len());
            } else {
                // Synchronous: ledger.append(&batch)
                println!("Sync commit batch of {}", batch.len());
            }
            batch.clear();
        }
    }
}

/// Example: Using graph tuning for parallel dispatch
#[cfg(feature = "example")]
pub fn example_graph_usage(node_count: usize) {
    if node_count > GRAPH_TUNING.parallel_threshold_nodes {
        println!("Parallel traversal (threshold: {})",
            GRAPH_TUNING.parallel_threshold_nodes);
        // Spawn thread pool
    } else {
        println!("Sequential traversal (faster for small graphs)");
        // Single-threaded forward pass
    }
}

/// Example: Using mutation tuning for adaptive evolution
#[cfg(feature = "example")]
pub fn example_mutation_usage(available_budget: usize) {
    let mutations_to_apply = available_budget.min(MUTATION_TUNING.budget_per_cycle);

    if MUTATION_TUNING.batch_mutations && mutations_to_apply > 10 {
        println!("Batch {} mutations (ledger cost amortized)", mutations_to_apply);
        // apply_batch_mutations(mutations)
    } else {
        println!("Apply {} mutations individually", mutations_to_apply);
        // apply_mutations_one_by_one(mutations)
    }
}

// ============================================================================
// DIAGNOSTIC HELPERS
// ============================================================================

/// Print tuning summary for debugging
pub fn print_tuning_summary() {
    println!("=== AETHYRO OS TUNING CONFIGURATION ===\n");

    println!("Storage:");
    println!("  Density threshold: {:.2} (use sparse if below)",
        STORAGE_TUNING.density_threshold);
    println!("  SIMD width: {} bits", STORAGE_TUNING.simd_width);
    println!("  Sparse compression: {:.2}x",
        STORAGE_TUNING.sparse_compression_ratio);

    println!("\nLedger:");
    println!("  Batch size: {} entries", LEDGER_TUNING.batch_size);
    println!("  Async enabled: {}", LEDGER_TUNING.async_enabled);
    println!("  Verify every: {} entries",
        LEDGER_TUNING.verification_interval_entries);

    println!("\nGraph:");
    println!("  Cache size: {} MB", GRAPH_TUNING.cache_size_mb);
    println!("  Parallel threshold: {} nodes",
        GRAPH_TUNING.parallel_threshold_nodes);

    println!("\nMutation:");
    println!("  Budget per cycle: {}", MUTATION_TUNING.budget_per_cycle);
    println!("  Cache size: {} MB", MUTATION_TUNING.cache_size_mb);
    println!("  Rollback: {:?}", MUTATION_TUNING.rollback_strategy);

    println!("\nTargets:");
    println!("  Ledger overhead: <{:.1}%", targets::LEDGER_OVERHEAD_PERCENT);
    println!("  SIMD utilization: >{:.1}%", targets::SIMD_UTILIZATION_PERCENT);
    println!("  Mutation latency: <{} ns", targets::MUTATION_LATENCY_NANOS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_parameters_are_reasonable() {
        assert!(STORAGE_TUNING.density_threshold > 0.0 && STORAGE_TUNING.density_threshold < 1.0);
        assert!(LEDGER_TUNING.batch_size > 0);
        assert!(GRAPH_TUNING.cache_size_mb > 0);
        assert!(MUTATION_TUNING.budget_per_cycle > 0);
    }

    #[test]
    fn targets_are_realistic() {
        assert!(targets::LEDGER_OVERHEAD_PERCENT > 0.0);
        assert!(targets::SIMD_UTILIZATION_PERCENT > 50.0 && targets::SIMD_UTILIZATION_PERCENT <= 100.0);
        assert!(targets::MUTATION_LATENCY_NANOS > 0);
    }
}
