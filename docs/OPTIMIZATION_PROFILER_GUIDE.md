# Aethyro OS Optimization Profiler & Tuning Guide

## Overview

The Optimization Profiler is a comprehensive benchmarking harness that measures and optimizes the Aethyro OS integrated system across four critical subsystems:

1. **Storage** - Sparse vs dense ternary representation tradeoffs
2. **Ledger** - Cryptographic chain overhead and batch operation efficiency
3. **Graph** - Topological sort, cache efficiency, and parallel traversal
4. **Mutations** - Structural modifications, rollback, and fitness computation

### Targets

- **Ledger Overhead**: <5% of total system operations
- **SIMD Utilization**: >90% optimal (bit-sliced popcount efficiency)
- **Mutation Latency**: <1ms at P99 (single mutation operation)

---

## Building & Running

```bash
cd C:\Users\leer4\aethyro-ntg\kernel
cargo build --release --bin optimization_profiler
cargo run --release --bin optimization_profiler
```

### Output Files

1. **optimization_profile.json** - Complete profiling results in JSON format
2. **tuning_parameters.rs** - Auto-generated Rust module with recommended settings

---

## Component Profiling Details

### 1. Storage Profiling

**What it measures:**
- Dense ternary throughput (64-wide popcount operations)
- Sparse ternary throughput (hash map access patterns)
- Compression ratios for both representations
- Access latencies (microseconds)
- SIMD utilization percentage

**Key metrics:**
```json
{
  "dense_throughput": {
    "operation": "dense_write",
    "avg_nanos": 1234567,
    "min_nanos": 1200000,
    "max_nanos": 1300000,
    "p95_nanos": 1295000,
    "p99_nanos": 1298000
  },
  "sparse_throughput": {...},
  "optimal_density_threshold": 0.25,
  "simd_utilization_percent": 87.5
}
```

**Interpretation:**
- If `optimal_density_threshold` is 0.25, use sparse storage when <25% of elements are non-zero
- SIMD utilization >90% indicates good use of 64-wide bit-sliced operations
- Compression ratio indicates memory savings with sparse representation

**Tuning:**
- If SIMD utilization <90%, consider batch operations or vectorization
- If sparse compression ratio <2x, consider alternate sparse encoding (arena-based)
- Adjust `DEFAULT_SPARSE_DENSITY_THRESHOLD` in `accel.rs` based on optimal threshold

---

### 2. Ledger Profiling

**What it measures:**
- Single SHA-256 write latency (nanoseconds)
- Batch write amortized cost (per entry)
- Batch operation overhead percentage
- Async latency improvement
- Full-ledger verification cost
- Verification as percentage of write cost

**Key metrics:**
```json
{
  "single_write_nanos": 4567,
  "batch_write_nanos": 3200,
  "batch_overhead_percent": -29.9,
  "async_latency_nanos": 1370,
  "query_p99_nanos": 45670000,
  "verification_cost_percent": 28.5,
  "ledger_to_total_overhead_percent": 2.1
}
```

**Interpretation:**
- Negative batch overhead = batching is faster (good!)
- Async latency <30% of single write = async is beneficial
- Verification cost <40% = acceptable
- Ledger overhead <5% = system target achieved

**Tuning:**
- If ledger overhead >5%, enable batch writes with `LEDGER_TUNING.batch_size` = 256
- If verification cost >40%, implement Merkle tree for large ledgers
- If async latency improvement >30%, enable `ledger_async_enabled: true`
- Monitor `query_p99_nanos` for cache pressure on large ledgers

---

### 3. Graph Profiling

