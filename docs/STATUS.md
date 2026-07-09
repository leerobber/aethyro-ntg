# Aethyro NTG Engine — Project Status Report

**As of:** 2026-07-09  
**Capability version:** 10 (`ternary_capability()` — Phase 5 runtime calib supported)  
**Build:** `cargo test` + `cargo build --release` green on host  
**Authority:** This document is the single source of truth for “where the project is.” Older session notes (`BUILD_STATUS.md`, `BREAKTHROUGH_SUMMARY.md`, `PHASE3_SUMMARY.md`) are historical; prefer this file and [ROADMAP.md](ROADMAP.md).

---

## 1. Executive verdict

| Dimension | Assessment |
|-----------|------------|
| **Maturity** | Pre-alpha **research kernel** — substantial foundations, not a product |
| **Correctness posture** | Strong: Result-based APIs, bit-identity tests, ledger tamper tests, ADR rails tested |
| **Performance posture** | Improving: density micro-bench recorded (bit-sliced ~12× vs i8 scalar; sparse conditional). Still no end-to-end vs aethyro.com; AVX-512 full kernels open |
| **Safety posture** | Phase 3 rails implemented and tested; self-mod **off by default** |
| **Product readiness** | **Not ready** — no production-benchmark win/loss vs aethyro.com inference |
| **Primary risk** | Docs and marketing language outrunning measurements; dual storage stacks need a clear “canonical path” story |

**Bottom line:** The engine has a real, tested stack through **Phase 5**: ternary storage → graph/SIS → ledger/self-mod → calibration → **precision push + CalibModel→GraphNode production scoring**. **Doctorate schooling** (`ntg_school`, [docs/schooling/](schooling/)) runs study+exam on real data for Phases 0–5 with a **75% pass bar** and full redo on fail — multi-run notebooks under `docs/schooling/runs/`. What it does **not** have is Phase 6 integration (WASM/FFI host product path), aethyro.com head-to-head, or GPU (explicitly deferred — CPU TOBL already 12–20×).

### Phase gates (policy 2026-07-09, certificates filed)

| Question | Answer |
|----------|--------|
| Soft advance without certificates? | **Forbidden** — see [PHASE_GATE_PROTOCOL.md](PHASE_GATE_PROTOCOL.md) |
| Phases 0–5 COMPLETE certificates? | **YES** — `docs/phases/PHASE_{0,1,2,3,4,5}_COMPLETE.md` |
| May Phase 6 begin? | **YES** — Integration (host load of frozen models + production compare) |

**Process:** after each phase COMPLETE, deep-dive is in the certificate;
do not start N+1 until N is certified.

---

## 2. Evidence base (this audit)

| Check | Result |
|-------|--------|
| `cargo test` (kernel) | Unit + integration suites green (capability v9; calib model/sparse/compare tests included) |
| `cargo build --release` | Success (`libntg_kernel.{so,rlib}`, `phase4_calib`, benches) |
| CI | `.github/workflows/ci.yml`: test + phase4 smoke + model roundtrip + density_bench + clippy |
| Host hardware (audit machine) | x86_64 with AVX2 + AVX-512F/VPOPCNTDQ advertised |

---

## 3. Architecture as implemented

```
┌──────────────────────────────────────────────────────────────┐
│  tools/ingest.py  — sequential GraphNode.id contract         │
└────────────────────────────┬─────────────────────────────────┘
                             │
┌────────────────────────────┴─────────────────────────────────┐
│  Runtime (ntg/runtime.rs) + AccelManager (ntg/accel.rs)      │
│  forward_native_parallel → density-selected AccelDevice      │
│  GraphNode.weights: SparseBitSlicedTernary                   │
└────────────────────────────┬─────────────────────────────────┘
┌────────────────────────────┴─────────────────────────────────┐
│  Mutation engine (ntg/mutation/) — OFF by default            │
│  BudgetTracker + FitnessEvaluator + MutationRule             │
└────────────────────────────┬─────────────────────────────────┘
┌────────────────────────────┴─────────────────────────────────┐
│  TamperEvidentLedger (ntg/ledger/) — SHA-256 chain + sign    │
│  StateSlotStore lineage, ExecutionTrace, SignedEntry         │
└────────────────────────────┬─────────────────────────────────┘
┌────────────────────────────┴─────────────────────────────────┐
│  Graph (ntg/graph/) — structural Node + edges + adj_list     │
│  docparse / pathparse / fsevents / leafsignal / interaction  │
└────────────────────────────┬─────────────────────────────────┘
┌────────────────────────────┴─────────────────────────────────┐
│  Storage family (ntg/storage/ + packed.rs + ternary.rs)      │
│  i8 scalar · PackedTernary · BitSliced · SparseBitSliced     │
│  SIMD dispatcher + TOBL FFI (ntg/simd/, ntg/ffi/)            │
└──────────────────────────────────────────────────────────────┘
```

