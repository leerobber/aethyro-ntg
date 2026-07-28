# Phase 7.5: Codebase Hygiene & Alignment Audit

**Status:** In Progress (Phases 7.5.1, 7.5.2 complete; 7.5.3–7.5.7 started)

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

### 7.5.3: Dead Code & Consolidation
**Goal:** Remove unused modules, functions, and types; consolidate duplicated logic.

#### Audit Findings
- **Candidates for removal:**
  - `genomic/report_gen.rs` — superseded by observer patterns in vitascale
  - `genomic/extended_validation.rs` — functionality moved to quality_control
  - Unused trait bounds in `genome::GenomeDelta`
  - Deprecated `genomic/selection_loop.rs` (functionality in EvolutionSim)

- **Consolidation opportunities:**
  - `LdMatrix` + `LdComputer` overlap — single responsibility opportunity
  - `KairosState` vs `LifeCourse` — unify lifecycle tracking
  - Multiple `Brain` implementations — abstract to single interface

#### Implementation Plan
1. Identify all unused exports in public API
2. Mark deprecated items (keep for 1 release cycle)
3. Consolidate overlapping modules
4. Run `cargo check --all-targets` to verify no breakage

### 7.5.4: Module Interface & API Cleanup
**Goal:** Simplify public API, reduce surface area, improve ergonomics.

#### Audit Findings
- **Overly broad re-exports:**
  - `genomic::*` re-exports 40+ types at top level
  - Should namespace: `genomic::ld::`, `genomic::variants::`, `genomic::evolution::`
  
- **Inconsistent naming:**
  - `LdComputer` vs `HaplotypeBlockComparator` (inconsistent -er/-or)
  - `ChainLog` vs `ChainEntry` vs `ChainLog::Entry` (unclear hierarchy)

- **Missing trait abstractions:**
  - No `Source` trait for VCF/CSV input (hard to mock)
  - No `Sink` trait for output (hard to extend)
  - Brain implementations not unified under trait

#### Implementation Plan
1. Introduce traits for file I/O (Source/Sink)
2. Create hierarchical module namespacing
3. Standardize naming conventions
4. Document breaking changes in CHANGELOG

### 7.5.5: Test Coverage & Robustness
**Goal:** 95%+ line coverage, property-based testing, edge case handling.

#### Audit Findings
- **Coverage gaps:**
  - Error paths in `storage/` (sparse encoding fallback)
  - `accel/` hardware detection under non-AVX2 systems
  - `ntg/mutation/` rollback edge cases
  
- **Missing property tests:**
  - `ld_compute.rs`: Round-trip validation (r² computation idempotency)
  - `genome::recombination`: Distribution properties
  - `graph::{add_node, remove_node}`: Invariant preservation

#### Implementation Plan
1. Add `proptest` crate for property-based testing
2. Increase coverage to 95% line coverage
3. Add edge case tests for error conditions
4. Stress-test concurrent graph operations

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
| Test count | 493 | 550+ | In progress |
| Coverage | ~85% | 95%+ | Auditing |
| Clippy warnings | 0 | 0 | ✅ |
| Documented modules | 45 | 60 | In progress |
| Dead code refs | TBD | 0 | Auditing |
| Public API items | 85+ | <50 (namespaced) | Planning |
| Security audit | None | Complete | Planning |

---

## Timeline

```
Phase 7.5.1    ━━━━━━━━━━━━━  Complete (2026-07-28)
Phase 7.5.2    ━━━━━━━━━━━━━  Complete (2026-07-28)
Phase 7.5.3    ━━━━━━━━━━━    In progress (deadcode removal)
Phase 7.5.4    ━━━━━━━━━━━    In progress (API cleanup)
Phase 7.5.5    ━━━━━━━━━━━    Planned (coverage audit)
Phase 7.5.6    ━━━━━━━━━━━    Planned (performance)
Phase 7.5.7    ━━━━━━━━━━━    Planned (security audit)
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
