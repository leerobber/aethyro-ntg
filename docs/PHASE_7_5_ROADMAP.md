# Phase 7.5: Codebase Hygiene & Alignment Audit

**Status:** In Progress (Phases 7.5.1–7.5.5 complete; 7.5.6–7.5.7 remain)

---

## Completed

### 7.5.1: Documentation Refresh & README Updates ✅
- Updated root README.md with Phase F status (493 tests, clippy clean)
- Added Hostframe deployment documentation
- Updated KAIROS/VITASCALE feature descriptions
- Created phase-specific documentation links

### 7.5.2: Clippy Warnings Elimination ✅
- Fixed all `-D warnings` violations (6 issues)
- Removed unused fields and imports
- Added missing Default derives
- Zero compiler warnings in release builds

---

## In Progress

### 7.5.3: Dead Code & Consolidation ✅
**Goal:** Remove unused modules, functions, and types; consolidate duplicated logic.

#### Completed Actions
- ✅ Removed `genomic/report_gen.rs` (1,600+ lines, unused since vitascale telemetry)
- ✅ Removed `genomic/extended_validation.rs` (1,000+ lines, functionality in quality_control)
- ✅ Removed demo binaries: `phase_e_extended_validation`, `domain_disease_complete`
- ✅ Marked `selection_loop.rs` deprecated (will remove after tests migrate to EvolutionSim)
- ✅ Verified no breakage: 524 tests passing (after cleanup)

#### Result
Reduced codebase size by ~2.8K lines while maintaining all core functionality.

### 7.5.4: Module Interface & API Cleanup ✅ (part 1)
**Goal:** Simplify public API, reduce surface area, improve ergonomics.

#### Completed Actions
- ✅ Created `io_traits` module with pluggable `Source`/`Sink` traits
  - Enables VCF/CSV/database abstraction without hard-coding formats
  - Supports mocking for testing, custom implementations
- ✅ Created unified `Brain` trait with `StructureMeasurement` and `BrainDescription`
  - Enables swappable ChromosomeBrain, SovereignBrain implementations
  - Improves composability and test doubles
- ✅ Added module documentation clarifying hierarchy (io_traits, input, analysis, simulation, brain)
- ✅ Updated README.md with new API info and test count (493→526 tests)

#### Remaining (part 2)
- Hierarchical namespacing: `genomic::ld::`, `genomic::analysis::`, etc.
- Naming standardization across the codebase
- Consolidate LdMatrix + LdComputer single responsibility

#### Result
Introduced 3 new trait abstractions, improved API ergonomics, 526 tests passing.

### 7.5.5: Test Coverage & Robustness ✅ (part 1)
**Goal:** 95%+ line coverage, property-based testing, edge case handling.

#### Completed Actions
- ✅ Added `proptest` 1.4 as dev-dependency
- ✅ Created property-based test suite: `tests/prop_ld_compute.rs`
  - Idempotency: LD computation produces identical results (deterministic)
  - Bounds: All r² scores in valid range (0.0, 1.0]
  - Threshold filtering: Correctly filters results by threshold
  - Deterministic tests: Fixed input/output validation
- ✅ Tests run 100+ random trials each (proptest default)
- ✅ 530 tests passing (+4 new property-based tests)

#### Remaining (part 2)
- Error paths in `storage/` (sparse encoding fallback)
- `accel/` hardware detection under non-AVX2
- `ntg/mutation/` rollback edge cases
- Graph invariant preservation tests
- Concurrent operation stress tests

#### Result
Established property-based testing foundation, validated LD computation stability, 530 tests passing.

### 7.5.6: Performance Profiling & Optimization
**Goal:** Profile hot paths, reduce allocations, optimize memory layout.

#### Known Bottlenecks
- `LdMatrix` computation: O(n²) algorithm is unavoidable but could be parallelized
- `VcfParser`: Memory allocation per record (could use arena allocator)
- `GraphNode` traversal: Cache-unfriendly HashMap lookups
- `Ternary` matmul: SIMD dispatch adds 5-10% overhead in selection path

#### Implementation Plan
1. Benchmark with `cargo bench --release`
2. Profile with `perf` / Flamegraph
3. Implement parallel LD computation (rayon)
4. Consider arena allocator for VCF parsing
5. Consider FxHashMap for graph traversal (better cache locality)

### 7.5.7: Security & Dependency Audit
**Goal:** Minimal, vetted dependencies; no CVEs; secure defaults.

#### Audit Findings
- **Dependency inventory:**
  - Core: 15 dependencies (tokio, serde, ndarray, etc.)
  - Optional: google-bigquery1, goauth (new, for Phase F)
  - Total: ~85 transitive deps

- **Security considerations:**
  - `ndarray`: No known CVEs, actively maintained
  - `tokio`: Production-grade async runtime, regular security updates
  - `google-bigquery1`: New dependency (Phase F), audit required
  - No cryptographic dependencies (ChainLog uses SHA-256 built-in)

- **Risk mitigations:**
  - No use of unsafe code outside FFI + SIMD boundaries
  - No arbitrary deserialization (JSON inputs validated)
  - No dynamic code loading (no script engines)

#### Implementation Plan
1. Run `cargo audit` to check for known CVEs
2. Audit google-cloud dependencies (Phase F)
3. Document unsafe justifications in code comments
4. Set up SBOM (software bill of materials) generation
5. Create SECURITY.md with vulnerability disclosure process

---

## Metrics (Target)

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Test count | 530 | 550+ | On track |
| Coverage | ~87% (est) | 95%+ | In progress |
| Clippy warnings | 0 | 0 | ✅ |
| Dead code removed | 2.8K lines | ~5K | ✅ Partial |
| Documented modules | 50+ | 60+ | ✅ |
| Public API items | 75+ | <60 (namespaced) | In progress |
| Trait abstractions | 3 new | 5+ | In progress |
| Security audit | None | Complete | Planned |

---

## Timeline

```
Phase 7.5.1    ━━━━━━━━━━━━━  Complete (2026-07-28)
Phase 7.5.2    ━━━━━━━━━━━━━  Complete (2026-07-28)
Phase 7.5.3    ━━━━━━━━━━━━━  Complete (2026-07-28) — dead code removal
Phase 7.5.4    ━━━━━━━━━━━━━  Complete (part 1, 2026-07-28) — trait abstractions
Phase 7.5.5    ━━━━━━━━━━━━━  Complete (part 1, 2026-07-28) — property-based tests
Phase 7.5.6    ━━━━━━━━━━     In progress (performance profiling)
Phase 7.5.7    ━━━━━━━━      Planned (security audit)
```

---

## Dependencies

- Phase 7.5 depends on Phase F completion ✅
- Phase 7.5.3–4 can proceed in parallel
- Phase 7.5.5–7 depend on 7.3–4 completion

---

## Rollforward Strategy

Each sub-phase is merged independently (no large batch commits):

1. Complete 7.5.3 (deadcode), create PR, test, merge to main
2. Complete 7.5.4 (API), create PR, test, merge to main
3. ...continue for 7.5.5–7

This keeps `main` continuously deployable with incremental improvements.
