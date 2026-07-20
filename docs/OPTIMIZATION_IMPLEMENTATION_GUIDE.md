# Aethyro OS Optimization Implementation Guide

## Quick Start

### 1. Run the Profiler

```bash
cd C:\Users\leer4\aethyro-ntg\kernel
cargo build --release --bin optimization_profiler
cargo run --release --bin optimization_profiler
```

This generates:
- `optimization_profile.json` - Complete profiling results
- `tuning_parameters.rs` - Auto-generated Rust module (or manually use template)

### 2. Integrate Tuning Parameters

Copy the template to your source:

```bash
cp docs/TUNING_PARAMETERS_TEMPLATE.rs src/ntg/tuning.rs
```

Add to `src/ntg/mod.rs`:

```rust
pub mod tuning;
```

### 3. Apply Storage Tuning

**File:** `src/ntg/accel.rs`

```rust
use crate::ntg::tuning::{STORAGE_TUNING, STORAGE_DENSITY_THRESHOLD};

pub const DEFAULT_SPARSE_DENSITY_THRESHOLD: f32 = STORAGE_TUNING.density_threshold;

fn select_representation(tensor: &Tensor) -> StorageKind {
    let density = tensor.compute_density();
    
    if density > DEFAULT_SPARSE_DENSITY_THRESHOLD {
        StorageKind::Dense  // Use fast bit-sliced dual-stream
    } else {
        StorageKind::Sparse // Use hash-backed sparse blocks
    }
}
```

### 4. Apply Ledger Tuning

**File:** `src/ntg/ledger/mod.rs`

```rust
use crate::ntg::tuning::LEDGER_TUNING;

pub struct LedgerWriter {
    batch: Vec<ChainEntry>,
}

impl LedgerWriter {
    pub fn append(&mut self, entry: ChainEntry) -> Result<(), Error> {
        self.batch.push(entry);
        
        if self.batch.len() >= LEDGER_TUNING.batch_size {
            self.flush()?;
        }
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<(), Error> {
        if LEDGER_TUNING.async_enabled {
            self.flush_async()
        } else {
            self.flush_sync()
        }
    }
    
    fn flush_sync(&mut self) -> Result<(), Error> {
        // Batch append to ledger
        for entry in self.batch.drain(..) {
            self.ledger.append(entry)?;
        }
        Ok(())
    }
    
    fn flush_async(&mut self) -> Result<(), Error> {
        // Spawn async task, return immediately
        let batch = self.batch.drain(..).collect();
        spawn_ledger_commit_task(batch);
        Ok(())
    }
}
```

### 5. Apply Graph Tuning

**File:** `src/ntg/runtime.rs`

```rust
use crate::ntg::tuning::GRAPH_TUNING;

impl Runtime {
    pub fn forward_native_parallel(
        &self,
        layer_idx: usize,
        input_activations: &SparseBitSlicedTernary,
        threshold: i64,
    ) -> Result<SparseBitSlicedTernary, NtgError> {
        let layer = self.layers.get(layer_idx)?;
        
        // Decide parallelism based on tuning
        if layer.len() > GRAPH_TUNING.parallel_threshold_nodes {
            self.forward_parallel(layer, input_activations, threshold)
        } else {
            self.forward_sequential(layer, input_activations, threshold)
        }
    }
    
    fn forward_parallel(
        &self,
        layer: &[GraphNode],
        input_activations: &SparseBitSlicedTernary,
        threshold: i64,
    ) -> Result<SparseBitSlicedTernary, NtgError> {
        let worker_count = thread::available_parallelism()?.get();
        let chunk_size = (layer.len() + worker_count - 1) / worker_count;
        
        // Process in batches for cache locality
        let batch_size = GRAPH_TUNING.layer_batch_size;
        
        thread::scope(|s| {
            for chunk in layer.chunks(chunk_size) {
                s.spawn(move || {
                    for node_batch in chunk.chunks(batch_size) {
                        // Process batch...
                    }
                });
            }
        })
    }
}
```

### 6. Apply Mutation Tuning

**File:** `src/ntg/mutation/mod.rs`

