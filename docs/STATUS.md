# Aethyro NTG Engine — Project Status Report

**As of:** 2026-10-27 (facts re-measured; narrative sections below are from 2026-07-12 unless noted)  
**Capability version:** 10 (`ternary_capability()` — Phase 5 runtime calib supported)  
**Build:** `cargo test --release` green: <!--tests-->683<!--/tests--> passed, 0 failed, 0 ignored. `cargo clippy --all-targets -- -D warnings` clean. Whole-stack smoke passes. Details: [BASELINE_2026-10-26.md](BASELINE_2026-10-26.md)  
**Phase 6 kernel gate (2026-10-26): passes for ternary-weight x int8-activation on a real pretrained model; fails for ternary x ternary popcount.** The popcount kernel is slower than SIMD f32 (0.15x to 1.05x). The `ternary_i8_gemm` kernel (2-bit weights unpacked in-register, AVX-512 VNNI / AVX2, exact `i32`) was run inside Microsoft's natively ternary-trained **BitNet b1.58 2B-4T** (all 210 BitLinear matmuls swapped, everything else official code): wikitext-2 perplexity 22.03 vs 22.06 official, ARC-Easy 75.3% vs 76.3% on 300 questions (not distinguishable), as close to the exact-specification model as the official implementation is. Decode-time matmuls at the model's real shapes are 7.7x to 8.5x faster than a bf16 kernel and 3.0x to 3.3x faster than dense int8 on one core (4.3x and 1.8x to 1.9x on four), with linear weights 16x smaller than f32 (0.52 GB). **Not good:** prefill was 2.7x to 3.3x slower than torch's path on the AMX host and is now 6x faster than torch bf16 on a non-AMX host with the panel prefill kernel (AMX host not retested), end-to-end decode through the Python harness is only 1.8x faster, four-thread matmul scaling was poor (1.5x with rayon) and is now 2.6-3.1x with the spinning pool, though end to end it only helps when torch and the kernel do not share all cores, and the reference runtime `bitnet.cpp` decodes the whole model 1.3x (one thread) to 3.5x (four threads) faster than this kernel inside Hugging Face on the same VM (kernel-level parity unknown; `docs/results/bitnet_cpp/`). Not run on tatortot. Details, raw data and caveats: [EXPERIMENTS.md](EXPERIMENTS.md) (2026-10-26, three entries) and `docs/results/bitnet_2b_4t/`. **Quad-Brain:** now on `main`, with γ and δ genuinely wired and `kernel_host` feeding it real routing behaviour (see [CLAUDE.md](../CLAUDE.md)); previously it existed only on an unmerged branch.  
**Known stub:** `hostframe-backend` accepts `/ingest` and serves `/metrics`, but its BigQuery client is a stub: it keeps counters in memory and does not write to BigQuery (startup only needs `GCP_PROJECT`). The telemetry → BigQuery → dashboard loop is therefore not wired end to end.  
**Foundation Lock (plan: [FOUNDATION_LOCK_PLAN.md](FOUNDATION_LOCK_PLAN.md)):** in progress. Done and recorded: marker accounting (A2, [baseline addendum](BASELINE_2026-10-26.md)), keymaster contract ([ADR 0012](architecture/0012-keymaster-contract.md), option A accepted: `KEYMASTER_BACKEND_URL` is a generic `{intent,payload}` backend, fail-closed; `backend/keymaster` is a separate policy service `kernel_host` does not call; the smoke test now exercises the contract with a mock backend), telemetry loop declared ingest-only ([ADR 0013](architecture/0013-telemetry-loop.md)), PackedTernary ADR (already ADR 0005), webhooks stub deleted (nothing consumed it), phase vocabulary. A1 live deploy done: `hostframe-backend` verified from outside on Render, https://aethyro-hostframe.onrender.com, 2026-10-27 ([DEPLOY.md](DEPLOY.md)). Waiting on the owner: A3 branch deletion and Dependabot #25, A4 `v0.1.0` tag. The `cuda` feature does not link (no build step for the `.cu` files).  
**Authority:** This document is the single source of truth for “where the project is.” Older session notes (`BUILD_STATUS.md`, `BREAKTHROUGH_SUMMARY.md`, `PHASE3_SUMMARY.md`) are historical; prefer this file and [ROADMAP.md](ROADMAP.md). **Phase names:** the 0–5 / 6.x / F scheme in the root [CLAUDE.md](../CLAUDE.md) phase table is the one vocabulary; the “7.5.x” labels in `PHASE_7_5_ROADMAP.md` were planning labels for a hygiene pass, not project phases. **Test counts** in README, CLAUDE.md and this file are checked against real test output by `scripts/check-doc-counts.sh` in CI.

