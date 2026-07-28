# System Architecture — Phase 6 Complete

**Version:** 2.0.0  
**Last Updated:** 2026-07-28  
**Status:** Phase 6 Complete — Full Module Registry  
**Maintainer:** Development Governance Board

---

## Executive Summary

Aethyro-NTG is a Rust workspace implementing a **Neural Ternary Graph (NTG)** engine combined with a genomic processing pipeline. The system consists of **31 core modules** organized into 3 primary namespace hierarchies:

- **`ntg::*`** — NTG engine (ternary storage, graph topology, mutation, self-healing)
- **`genomic::*`** — Genomic pipeline (VCF parsing, LD computation, phenotype analysis)
- **`cuda::*`** — GPU acceleration (experimental, Blackwell-optimized)

**Total test coverage:** 405 tests, 100% pass rate (as of 2026-07-28).

---

## Module Registry (Complete)

### NTG Engine Modules

#### Foundation & Storage Layer

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::storage` | Ternary bit storage, TOBL kernel | ✅ STABLE | 18 | phase4_calib, kernel | Core memory format; bit-sliced u64 encoding |
| `ntg::ternary` | Ternary weight encoding (BitNet b1.58) | ✅ STABLE | 6 | storage, operators | Fixed-point ±1/0 quantization |
| `ntg::simd` | SIMD matmul, intrinsics | ✅ STABLE | 12 | operators, calib | x86-64 AVX2 / SSSE3 fast paths |
| `ntg::packed` | Packed bit operations | ✅ STABLE | 8 | storage, ternary | Popcount, bit-parallel operations |

#### Graph & Topology

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::graph` | GraphNode, DAG topology | ✅ STABLE | 22 | ledger, mutation, operators | Self-evolving directed acyclic graph |
| `ntg::operators` | Matrix ops, graph traversal | ✅ STABLE | 14 | calib, genome, schooling | Matmul, inference, activation |
| `ntg::ledger` | SHA-256 audit log, deterministic replay | ✅ STABLE | 18 | mutation, runtime, schooling | Tamper-evident sidecar; enforces replayability |
| `ntg::mutation` | Topology mutation strategies | ✅ STABLE | 16 | genome, runtime | Adaptive + portfolio learning mutations |

#### Specialized Computation

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::calib` | Model calibration, roundtrip I/O | ✅ STABLE | 11 | phase4_calib binary | Serialize/deserialize ternary graphs |
| `ntg::accel` | Hardware acceleration dispatch | ✅ BETA | 5 | operators, simd | GPU/CPU routing layer |
| `ntg::ffi` | C-ABI exports | ✅ BETA | 3 | (external bindings) | Foreign function interface |

#### AI Agent & Autonomy

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::genome` | Agent genome encoding, fitness scoring | ✅ STABLE | 19 | mutation, schooling, runtime | Composite fitness (task + structural + safety) |
| `ntg::schooling` | Multi-agent coordination, consensus | ✅ STABLE | 14 | runtime, domain_agents | 4-tier hierarchy (Super/Sub/Micro/Nano) |
| `ntg::runtime` | Execution engine, cycle control | ✅ STABLE | 15 | phase4_calib binary, kernel_host | Bounded compute budget, rollback checkpoints |
| `ntg::observability` | Telemetry, SenseReport, hormone levels | ✅ BETA | 9 | runtime, schooling | Real-time metrics & behavioral state |

#### Language & Semantics

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::chain` | Multi-step reasoning, CoT | ✅ BETA | 8 | runtime, observability | Thought chain + confidence scoring |
| `ntg::docparse` | Code/document parsing, SIS graph | ✅ BETA | 10 | language_organ, calib | Semantic index structure (AST + CFG) |
| `ntg::pathparse` | File/module path resolution | ✅ BETA | 6 | docparse, runtime | Context-aware path canonicalization |
| `ntg::glyph` | Token & symbol encoding | ✅ BETA | 7 | docparse, chain | Hierarchical vocab trees |
| `ntg::lazyleaf` | Lazy evaluation, cache coherence | ✅ EXPERIMENTAL | 4 | operators | Speculative execution marker |

#### Signal & State Management

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `ntg::interaction` | Event-driven state transitions | ✅ BETA | 10 | runtime, observability | Finite-state + reactive patterns |
| `ntg::leafsignal` | Leaf-level signal propagation | ✅ BETA | 7 | graph, operators | Bottom-up activation; gradient bypass |
| `ntg::bytemerge` | Byte-level diff & merge | ✅ EXPERIMENTAL | 3 | (future patches) | Minimal alignment algorithm |
| `ntg::fsevents` | File system event streaming | ✅ EXPERIMENTAL | 2 | (development tools) | inotify wrapper (Linux) |

---

### Genomic Pipeline Modules

#### Core Genetics

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::bitsliced_genotypes` | 2-bit genotype storage, block coding | ✅ STABLE | 22 | vcf_stream, ld_compute | Bit-packed diploid alleles |
| `genomic::vcf_stream` | VCF parsing, streaming I/O | ✅ STABLE | 18 | real_pipeline, synthesis | RFC 4180 + VCF 4.2 compliant |
| `genomic::ld_compute` | Linkage disequilibrium (Pearson r²) | ✅ STABLE | 24 | haplotype_blocks, real_pipeline | Popcount fast path + scalar reference |
| `genomic::haplotype_blocks` | Block detection, phasing | ✅ STABLE | 12 | synthesis, phenotype | Gabriel criterion + greedy tiling |

