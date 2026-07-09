# TOBL FFI Reference Guide

C interface for PackedTernary ternary dot-product operations.

---

## Quick Start

```c
#include <stdint.h>

// 1. Create two PackedTernary buffers (10 elements each)
struct ToблHandle* a = ntg_tobl_new(10);
struct ToблHandle* b = ntg_tobl_new(10);

// 2. Set ternary values (-1, 0, or 1)
ntg_tobl_set(a, 0, 1);    // a[0] = 1
ntg_tobl_set(a, 1, -1);   // a[1] = -1
ntg_tobl_set(b, 0, 1);    // b[0] = 1
ntg_tobl_set(b, 1, -1);   // b[1] = -1

// 3. Compute dot-product with cycle tracking
int64_t result = 0;
uint64_t cycles = 0;
int32_t err = ntg_tobl_dot(a, b, &result, &cycles);

if (err == 0) {
    printf("Result: %ld, Cycles: %lu\n", result, cycles);
    // Result: 2 (1*1 + (-1)*(-1) = 2)
} else {
    fprintf(stderr, "Error: %d\n", err);
}

// 4. Cleanup
ntg_tobl_drop(a);
ntg_tobl_drop(b);
```

---

## API Reference

### Creation & Destruction

```c
/// Create new PackedTernary buffer
///
/// len: Number of ternary elements (0-4GB)
/// returns: Opaque handle (non-null on success)
ToблHandle* ntg_tobl_new(uint32_t len);

/// Destroy PackedTernary buffer
/// handle: Opaque handle from ntg_tobl_new (safe to call on NULL)
void ntg_tobl_drop(ToблHandle* handle);
```

### Element Access

```c
/// Set single ternary value
///
/// handle: Opaque handle
/// idx: Element index (0 <= idx < len)
/// val: Ternary value (-1, 0, or 1)
/// returns: 0 on success, -1 on error (invalid idx or null handle)
int32_t ntg_tobl_set(ToблHandle* handle, uint32_t idx, int8_t val);

/// Get single ternary value
///
/// handle: Opaque handle
/// idx: Element index
/// returns: Ternary value (-1, 0, 1) or 0 on error
int8_t ntg_tobl_get(const ToблHandle* handle, uint32_t idx);
```

### TOBL Operations

```c
/// Ternary dot-product with cycle tracking
///
/// a: First buffer (non-null)
/// b: Second buffer (non-null, same length as a)
/// result: Output pointer for dot-product sum (int64_t)
/// cycles: Output pointer for wall-clock cycles (uint64_t)
/// returns: 0 on success, -1 on error (mismatch/null), -2 on internal error
int32_t ntg_tobl_dot(
    const ToблHandle* a,
    const ToблHandle* b,
    int64_t* result,
    uint64_t* cycles
);
```

### Metrics & Observability

```c
/// Compute non-zero density metric
///
/// Density = (count of non-zero elements) / (total elements)
/// Used for structural evolution heuristics
///
/// handle: Opaque handle
/// returns: Density [0.0, 1.0] (0.0 on error)
float ntg_tobl_density(ToблHandle* handle);

/// Get global TOBL operation counter
///
/// returns: Total ntg_tobl_dot operations executed
uint64_t ntg_tobl_op_count();
```

---

## Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| 0 | Success | — |
| -1 | Invalid handle or index | Check pointers, bounds |
| -2 | Internal error | Retry or report |

All error codes are negative; check `if (result < 0)` to detect errors.

---

## Usage Patterns

### Pattern 1: Batch Dot-Product (Orchestrator Loop)

```c
// Phase 3: Reflexive Fitness Evaluator calls TOBL
for (int i = 0; i < num_mutations; i++) {
    int64_t dot = 0;
    uint64_t cycles = 0;
    
    if (ntg_tobl_dot(proposal[i], baseline, &dot, &cycles) == 0) {
        // Log to ledger: dot, cycles, generation
        log_mutation_result(proposal[i], dot, cycles);
    }
}
```

### Pattern 2: Structural Evolution (Density Gating)

```c
// Phase 4: KernelSentinel checks if mutation is degenerate
float density = ntg_tobl_density(mutated);
if (density < MIN_DENSITY_THRESHOLD) {
    // Reject: graph lost information
    ntg_tobl_drop(mutated);
    continue;
}
// Accept: within structural bounds
```