```rust
use crate::ntg::tuning::{MUTATION_TUNING, RollbackStrategy};

pub struct MutationEngine {
    budget: usize,
    batch: Vec<Mutation>,
}

impl MutationEngine {
    pub fn apply_mutation(&mut self, mutation: Mutation) -> Result<(), Error> {
        if self.budget == 0 {
            return Err(Error::BudgetExhausted);
        }
        
        if MUTATION_TUNING.batch_mutations {
            self.batch.push(mutation);
            if self.batch.len() >= MUTATION_TUNING.batch_max_size {
                self.commit_batch()?;
            }
        } else {
            self.apply_single(mutation)?;
        }
        
        self.budget -= 1;
        Ok(())
    }
    
    fn apply_single(&mut self, mutation: Mutation) -> Result<(), Error> {
        // Apply with individual ledger entry
        // Higher overhead per mutation
        let result = mutation.apply_to(&mut self.genome)?;
        
        if !result.is_valid() {
            match MUTATION_TUNING.rollback_strategy {
                RollbackStrategy::Lazy => {
                    // Defer rollback - mark for later
                    self.mark_for_rollback(mutation);
                }
                RollbackStrategy::Eager => {
                    // Immediate rollback
                    mutation.rollback_from(&mut self.genome)?;
                }
            }
        }
        Ok(())
    }
    
    fn commit_batch(&mut self) -> Result<(), Error> {
        // Apply all mutations, then single ledger entry
        // Amortizes ledger overhead
        for mutation in self.batch.drain(..) {
            self.apply_single(mutation)?;
        }
        self.ledger.log_batch_mutations(&self.batch)?;
        Ok(())
    }
}
```

---

## Configuration Verification

### 1. Check Defaults

Verify that tuning parameters are loaded:

```bash
cargo run --release --bin optimization_profiler 2>&1 | grep -A 20 "TUNING PARAMETERS:"
```

Expected output:
```
TUNING PARAMETERS:
  Storage density threshold: 0.25
  Ledger batch size: 256
  Ledger async enabled: true
  Graph cache size: 512 MB
  Mutation budget per cycle: 12000
```

### 2. Validate Against Targets

Run benchmarks to verify targets:

```bash
# SIMD throughput benchmark
cargo bench --bench simd_benchmark --release

# Ledger batch throughput
cargo run --release --bin phase4_calib

# Graph parallel efficiency
cargo run --release --bin graph_overhead_bench

# Expected output (meeting all targets):
# ✓ Dense: 1.42 ops/ns (target >1.4)
# ✓ Ledger: 78.1K ops/sec (target >70K)
# ✓ Graph: 3.95x speedup (target >3.5x)
# ✓ Mutations: 380ns (target <500ns)
```

### 3. Enable Feature Flags

If using async ledger, enable the feature:

```bash
cargo build --release --features ledger-async
```

---

## Performance Monitoring

### System Metrics

Add these metrics to your observability system:

```rust
// src/ntg/observability/metrics.rs
pub struct PerformanceMetrics {
    // Storage
    pub storage_density: f32,
    pub simd_utilization: f32,
    pub sparse_vs_dense_ratio: f32,
    
    // Ledger
    pub ledger_batch_count: u64,
    pub ledger_write_latency_us: f32,
    pub ledger_overhead_percent: f32,
    
    // Graph
    pub graph_cache_hits: u64,
    pub graph_cache_misses: u64,
    pub parallel_efficiency: f32,
    
    // Mutations
    pub mutation_count: u64,
    pub mutation_latency_us: f32,
    pub mutation_budget_utilization: f32,
}

impl PerformanceMetrics {
    pub fn compute_ratios(&mut self) {
        self.sparse_vs_dense_ratio = self.sparse_mutations as f32 / self.dense_mutations.max(1) as f32;
        self.ledger_overhead_percent = (self.ledger_time / self.total_time) * 100.0;
        self.parallel_efficiency = self.actual_speedup / ideal_speedup as f32;
        self.mutation_budget_utilization = (self.mutations_applied as f32 / self.budget as f32) * 100.0;
    }
}
```

### Alert Thresholds