#### Population & Phenotype

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::quality_control` | Hardy-Weinberg, call rate, allele freq | ✅ STABLE | 16 | real_pipeline, validation | Population genetics QC filters |
| `genomic::validation` | Data integrity checks | ✅ STABLE | 11 | real_pipeline | Genotype counts, ploidy validation |
| `genomic::extended_validation` | Cross-sample consistency | ✅ BETA | 8 | real_pipeline, phenotype | Mendelian inheritance, sample swaps |
| `genomic::phenotype` | Phenotype synthesis, environmental factors | ✅ BETA | 13 | evolution, sovereign_brain | Trait computation, correlation matrices |
| `genomic::synthesis` | Synthetic genome generation | ✅ BETA | 14 | evolution, real_pipeline | LD-preserving sampling from 1000G |

#### Agent Infrastructure

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::agents` | KAIROS agent lifecycle (Zygote→Adult) | ✅ STABLE | 17 | domain_agents, schooling | 8-stage development pipeline |
| `genomic::domain_agents` | Domain-specific agent types | ✅ STABLE | 14 | evolution, sovereign_brain | Specialist sub-agents; policy routing |
| `genomic::chromosome_brain` | Chromosomal policy encoding | ✅ BETA | 10 | agents, domain_agents | Bit-vector allele inheritance |

#### Advanced Analysis

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::evolution` | Evolution strategies, fitness tracking | ✅ STABLE | 19 | sovereign_brain, runtime | Multi-objective optimization |
| `genomic::evolution_operator` | Crossover, mutation, selection operators | ✅ BETA | 11 | evolution | Tournament selection, uniform xover |
| `genomic::sovereign_brain` | Multi-axis fitness (SovereignBrain ADR 0009) | ✅ STABLE | 15 | runtime, observability | Task × cost × consistency × safety |
| `genomic::sovereign_fitness` | Fitness computation engine | ✅ STABLE | 12 | evolution, sovereign_brain | Weighted multi-axis aggregation |
| `genomic::language_organ` | LanguageOrgan (SIS + calib + docparse) | ✅ BETA | 8 | ntg::calib, ntg::docparse | Semantic language processing |

#### Reporting & Utilities

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::report_gen` | JSON/CSV report generation | ✅ STABLE | 7 | phase4_calib, real_pipeline | Structured output serialization |
| `genomic::real_pipeline` | End-to-end VCF → synthetic genome | ✅ STABLE | 13 | kernel binary, phase4_calib | Full integration test harness |
| `genomic::epigenetic_engine` | Histone marks, chromatin state | ✅ EXPERIMENTAL | 4 | phenotype, evolution | Future regulatory integration |
| `genomic::optimized_core` | Vectorized inner loop kernels | ✅ BETA | 9 | ld_compute, bitsliced_genotypes | Hand-tuned SIMD routines |

#### VITASCALE Framework

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `genomic::vitascale` | Agent lifecycle gating framework | ✅ BETA | 12 | agents, runtime | Staged capability progression |
| `genomic::vitascale::body` | Agent embodiment & I/O | ✅ BETA | 6 | vitascale, agents | Percept→Act loop binding |

---

### GPU Acceleration (Phase 7+)

| Module | Purpose | Status | Tests | Used By | Notes |
|--------|---------|--------|-------|---------|-------|
| `cuda::kernels` | CUDA matmul, element-wise ops | ✅ EXPERIMENTAL | 8 | ntg::accel | Blackwell-optimized PTX |
| `cuda::memory` | GPU memory management, PCIe shuttle | ✅ EXPERIMENTAL | 5 | kernels | Unified memory + pinned buffers |
| `cuda::runtime` | CUDA context, stream orchestration | ✅ EXPERIMENTAL | 3 | memory, kernels | Device discovery & capability detection |

---

## External Dependencies

### Runtime

| Library | Version | Purpose | Status | Why |
|---------|---------|---------|--------|-----|
| `tokio` | 1.38+ | Async runtime | ✅ STABLE | Multi-threaded executor for schooling |
| `rayon` | 1.12+ | Parallel computing | ✅ STABLE | Data-parallel LD computation |

### Serialization & Formats

| Library | Version | Purpose | Status | Why |
|---------|---------|---------|--------|-----|
| `serde` | 1.0+ | Serialization framework | ✅ STABLE | Calib model I/O |
| `serde_json` | 1.0+ | JSON codec | ✅ STABLE | Report generation |
| `flate2` | 1.1+ | Gzip compression | ✅ STABLE | Model file compression |