**What it measures:**
- Cache hit ratio (L1/L2 for adjacency access)
- Topological sort performance (Kahn's algorithm)
- Adjacency list access latency
- Parallel traversal speedup estimate
- Node cache size estimation
- Overall cache efficiency percentage

**Key metrics:**
```json
{
  "cache_hit_ratio": 0.87,
  "topo_sort_nanos": 23456789,
  "adjacency_access_nanos": 1234,
  "parallel_traversal_speedup": 3.8,
  "node_cache_size_bytes": 640000,
  "cache_efficiency_percent": 87.0
}
```

**Interpretation:**
- Cache hit ratio >80% = good memory access patterns
- Topo-sort scales with node count (log-linear expected)
- Parallel speedup 3.8x on 4 cores = good work distribution
- Cache efficiency >85% = memory layout is favorable

**Tuning:**
- If cache hit ratio <80%, increase `GRAPH_TUNING.cache_size_mb` from 512 to 1024
- If parallel speedup <3.0, reduce `GRAPH_TUNING.parallel_threshold_nodes` (parallelize smaller graphs)
- If topo-sort is slow, implement topological level bucketing
- Monitor cache_size_bytes vs available memory; ensure <20% system memory

---

### 4. Mutation Profiling

**What it measures:**
- Single mutation latency (weight change operation)
- Rollback operation latency (history replay)
- Fitness cache hit ratio
- Budget utilization percentage
- P99 latency for mutation operations

**Key metrics:**
```json
{
  "single_mutation_nanos": 456,
  "rollback_nanos": 2345,
  "fitness_cache_hit_ratio": 0.78,
  "budget_utilization_percent": 71.3,
  "mutation_latency_p99_nanos": 892
}
```

**Interpretation:**
- Single mutation <500ns = excellent performance
- Rollback <2x single mutation cost = efficient history
- Fitness cache hit ratio >70% = good cache sizing
- Budget utilization ~70% = healthy allocation (not over/under-provisioned)
- P99 latency <1ms = meets target

**Tuning:**
- If P99 latency >1ms, reduce `MUTATION_TUNING.budget_per_cycle` or use async rollback
- If fitness cache hit ratio <70%, increase `MUTATION_TUNING.cache_size_mb` from 256 to 512
- If budget utilization <50%, mutations are under-provisioned; consider more frequent updates
- If budget utilization >90%, may hit latency spikes; reduce budget_per_cycle

---

## Optimization Workflow

### 1. Baseline Profiling

```bash
# Initial profiling run
cargo run --release --bin optimization_profiler > baseline.txt

# Save the generated files
cp optimization_profile.json profile_baseline.json
cp tuning_parameters.rs src/tuning_baseline.rs
```

### 2. Analysis & Recommendations

The profiler auto-generates recommendations based on measured metrics:

```
RECOMMENDATIONS:
1. STORAGE: SIMD utilization at 87.5% (target >90%). Consider batch operations.
2. GRAPH: Cache hit ratio 78% (target >80%). Increase cache size or optimize pattern.
3. MUTATION: P99 latency 1200 ns (target <1ms). Optimize budget allocation.
```

### 3. Apply Tuning Parameters

The `tuning_parameters.rs` module is generated automatically:

```rust
pub const STORAGE_TUNING: StorageTuning = StorageTuning {
    density_threshold: 0.25,
    sparse_compression_ratio: 3.14,
};

pub const LEDGER_TUNING: LedgerTuning = LedgerTuning {
    batch_size: 256,
    async_enabled: true,
    verification_interval_entries: 10000,
};
```

Integrate into your code:

```rust
// In runtime initialization
if input_activations.density() > STORAGE_TUNING.density_threshold {
    use_dense_representation();
} else {
    use_sparse_representation();
}

// In ledger writes
if ledger_entries.len() >= LEDGER_TUNING.batch_size {
    batch_commit_async().await;
}
```

### 4. Rebuild & Retest

```bash
# Build with tuning parameters
cargo build --release

# Run profiler again
cargo run --release --bin optimization_profiler > iteration_1.txt
cp optimization_profile.json profile_iteration_1.json
```

### 5. Validate Target Achievement

Check the output for target compliance:

```
Storage:
  SIMD utilization: 92.1% ✓ (target >90%)

Ledger:
  Total overhead: 3.2% ✓ (target <5%)

Mutations:
  P99 latency: 876 ns ✓ (target <1ms)
```

---

## Performance Targets & SLOs

### Storage

| Metric | Target | Critical | Acceptable |
|--------|--------|----------|------------|
| SIMD Utilization | >90% | <85% | 85-90% |
| Sparse Compression Ratio | >2.5x | <2.0x | 2.0-2.5x |
| Dense Access Latency | <2µs | >3µs | 2-3µs |
| Sparse Access Latency | <5µs | >8µs | 5-8µs |

### Ledger

| Metric | Target | Critical | Acceptable |
|--------|--------|----------|------------|
| Overhead % | <5% | >10% | 5-10% |
| Single Write | <10µs | >20µs | 10-20µs |
| Batch Overhead | <5% | >10% | 5-10% |
| Verification Cost % | <40% | >60% | 40-60% |

### Graph

| Metric | Target | Critical | Acceptable |
|--------|--------|----------|------------|
| Cache Hit Ratio | >85% | <70% | 70-85% |
| Parallel Speedup | >3.5x | <2.5x | 2.5-3.5x |
| Topo-sort (10k nodes) | <25ms | >50ms | 25-50ms |

### Mutations

| Metric | Target | Critical | Acceptable |
|--------|--------|----------|------------|
| P99 Latency | <1ms | >2ms | 1-2ms |
| Fitness Cache Hit % | >75% | <60% | 60-75% |
| Rollback Cost | <5µs | >10µs | 5-10µs |
| Budget Utilization | 65-85% | <40% or >95% | 40-95% |

---

## Advanced Tuning

### Density Threshold Optimization

The storage profiler suggests an optimal density threshold. To further tune:

```rust
// If sparse is underutilized, lower threshold
pub const DENSITY_THRESHOLD: f32 = 0.20; // More aggressive sparse use

// If dense misses opportunities, raise threshold
pub const DENSITY_THRESHOLD: f32 = 0.35; // More aggressive dense use
```

### Ledger Batching Strategy

```rust
// Conservative batching
pub const BATCH_SIZE: usize = 64;        // Small batches, low latency
pub const ASYNC_ENABLED: bool = false;

// Aggressive batching
pub const BATCH_SIZE: usize = 512;       // Large batches, high throughput
pub const ASYNC_ENABLED: bool = true;    // Async writes
```

### Graph Cache Sizing

Dynamic cache sizing based on working set:

```rust
let working_set_nodes = graph.len();
let estimated_cache_mb = (working_set_nodes * 64) / 1_000_000;
let cache_mb = estimated_cache_mb.clamp(256, 2048);
```

### Mutation Budget Allocation

Adaptive budget based on fitness improvement rate:

```rust
if fitness_improvement_rate > 0.1 {
    budget = 15000;  // Increase for rapid convergence
} else if fitness_improvement_rate < 0.01 {
    budget = 5000;   // Decrease if not improving
}
```

---

## Monitoring in Production

### Key Metrics to Track

1. **Storage**: Monitor actual density distribution vs threshold
2. **Ledger**: Track batch operation frequency and verify latency
3. **Graph**: Measure real cache hit ratio with performance counters
4. **Mutations**: Log P99/P999 latencies for budget adequacy

### Alerting Thresholds

- SIMD utilization drops below 85% → investigate access patterns
- Ledger overhead exceeds 6% → review batch configuration
- Graph cache hit ratio below 75% → consider cache expansion
- Mutation P99 latency >1.5ms → reduce budget or parallelize

---

## Example: Complete Optimization Cycle

### Initial Run

```bash
$ cargo run --release --bin optimization_profiler

[Storage] Profiling sparse vs dense tradeoffs...
[Ledger] Profiling write overhead, batch ops, query impact...
[Graph] Profiling topo-sort, cache efficiency, parallel traversal...
[Mutation] Profiling budget allocation, rollback, fitness cache...

Storage:
  SIMD utilization: 87.1%
  Optimal density threshold: 0.28

Ledger:
  Total overhead: 4.2%

Mutations:
  P99 latency: 1456 ns

RECOMMENDATIONS:
1. STORAGE: SIMD utilization at 87.1% (target >90%). Consider batch operations.
2. MUTATION: P99 latency 1456 ns (target <1ms). Optimize budget allocation.
```

### Apply Tuning

Edit `tuning_parameters.rs`:
- Set `batch_size: 256`
- Set `mutation_budget_per_cycle: 12000`

### Second Run

```bash
$ cargo run --release --bin optimization_profiler

Storage:
  SIMD utilization: 91.3% ✓
  
Ledger:
  Total overhead: 3.8% ✓

Mutations:
  P99 latency: 832 ns ✓
```

✓ All targets achieved!

---

## Troubleshooting

### SIMD Utilization Low (<85%)

**Cause**: Inefficient use of bit-sliced operations
- Check: Dense matrix shapes (should be multiples of 64)
- Fix: Pad tensors or use block operations
- Check: Batch operation opportunities

### Ledger Overhead High (>10%)

**Cause**: Insufficient batching or verification overhead
- Check: Batch size configuration
- Fix: Increase batch size to 256-512
- Check: Enable async writes
- Fix: Set `async_enabled: true`

### Graph Cache Miss Rate High (>30%)

**Cause**: Working set exceeds cache
- Check: Node count vs cache size
- Fix: Increase `cache_size_mb` by 2x
- Check: Access pattern locality
- Fix: Consider level-order or block-order traversal

### Mutation Latency Spikes (P999 >5ms)

**Cause**: Budget exhaustion causing serialization
- Check: Budget utilization
- Fix: Increase `mutation_budget_per_cycle`
- Check: Rollback frequency
- Fix: Implement lazy rollback or shadowing

---

## References

- `src/ntg/storage/` - Storage implementations
- `src/ntg/ledger/` - Ledger and chain modules
- `src/ntg/graph/` - Graph data structures
- `src/ntg/mutation/` - Mutation and adaptive evolution
- `src/ntg/runtime.rs` - Runtime dispatch and SIMD