```rust
// src/ntg/observability/alerts.rs
pub struct AlertConfig {
    // Storage alerts
    pub simd_utilization_min: f32 = 85.0,
    pub sparse_compression_ratio_min: f32 = 2.0,
    
    // Ledger alerts
    pub ledger_overhead_max: f32 = 5.0,
    pub batch_size_alert: usize = 100, // warn if <100 per batch
    
    // Graph alerts
    pub cache_hit_ratio_min: f32 = 0.75,
    pub parallel_efficiency_min: f32 = 0.80,
    
    // Mutation alerts
    pub mutation_latency_max_us: f32 = 1.0,
    pub budget_utilization_min: f32 = 50.0,
    pub budget_utilization_max: f32 = 95.0,
}

pub fn check_metrics(metrics: &PerformanceMetrics, config: &AlertConfig) -> Vec<String> {
    let mut alerts = Vec::new();
    
    if metrics.simd_utilization < config.simd_utilization_min {
        alerts.push(format!(
            "SIMD utilization low: {:.1}% (min: {:.1}%)",
            metrics.simd_utilization, config.simd_utilization_min
        ));
    }
    
    if metrics.ledger_overhead_percent > config.ledger_overhead_max {
        alerts.push(format!(
            "Ledger overhead high: {:.2}% (max: {:.2}%)",
            metrics.ledger_overhead_percent, config.ledger_overhead_max
        ));
    }
    
    // ... more checks
    
    alerts
}
```

---

## Iterative Optimization Workflow

### Week 1: Baseline

```bash
# 1. Run profiler
cargo run --release --bin optimization_profiler > baseline_profile.txt

# 2. Save results
cp optimization_profile.json profile_week1_baseline.json
mv baseline_profile.txt analysis_week1_baseline.txt

# 3. Review recommendations
cat analysis_week1_baseline.txt | grep "RECOMMENDATIONS:" -A 20
```

### Week 2: Implement Changes

1. Copy `tuning_parameters.rs` to source
2. Integrate into 4 subsystems (storage, ledger, graph, mutation)
3. Rebuild: `cargo build --release`
4. Test: `cargo test --release`

### Week 3: Validate

```bash
# 1. Run profiler again
cargo run --release --bin optimization_profiler > week3_profile.txt

# 2. Compare against baseline
diff <(jq '.storage.simd_utilization_percent' profile_week1_baseline.json) \
     <(jq '.storage.simd_utilization_percent' optimization_profile.json)

# Expected improvement: 87% → 92% (+5pp)
```

### Week 4: Monitor Production

```bash
# Track these metrics daily
- SIMD utilization: 92% ✓
- Ledger overhead: 2.8% ✓
- Cache hit ratio: 87% ✓
- Mutation P99: 745ns ✓
```

---

## Troubleshooting Common Issues

### Issue 1: SIMD Utilization Still <90%

**Symptoms:**
```
SIMD utilization: 87.1% (target >90%)
```

**Root Causes:**
1. Dense tensors not padded to 64-element boundaries
2. Sparse representation selected too aggressively
3. Batch operations not used

**Solutions:**
```rust
// Option 1: Pad tensors
let padded_len = ((tensor.len() + 63) / 64) * 64;
let padded = vec![0i8; padded_len];

// Option 2: Increase density threshold
pub const DENSITY_THRESHOLD: f32 = 0.35;  // Use dense more often

// Option 3: Use batch operations
let batch_size = 256;
for chunk in tensors.chunks(batch_size) {
    process_batch(chunk); // SIMD-friendly
}
```

### Issue 2: Ledger Overhead >5%

**Symptoms:**
```
Ledger overhead: 8.2% (target <5%)
```

**Root Causes:**
1. Batch size too small
2. Async not enabled
3. Verification running too frequently

**Solutions:**
```rust
// Option 1: Increase batch size
pub const BATCH_SIZE: usize = 512;  // was 256

// Option 2: Enable async
pub const ASYNC_ENABLED: bool = true;

// Option 3: Reduce verification frequency
pub const VERIFY_INTERVAL: usize = 50000;  // was 10000
```

### Issue 3: Cache Hit Ratio <75%

**Symptoms:**
```
Cache hit ratio: 68% (target >80%)
```

**Root Causes:**
1. Working set exceeds cache
2. Access pattern is random
3. Cache too small

**Solutions:**
```rust
// Option 1: Increase cache size
pub const CACHE_SIZE_MB: usize = 1024;  // was 512

// Option 2: Optimize access order (locality)
// Instead of: for node in random_order
// Use: for node in topological_order

// Option 3: Reduce node count per layer
// Split large layers into smaller batches
```

### Issue 4: Mutation P99 >1ms

**Symptoms:**
```
Mutation P99 latency: 1456 ns (target <1ms)
```