### Module map (kernel)

| Path | Role | Maturity |
|------|------|----------|
| `ntg/ternary.rs` | Scalar golden reference, `encode` / `encode_fixed`, `matmul_scalar` | **Solid** |
| `ntg/packed.rs` | Early 2-bit packing (4 values/byte) | Solid; overlaps storage PackedTernary |
| `ntg/storage/packed_ternary.rs` | Cache-aligned 2-bit/u64 packing + density/generation | Solid |
| `ntg/storage/bit_sliced_ternary.rs` | Dual-stream pos/neg, 64-wide popcount dot | Solid |
| `ntg/storage/sparse_bit_sliced_ternary.rs` | Flat COO sparse, tombstones, matmul/residual, ledger mutations | Solid |
| `ntg/storage/tobl_kernel.rs` | TOBL dot dispatch (scalar / AVX2 / NEON stub) | Correctness-first; perf unproven |
| `ntg/simd/*` | Runtime SIMD path selection + AVX2 wrapper | Bit-identity OK; AVX2 matmul is correctness path |
| `ntg/ffi/*` | C ABI matmul + TOBL handles + OpStats | Solid boundary tests |
| `ntg/graph/*` | Structural graph + `GraphNode` weights | Solid; adj_list added |
| `ntg/docparse`, `pathparse`, `fsevents`, `leafsignal` | SIS front-end (ADR 0003 partial) | Solid within scope |
| `ntg/interaction.rs` | Edge interaction scores | Kept with known empirical limits |
| `ntg/chain.rs` | Non-crypto hash chain (legacy / change detect) | Honest scope |
| `ntg/ledger/*` | Crypto ledger + slots + replay | Solid for Phase 3 rails |
| `ntg/mutation/*` | Self-mod rules, budget, fitness | Solid; disabled default |
| `ntg/runtime.rs` + `accel.rs` | Native parallel forward + device select | Solid API; HW kernels shared |
| `tools/ingest.py` | Layer node-ID contract | Solid |

---

## 4. Phase status (honest)

### Phase 0 — Repo / process — **DONE**
- Private proprietary LICENSE, CI, ADRs, CONTRIBUTING.

### Phase 1 — Ternary core — **MOSTLY DONE**
| Item | Status |
|------|--------|
| 1.1 Scalar reference + tests | ✅ |
| 1.2 Packed storage | ✅ (two implementations: `packed.rs` + `storage/packed_ternary.rs`) |
| Bit-sliced dense + sparse COO | ✅ |
| SIMD dispatch + bit-identity tests | ✅ |
| Native runtime + accel selection | ✅ |
| Recorded micro-bench deltas vs scalar (dots) | ✅ EXPERIMENTS.md 2026-07-09 |
| End-to-end / GEMM / production deltas | ❌ **open** |
| True AVX-512 multi-block popcnt kernels | ❌ detect only / portable `count_ones` |
| NEON full path | ⚠️ stub / fallback |

### Phase 2 — Graph + SIS — **STRUCTURALLY DONE; GAPS REMAIN**
| Item | Status |
|------|--------|
| Graph CRUD, topo order, forward_pass, fingerprint | ✅ |
| adj_list O(degree) children | ✅ |
| Doc / path parse, fs-event pure layer, leaf signal | ✅ |
| Self-parse dogfood tests | ✅ |
| GraphNode + sparse weights for native runtime | ✅ |
| Lazy glyph / PIXEL-lite | ❌ design only (ADR 0003) |
| Forward-pass overhead vs static baseline (measured) | ❌ |
| Edge interaction as structural relatedness | ❌ empirically weak (see EXPERIMENTS) |

### Phase 3 — Self-mod + ledger — **DONE for ADR 0002 rails**
| Rail | Status |
|------|--------|
| 1 Off by default | ✅ tested |
| 2 Bounded budget | ✅ tested |
| 3 Auto rollback on regression | ✅ tested |
| 4 Deterministic replay | ✅ tested |
| 5 Every mutation ledger-logged | ✅ tested |
| SHA-256 chain + signed entries | ✅ |
| StateSlot lineage (1-based parent pointers) | ✅ |

**Not claimed:** production mmap ChronosLedger file format parity, multi-agent orchestration, live Reflexive Fitness critics driving topology at scale.

### Phase 4 — **COMPLETE** (2026-07-09)
- Certificate: `docs/phases/PHASE_4_COMPLETE.md`
- Real docs WIN: bal_acc≈0.61, rec≈0.25, fp≈12; self-mod off by default
- Optional `--self-mod` probe ledgered, fitness may reject

### Phase 5–8 — **NOT STARTED** (Phase 5 may begin)
GPU/optimization, WASM, product head-to-head remain later.

---

## 5. Testing proof matrix