### Pattern 3: Performance Tracking

```c
// Measure and record TOBL speed over time
for (int run = 0; run < 100; run++) {
    int64_t result;
    uint64_t cycles;
    ntg_tobl_dot(a, b, &result, &cycles);
    
    // Track cycle trend for adaptive kernel selection
    log_performance(run, cycles);
}
```

---

## Performance Characteristics

| Operation | Latency | Throughput |
|-----------|---------|-----------|
| ntg_tobl_new(N) | O(1) | — |
| ntg_tobl_set/get | ~10ns | 100M/sec |
| ntg_tobl_dot (scalar) | ~2µs (100), ~20µs (1000) | 50k/sec (1000-element) |
| ntg_tobl_dot (AVX2) | ~1.2µs (100), ~12µs (1000) | 80k/sec (1000-element) |
| ntg_tobl_density | ~100ns | 10M/sec |

**Actual latency depends on CPU, cache state, and kernel selection (detected at runtime).**

---

## Safety & Invariants

### FFI Safety Contract
1. **Null Safety**: All functions check for null pointers
2. **Bounds Safety**: Index checks prevent out-of-bounds access
3. **Lifetime**: Caller owns handle after ntg_tobl_new; must call ntg_tobl_drop
4. **Thread Safety**: Each handle is independent; concurrent calls safe

### Value Domain
- **ternary values**: Only -1, 0, 1 are valid
- Invalid values (e.g., 2, -2) are silently clamped to 0

### Determinism
- Results are deterministic: same input → same output
- Cycle count may vary (depends on CPU state)
- Generation counter tracks mutations for replay

---

## Integration with Ledger

Each TOBL operation can be logged to Phase 3's TamperEvidentLedger:

```json
{
  "operation": "tobl_dot",
  "result": 42,
  "cycles": 15234,
  "generation": 5,
  "timestamp_ns": 1234567890000,
  "simd_path": "AVX2"
}
```

Ledger verifies:
- ✅ Generation counter consistency (no gaps)
- ✅ Timestamp ordering (monotonic)
- ✅ Result reproducibility (same inputs → same output)

---

## Building & Linking

### Rust (Cargo)
```toml
[package]
name = "my_orchestrator"
[dependencies]
ntg_kernel = { path = "../kernel" }
```

### C/C++ (Manual)
```bash
# Build Rust library
cd kernel && cargo build --release

# Link in C code
gcc -o orchestrator orchestrator.c \
    -L kernel/target/release \
    -lntg_kernel
```

### Header File
```c
#ifndef NTG_TOBL_FFI_H
#define NTG_TOBL_FFI_H

#include <stdint.h>

// Opaque handle
typedef struct ToблHandle ToблHandle;

// API
ToблHandle* ntg_tobl_new(uint32_t len);
void ntg_tobl_drop(ToблHandle* handle);
int32_t ntg_tobl_set(ToблHandle* h, uint32_t idx, int8_t val);
int8_t ntg_tobl_get(const ToблHandle* h, uint32_t idx);
int32_t ntg_tobl_dot(const ToблHandle* a, const ToблHandle* b, int64_t* result, uint64_t* cycles);
float ntg_tobl_density(ToблHandle* handle);
uint64_t ntg_tobl_op_count();

#endif
```

---

## Troubleshooting

| Issue | Cause | Fix |
|-------|-------|-----|
| `ntg_tobl_set` returns -1 | Index out of bounds | Check `idx < length` |
| `ntg_tobl_dot` returns -2 | Internal error | Verify both handles non-null and same length |
| Segfault | Null handle passed | Check `handle != NULL` |
| Wrong result | Value domain violation | Ensure only -1, 0, 1 set |
| Slow performance | Scalar path selected | Check CPU supports AVX2/NEON |

---

## Examples

See `kernel/examples/` for full C integration examples:
- `tobl_usage.c` — complete lifecycle example
- `tobl_ledger_integration.c` — ledger logging pattern
- `tobl_performance_bench.c` — performance measurement

---

## References

- **PHASE1_2_3_STORAGE_COMPLETE.md** — architecture & design decisions
- **FFI_INTEGRATION.md** — general FFI guidelines
- **kernel/src/ntg/ffi/tobl_ffi.rs** — implementation source