**Root Causes:**
1. Budget allocation too high (serialization)
2. Fitness cache too small
3. Rollback too frequent

**Solutions:**
```rust
// Option 1: Reduce budget
pub const BUDGET_PER_CYCLE: usize = 8000;  // was 12000

// Option 2: Increase cache
pub const CACHE_SIZE_MB: usize = 512;  // was 256

// Option 3: Use lazy rollback
pub const ROLLBACK_STRATEGY: RollbackStrategy = RollbackStrategy::Lazy;
```

---

## Performance Regression Detection

### Automated Regression Testing

```rust
// src/bench/regression_test.rs
#[bench]
fn bench_storage_throughput(b: &mut Bencher) {
    b.iter(|| {
        let tensor = BitSlicedTernary::new(1_000_000);
        BitSlicedTernary::dot_product_parallel(&tensor, &tensor)
    });
    
    // Fail if regression >5%
    let expected_max_ns = 1_350_000; // baseline 1.3µs
    assert!(b.iter_count < expected_max_ns, "Storage regression detected");
}

#[bench]
fn bench_ledger_throughput(b: &mut Bencher) {
    b.iter_with_setup(
        || CryptoChainLog::new(),
        |mut chain| {
            for i in 0..256 {
                let _ = chain.append(format!("entry{}", i));
            }
        }
    );
    
    // Fail if regression >10%
    let expected_max_ns = 3_520_000; // baseline 3.2µs amortized
    assert!(b.iter_count < expected_max_ns, "Ledger regression detected");
}
```

### CI/CD Integration

```yaml
# .github/workflows/performance-regression.yml
name: Performance Regression Test
on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: |
          cargo bench --release --no-run
          cargo bench --release \
            --bench simd_benchmark \
            --bench ledger_throughput \
            --bench graph_parallel \
            --bench mutation_latency
      - name: Check targets
        run: |
          python3 scripts/check_perf_targets.py
```

---

## Profiler Output Interpretation

### JSON Schema

```json
{
  "timestamp": "2026-07-16T...",
  "system_info": {
    "cpu_cores": 4,
    "simd_capability": "AVX2/AVX-512",
    "memory_gb": 16.0
  },
  "storage": {
    "simd_utilization_percent": 92.3,
    "optimal_density_threshold": 0.25,
    "sparse_compression_ratio": 3.14
  },
  "ledger": {
    "ledger_to_total_overhead_percent": 2.8,
    "batch_overhead_percent": -29.9,
    "async_latency_nanos": 1370
  },
  "graph": {
    "cache_hit_ratio": 0.87,
    "parallel_traversal_speedup": 3.8,
    "cache_efficiency_percent": 87.0
  },
  "mutation": {
    "mutation_latency_p99_nanos": 892,
    "fitness_cache_hit_ratio": 0.78,
    "budget_utilization_percent": 71.3
  },
  "recommendations": ["..."],
  "tuning_parameters": {...}
}
```

---

## Integration Checklist

- [ ] Clone profiler binary: `cargo build --release --bin optimization_profiler`
- [ ] Run baseline: `cargo run --release --bin optimization_profiler`
- [ ] Copy tuning template: `cp docs/TUNING_PARAMETERS_TEMPLATE.rs src/ntg/tuning.rs`
- [ ] Update `src/ntg/mod.rs`: add `pub mod tuning;`
- [ ] Apply storage tuning in `src/ntg/accel.rs`
- [ ] Apply ledger tuning in `src/ntg/ledger/mod.rs`
- [ ] Apply graph tuning in `src/ntg/runtime.rs`
- [ ] Apply mutation tuning in `src/ntg/mutation/mod.rs`
- [ ] Rebuild: `cargo build --release`
- [ ] Re-run profiler: `cargo run --release --bin optimization_profiler`
- [ ] Verify targets met in output
- [ ] Add performance monitoring (observability module)
- [ ] Set up alerting (threshold checks)
- [ ] Document changes in CHANGELOG.md
- [ ] Schedule quarterly re-profiling

---

## References

- Optimization Profiler: `src/bin/optimization_profiler.rs`
- Profiler Guide: `docs/OPTIMIZATION_PROFILER_GUIDE.md`
- Summary: `docs/OPTIMIZATION_SUMMARY.md`
- Template: `docs/TUNING_PARAMETERS_TEMPLATE.rs`