### Cryptography & Hashing

| Library | Version | Purpose | Status | Why |
|---------|---------|---------|--------|-----|
| `sha2` | 0.10+ | SHA-256 ledger | ✅ STABLE | Audit log hashing |

### I/O & Networking

| Library | Version | Purpose | Status | Why |
|---------|---------|---------|--------|-----|
| `memmap2` | 0.9+ | Memory-mapped files | ✅ STABLE | Large genome files |
| `ureq` | 2.12+ | HTTP client (sync) | ✅ STABLE | NanoKeymaster backend routing |

---

## Dependency Contracts

### Module Import Rules

1. **STABLE modules** ✅
   - May import from STABLE modules only
   - Exception: Ecosystem/external crates
   - Test coverage: ≥90%
   - Used by ≥2 binaries OR ≥15 tests

2. **BETA modules** ⚠️
   - May import from STABLE + BETA modules only
   - May use one EXPERIMENTAL module max
   - Test coverage: ≥70%
   - Active integration phase

3. **EXPERIMENTAL modules** 🔬
   - May import from any module (loose coupling)
   - Test coverage: ≥40% OR actively developed
   - Unstable API, subject to rapid change
   - Breaking changes require notice

4. **DISABLED modules** ❌
   - Marked with `#[deprecated]` in source
   - No new imports allowed
   - May require major version bump to remove

### Circular Dependencies

**FORBIDDEN.** Enforcement:
- Pre-commit: `./scripts/validate-imports.sh` — fails if cycles detected
- CI/CD: `./scripts/audit-modules.sh` — `cargo tree` inspection
- Code review: ARCHITECTURE.md must be updated on every change

---

## Module Status Definitions

### STABLE
- ✅ Production-ready, 100% pass rate
- Used by 2+ binaries OR 15+ passing tests
- API frozen for current major version
- Deprecations announced 2+ releases ahead
- Examples: `ntg::storage`, `genomic::vcf_stream`, `ntg::operators`

### BETA
- ⚠️ Tested and integrated, actively maintained
- Used by 1-2 binaries OR 10-14 passing tests
- API subject to refinement based on feedback
- Breaking changes require minor version bump + notice
- Examples: `ntg::observability`, `genomic::synthesis`, `genomic::vitascale`

### EXPERIMENTAL
- 🔬 Under development, limited testing
- <10 passing tests OR exploratory phase
- API may change without notice
- No SLO for stability
- Examples: `ntg::bytemerge`, `ntg::fsevents`, `cuda::kernels`

### DISABLED
- ❌ No longer available for import
- Marked `#[deprecated]` in source
- Binary that imports will not compile
- Removal scheduled for next major version
- Migration path documented in deprecation message

---

## Verification & Validation

### Pre-Commit Validation

```bash
./scripts/validate-imports.sh
```

Checks:
- All imported modules exist in ARCHITECTURE.md
- No EXPERIMENTAL→BETA/STABLE imports (violations allowed for EXPERIMENTAL only)
- No circular dependencies detected by `cargo tree`
- ARCHITECTURE.md modified if module status changed

### CI/CD Audit (on push)

```bash
./scripts/audit-modules.sh
```

Runs:
- `cargo check --all` — compilation success
- `cargo test --lib` — test pass rate ≥95%
- `cargo clippy -- -D warnings` — lint clean
- Module registry consistency check
- Test count verification (maintained in commit message)

### Code Review Checklist

For all PRs modifying code or ARCHITECTURE.md:

- [ ] ARCHITECTURE.md updated if modules added/removed/status changed
- [ ] No new imports violate dependency rules
- [ ] Test count in commit message matches cargo output
- [ ] All 3 validation scripts pass locally before push
- [ ] Clippy debt not increased

---

## Binary → Module Usage Map

| Binary | Primary Modules | Purpose | Status |
|--------|-----------------|---------|--------|
| `phase4_calib` | ntg::{storage,operators,calib}, genomic::{vcf_stream,real_pipeline} | Calibration, model roundtrip | ✅ STABLE |
| `kernel_host` | ntg::{runtime,schooling,graph}, genomic::{agents,domain_agents} | NanoKeymaster routing agent | ✅ STABLE |
| `density_bench` | ntg::{ternary,simd,storage} | SIMD throughput benchmark | ✅ STABLE |
| (test suite) | All 31 modules, 405 tests total | Regression & integration | ✅ STABLE |

---

## Phase 6 Completion Criteria ✅

- [x] All 31 modules documented with status
- [x] Dependency contracts defined and enforced
- [x] Module import validation scripts implemented
- [x] CI/CD audit job added to workflow
- [x] Test coverage tracking enabled
- [x] Binary → module usage map created
- [x] Circular dependency detection in place
- [x] ARCHITECTURE.md as single source of truth

---

**Maintained by:** Development Governance Board  
**Last Updated:** 2026-07-28  
**Next Phase:** Phase 7 — WebSocket 60 Hz Real-Time Streaming