### Sovereign stack (2026-07-12 — post Phase F rungs)

In addition to kernel Phases 0–5, the genomic path now has a measured
**SovereignBrain** multi-organ loop (ADR 0009):

| Piece | Location | Evidence |
|-------|----------|----------|
| Multi-chr brain + LTM + working set | `genomic/sovereign_brain.rs` | unit + campaign |
| Language/SIS organ + `activate_from_text` | `genomic/language_organ.rs` | unit + demo |
| Multi-axis fitness + ledgered select | `genomic/sovereign_fitness.rs` | train✓ prune✗ pattern |
| Real 1000G multi-chr campaign | `bin/sovereign_campaign` | EXPERIMENTS.md |
| LD r² bitparallel ~11× | `bitsliced_genotypes` / `ld_compute` | EXPERIMENTS.md |

**Not product-ready:** no aethyro.com head-to-head; durable memory is
snapshot-based (not full lifelong host integration); self-mod remains
off by default outside explicit selection loops. Harden notes:
[SOVEREIGN_HARDEN.md](SOVEREIGN_HARDEN.md).

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
| `cargo test --release` (kernel) | <!--tests-->683<!--/tests--> passed, 0 failed, 0 ignored (capability v10; calib model/sparse/compare tests included) |
| `cargo build --release` | Success (`libntg_kernel.{so,rlib}`, `phase4_calib`, benches) |
| CI | `.github/workflows/ci.yml`: kernel test + phase4 smoke + model roundtrip + density_bench + clippy + doc-count check; keymaster test + clippy; whole-stack smoke (dashboard + kernel_host + keymaster) |
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

### Phase 5 and later
Phase 5 is certified COMPLETE (see the phase-gate table above). Later work is tracked under the 6.x / F scheme in [CLAUDE.md](../CLAUDE.md): 6.0 re-measured (kernel gate fails on throughput, see EXPERIMENTS.md 2026-10-26), 6.1–6.18 present with the caveats in CLAUDE.md, Phase F (L0) scaffold complete but no verified live deployment. Still open: Phase 6 host product path (WASM/FFI) per the gate protocol, and the aethyro.com head-to-head.

---

## 5. Testing proof matrix

| Suite | Count | What it proves |
|-------|------:|----------------|
| Unit (`--lib`) | 485 | Core algorithms, ledger, mutation, storage, runtime, graph, accel, genomic, VITASCALE |
| `hostframe_integration` | 7 | Telemetry payloads against a mock backend |
| `phase1_2_3_simd_ffi` | 11 | SIMD parity, FFI, OpStats |
| `phase1_2_3_storage_integration` | 10 | PackedTernary + TOBL + ledger glue |
| `phase3_integration` | 7 | All five ADR 0002 rails end-to-end |
| `prop_ld_compute` | 4 | LD fast path vs scalar reference (property tests) |
| `self_parse` | 3 | Real repo docs parse without panic |
| `sovereign_integration` | 2 | SovereignBrain integration |
| `test_genomic_operator` | 11 | Genomic operator |
| **Total** | **<!--tests-->683<!--/tests-->** | measured 2026-10-26 |

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
3. ~~Canonical storage ADR~~ **DONE** — [ADR 0005](architecture/0005-canonical-ternary-storage.md) (accepted 2026-07-09; addendum 2026-10-27). `SparseBitSlicedTernary` for graph weights, `BitSlicedTernary` for dense popcount, `storage::PackedTernary` for TOBL/FFI, `ntg::packed::PackedTernary` legacy.

### P1 — Engineering hardening
1. ~~Unify or clearly namespace dual `PackedTernary`~~ **Namespaced** per ADR 0005; unification deferred until a measured win.
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

- This section described a 2026-07-12 local tree that has long since been committed and merged; use `git log` for delivery state.
- Open PRs/branches are tracked in GitHub, not here.

---

## 9. Capability snapshot (v10)

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
phase4_calibration_supported: true
phase5_runtime_calib_supported: true
version: 10
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
**Merge readiness:** `main` is green in CI (kernel, keymaster, whole-stack smoke).
