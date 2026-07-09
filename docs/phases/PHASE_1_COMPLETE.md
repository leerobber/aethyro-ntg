# Phase 1 COMPLETE Certificate

**Sign-off date:** 2026-07-09  
**Phase N+1 may begin:** **YES** (Phase 2) — *only after this cert was
filled; Phase 2 work already present in tree is certified separately.*

## Scope

Ternary tensor core: scalar reference, packing, SIMD/TOBL, FFI,
observability, dense/sparse bit-sliced storage, density measurement.

## Deliverables map

| Responsibility | Path |
|----------------|------|
| Scalar golden | `ntg/ternary.rs` |
| Legacy 4/u8 pack | `ntg/packed.rs` |
| TOBL 32/u64 pack | `ntg/storage/packed_ternary.rs` |
| Dense dual-stream | `ntg/storage/bit_sliced_ternary.rs` |
| Sparse COO | `ntg/storage/sparse_bit_sliced_ternary.rs` |
| TOBL dispatch | `ntg/storage/tobl_kernel.rs` |
| SIMD | `ntg/simd/*` |
| FFI | `ntg/ffi/*` |
| Density bench | `src/bin/density_bench.rs` |
| Canonical types ADR | `docs/architecture/0005-canonical-ternary-storage.md` |

## Test proof

```bash
cd kernel && cargo test
# 192+ unit + integration suites (see STATUS)
cargo run --release --bin density_bench
```

Bit-identity: SIMD/FFI tests vs scalar. Density bench: sums match across
scalar / bit-sliced / sparse.

## Measurements

See EXPERIMENTS.md 2026-07-09 density micro-bench (~12× bit-sliced,
sparse ~20× at 1% density).

## Explicit re-scopes (documented, not silent)

| Item | Decision |
|------|----------|
| Full AVX-512 multi-block intrinsic kernels | **Phase 5** — host detect exists; portable `count_ones` is production path until measured win on real task |
| Full NEON intrinsic matmul | **Phase 5** — aarch64 path exists with bit-identical scalar-chunk structure; full `vmull` wiring deferred to ARM CI measurement |

## Deep dive

**Highest leverage from Phase 1:** density-aware dual-path compute
(bit-sliced vs sparse). Next advance is not more packings — it is
**using** these tensors on a real graph task (Phase 4), after Phase 2–3
certificates.

## Sign-off

**COMPLETE.** Phase 2 may begin under the gate protocol.