| Suite | Count | What it proves |
|-------|------:|----------------|
| Unit (`--lib`) | 182 | Core algorithms, ledger, mutation, storage, runtime, graph, accel |
| `phase1_2_3_simd_ffi` | 11 | SIMD parity, FFI, OpStats |
| `phase1_2_3_storage_integration` | 10 | PackedTernary + TOBL + ledger glue |
| `phase3_integration` | 7 | All five ADR 0002 rails end-to-end |
| `self_parse` | 3 | Real repo docs parse without panic |
| **Total** | **213** | |

### Known test honesty notes
- Sparse `ternary_matmul` is **chunk-level score → ±1 gate**, not full dense GEMM.
- AVX2 dense matmul currently prioritizes **scalar-correct bit-identity** over peak SIMD throughput.
- AccelDevice variants share the same sparse kernel today; selection is policy + observability, not separate HW implementations.
- `edge_interaction_score` tests prove self-similarity / edit sensitivity, **not** structural edge discovery on real docs.

---

## 6. Documentation inventory (organized)

| Document | Role after this audit |
|----------|------------------------|
| **[STATUS.md](STATUS.md)** (this file) | **Current truth** — read first |
| [ROADMAP.md](ROADMAP.md) | Phased checklist (kept in sync with STATUS) |
| [DESIGN.md](DESIGN.md) | Target architecture (updated layer diagram) |
| [LITERATURE.md](LITERATURE.md) | Novelty claims + sources |
| [EXPERIMENTS.md](EXPERIMENTS.md) | Measured wins / non-wins |
| [architecture/](architecture/) | ADRs 0001–0004 |
| [PHASE1_2_3_*.md](PHASE1_2_3_IMPLEMENTATION.md) | Historical implementation notes (may lag STATUS) |
| Root `BUILD_STATUS.md`, `BREAKTHROUGH_SUMMARY.md`, `PHASE3_SUMMARY.md` | **Archived session narratives** → see STATUS |
| `kernel/FFI_INTEGRATION.md`, `TOBL_FFI_REFERENCE.md` | FFI operator docs |
| `CONTRIBUTING.md` | Engineering rules |

---

## 7. Critical gaps & recommended next work (agency priority)

### P0 — Truth & measurement
1. ~~Micro-bench suite~~ **DONE 2026-07-09** — `cargo run --release --bin density_bench`
2. ~~Log numbers in EXPERIMENTS.md~~ **DONE** — see EXPERIMENTS.md “density micro-bench”
   - Bit-sliced ~12× vs scalar i8 at N=262144
   - Sparse best at 1% (~20×); at 10–50% random fill loses to bit-sliced
3. **Canonical storage ADR**: which type GraphNode / product path owns long-term (**open**; naming note added in `storage/mod.rs`)

### P1 — Engineering hardening
1. Unify or clearly namespace dual `PackedTernary` (`packed.rs` vs `storage/`).
2. Real AVX-512 VPOPCNTDQ kernels for dense dual-stream words (host already has features).
3. Wire OpStats + device name into ledger entries on forward.
4. CI: ensure clippy `-D warnings` stays green (dead-code stubs allowed explicitly).

### P2 — Phase 4 entry
1. One real calibration task (small) exercising Runtime + ledger.
2. Report outcome honestly regardless of win/loss.

### Explicit non-goals (now)
- Product marketing claims of “40% cycle reduction” until measured.
- Legal/Healthcare vertical code.
- Claiming ChronosLedger full file format compatibility.

---

## 8. Git / delivery state (audit host)

- Branch: `main`, **ahead of origin by 5 commits** with large uncommitted local work (storage, runtime, accel, graph split, docs).
- Recommendation: one clean commit series after STATUS/ROADMAP sync:
  1. storage + TOBL correctness
  2. sparse + runtime + accel
  3. graph adj_list + GraphNode
  4. docs STATUS package

---

## 9. Capability snapshot (v8)

```
scalar_supported: true
packed_supported: true
simd_supported: true          // dispatcher present; not “all paths peak HW”
graph_supported: true
doc_path_parsing_supported: true
forward_pass_supported: true  // structural LeafSignal aggregate
fingerprint_supported: true
edge_interaction_score_supported: true  // narrow properties only
chain_log_supported: true
bit_sliced_supported: true
sparse_bit_sliced_supported: true
native_parallel_forward_supported: true
version: 8
```

---

## 10. Sign-off

| Role lens | Conclusion |
|-----------|------------|
| **Research agency** | Credible experimental platform with unusually honest non-win documentation (edge scoring). |
| **Systems eng** | Kernel modular, test-heavy, replay/safety conscious; perf path incomplete. |
| **Product** | Do not ship efficiency claims yet. |
| **Security / sovereignty** | Ledger + off-by-default self-mod is the right shape; needs threat model review before air-gap product claims. |

**Status classification:** `RESEARCH-READY / PRE-PRODUCT`  
**Merge readiness of local tree:** code green locally; docs consolidated here; commit & CI push still required.
