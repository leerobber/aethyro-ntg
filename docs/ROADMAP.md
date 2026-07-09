# Aethyro NTG Engine — build roadmap

**Live status snapshot:** [STATUS.md](STATUS.md) (2026-07-09 audit).  
This file is the phase checklist; STATUS is the research-agency report.

See [DESIGN.md](DESIGN.md) for the architecture these phases build, and
[ADR 0001](architecture/0001-vision-and-pivot.md) for why there's no
fixed calendar here — phases are gated on real green CI and real
measurements, not dates.

**Non-negotiable rule across every phase:** docs updated + full CI green
before starting the next phase. No exceptions, no "we'll fix the tests
later." A phase that isn't green isn't done, regardless of how much code
exists for it.

**Test baseline (2026-07-09):** 200+ automated tests green (`cargo test`
in `kernel/`). Capability report version **10** (Phase 5 optimization complete).

### Phase gate policy (binding — 2026-07-09)

See **[PHASE_GATE_PROTOCOL.md](PHASE_GATE_PROTOCOL.md)**.

| Gate | Verdict |
|------|---------|
| Soft advance without certificates | **REJECTED** |
| Phases 0–3 COMPLETE certificates | **YES** — `docs/phases/PHASE_{0,1,2,3}_COMPLETE.md` |
| Phase 4 may begin | **YES** (only after certificates + green tests; still implement Phase 4 as its own gated phase) |

---

## Phase 0 — Repo setup ✅ done

- [x] Private repo, proprietary LICENSE (not Apache 2.0 — see repo history)
- [x] `docs/architecture/` ADR structure (0001 vision, 0002 safety rails)
- [x] `docs/DESIGN.md`, `docs/LITERATURE.md`, `docs/ROADMAP.md` (this file)
- [x] `kernel/` Rust crate scaffold + CI (cargo test + clippy)

## Phase 1 — Ternary Tensor Core

### 1.1 Scalar reference ✅ done — green on CI (PR #1)
- [x] `Ternary` enum, `encode()` (absmean threshold), `matmul_scalar()`
- [x] `NtgError` — `Result`-based, no panics on bad input
- [x] Unit tests: roundtrip, invalid-value rejection, threshold
      separation, empty input, zero-vector matmul, hand-computed matmul
      reference, shape-mismatch rejection
- [x] Confirmed green on GitHub Actions (CI caught one real bug: an
      arithmetic error in a hand-computed test expectation, inherited
      from an unverified pasted example — fixed, not the implementation)
- [x] Measured baselines: density_bench + EXPERIMENTS.md (host wall-clock);
      CI-runner-specific counters optional ops note in PHASE_1_COMPLETE

### 1.2 Bit-packing + SIMD + sparse TOBL ✅ COMPLETE
- [x] `PackedTernary` (early `packed.rs` + storage `packed_ternary.rs`)
- [x] Tests: roundtrip, density, bounds, invalid input
- [x] Runtime feature-detected SIMD dispatch (`simd/`) + TOBL paths;
      scalar bit-identity enforced in tests
- [x] **BitSlicedTernary** — dual-stream pos/neg, 64-wide popcount dot
- [x] **SparseBitSlicedTernary** — flat COO, tombstones, ledgered compact
- [x] `ternary_matmul` + `sparse_residual_add` (chunk merge-join)
- [x] `Runtime::forward_native_parallel` + `AccelManager` density select
- [x] `GraphNode` weighted nodes (`graph/node.rs`)
- [x] `tools/ingest.py` sequential layer node-ID contract
- [x] Micro-bench vs scalar i8 dots; recorded in EXPERIMENTS.md
- [x] AVX-512 multi-block **re-scoped to Phase 5** (ADR/PHASE_1_COMPLETE:
      detect exists; portable popcount is production path)
- [x] NEON path **documented complete for Phase 1**: bit-identical
      aarch64 chunk path + scalar fallback; full `vmull` → Phase 5
- [x] Canonical storage: [ADR 0005](architecture/0005-canonical-ternary-storage.md)

### 1.3 FFI + observability ✅ DONE
- [x] `#[no_mangle] extern "C"` surface for orchestrator integration (ntg_matmul_ffi)
- [x] `Stats` struct (op count, timing) feeding future ledger ingestion (OpStats)
- [x] Memory-safety review of the FFI boundary (all pointer validation, no panics)
- [x] Integration test calling the FFI surface (phase1_2_3_simd_ffi.rs)
- [x] **PackedTernary storage layer** — 2-bit ternary encoding, cache-aligned
- [x] **TOBL kernel dispatch** — runtime-selected dot-product (AVX2/NEON/Scalar)
- [x] **FFI TOBL extension** — C interface for PackedTernary operations (tobl_ffi.rs)
- [x] **Observability hooks** — density metric + cycle tracking for Phase 3+ integration

**Phase 1 exit criteria:** scalar, SIMD, and FFI paths all green in CI,
SIMD/FFI outputs proven bit-identical to the scalar reference (see
[PHASE1_2_3_IMPLEMENTATION.md](PHASE1_2_3_IMPLEMENTATION.md) for test results)
measured performance delta recorded (positive or not) before Phase 2
starts.

## Phase 2 — Graph Structure (+ SIS front-end, ADR 0003)

- [x] `kernel/src/ntg/graph/`: node/edge representation, `add_node`,
      `remove_node`, `add_edge`, `remove_edge` as first-class operations
- [x] Adjacency list (`adj_list`) for O(degree) `children()`; flat
      `edges` retained for serialization / audit
- [x] Deterministic forward iteration order (`children()` is
      insertion-ordered) — proven under test
- [x] Typed nodes: `NodeKind::Content` / `NodeKind::Execution` (ADR 0003)
- [x] Weighted runtime nodes: `GraphNode { id, weights }`
- [x] Document structure parser (`docparse.rs`): headings (nested by
      level), bullets, numbered items, fenced code blocks (->
      `Execution` nodes) — GraphMD-style structural parsing, tested
      (including nested-heading reparenting)
- [x] Self-parse test (`kernel/tests/self_parse.rs`): the real, buildable
      version of "self-referential kernel" — this repo's own ADRs and
      DESIGN.md are parsed by its own parser and checked for sane
      structure (no panic, >1 node, zero Execution nodes in the
      fence-free ADRs, ≥1 in DESIGN.md's one fenced diagram) — not
      recursive self-awareness, just dogfooding, CI-enforced
- [x] Path parser (`pathparse.rs`): filesystem paths -> the same typed
      graph (directory segments as `Content` nodes, leaf typed
      `Execution`/`Content` by extension — `.rs`/`.py`/`.sh`/`.js`/`.ts`
      count as executable, everything else is content); shared
      directory prefixes reuse existing nodes instead of duplicating
      them; tested including a pure lookup-only `find_path`
- [x] Fs-event -> graph mutation (`fsevents.rs`): `Created`/`Removed`/
      `Renamed` translated into real `add_node`/`remove_node` calls,
      tested including the no-op-on-missing-path case. **This is not a
      real filesystem watcher** — it's the pure, deterministic
      translation layer only. Wiring it to actual OS filesystem events
      needs an external crate (e.g. `notify`) — a new dependency
      decision, deliberately not made in this pass. Do not describe this
      as "watching the filesystem" until that wiring exists.
- [x] Leaf signal extractor (`leafsignal.rs`): real per-character
      case/punctuation/whitespace counts, every character accounted for
      (tested: total always equals input length, nothing dropped). This
      is **not** the PIXEL-lite glyph-geometry fingerprint ADR 0003
      describes — that needs an actual trained visual feature
      extractor, which doesn't exist here. This is a plain, honest count,
      not a learned representation.
- [x] Leaf signal wired onto graph nodes: every `Node` now carries a
      real `signal: LeafSignal` field, computed from its `label` at
      creation time in `add_node` (always in sync — no separate API to
      set it, so it can't drift from the label it describes)
- [x] `kernel/src/ntg/chain.rs` (`ChainLog`): a real hash-chain
      primitive, built after directly reading GH05T3's actual
      ChronosLedger source and finding the plan's original assumption
      false — see [docs/EXPERIMENTS.md](EXPERIMENTS.md). ChronosLedger
      is a real, reusable mutable state store with **no** hashing or
      tamper-evidence; `seal.py`'s LexGenSeal is real per-record SHA256
      signing but not chained; no genuine hash chain existed anywhere.
      Tested: altering or removing a historical entry breaks
      verification from that point forward; a given content string's
      chain value depends on what preceded it, not just itself. Uses
      `std`'s non-cryptographic `DefaultHasher` for now (same honest
      caveat as `Graph::fingerprint`) — a real crypto hash is a
      dependency decision for whoever wires up the full Phase 3 ledger.
- [x] Full ledger module — **completed in Phase 3** (`ntg/ledger/`):
      CryptoChainLog (SHA-256), SignedEntry, StateSlotStore, ExecutionTrace,
      TamperEvidentLedger. (Non-crypto `ChainLog` in `chain.rs` remains for
      change-detection-style use; crypto path is the ledger.)
- [x] Actual "forward pass" over the graph (`Graph::forward_pass`): a
      real Kahn's-algorithm `topological_order` (dataflow-ordered
      execution — a node runs only once every node with an edge into it
      already has, ties broken by ascending id for determinism) feeding
      an aggregation of every node's `LeafSignal`. Tested: edges (not
      creation/id order) determine execution order; cycles are detected
      and rejected rather than looping forever; every node is visited
      exactly once (checked against a manual sum); repeated runs on the
      same graph agree (ADR 0002 replay property, proven not assumed).
      This is the real version of "time-irrelevant execution" — order
      comes from dependency readiness, not a fixed loop. It is **not**
      full ternary-tensor compute over the graph — attaching real ops
      per node (so the graph does more than aggregate a signal count)
      is a separate, larger feature, not yet started.
- [x] Property tests: same topology + same input -> same output, across
      repeated `forward_pass` runs — proven, not assumed
- [x] `Graph::fingerprint`: deterministic std-hash (SipHash via
      `DefaultHasher`, not cryptographic) over dataflow-ordered
      `(kind, label, signal, child_count)` — for Phase 3's ledger to
      skip logging a "change" when nothing actually changed. Tested:
      identical content -> identical fingerprint; a changed label or a
      changed structural shape -> a different one; stable across
      repeated calls. **Not a substitute for the real ledger's
      tamper-evidence hash** (needs SHA-256/BLAKE3, an external
      dependency decision, not made here) — this is change detection only.
- [x] `Graph::edge_interaction_score` / `encode_fixed`
      (`interaction.rs`, `ternary.rs`): the real ternary-matmul "edge
      interaction score" between two nodes' labels — attempt #1
      (`encode()`'s per-string threshold) failed, diagnosed, documented;
      the fix (a fixed/global threshold, `encode_fixed`) was validated
      empirically in Python *before* being written as Rust, and works:
      opposite-byte strings score negatively, self-similarity beats a
      one-character edit. Honestly scoped as a byte-position
      correlation, not semantic similarity, and sensitive to positional
      shifts. **Tested against this repo's real 486-edge ADR/doc graph:
      does not reliably distinguish a real heading→content edge from a
      random pair** — raw score correlates 0.56-0.60 with string length
      (confound); `normalized_edge_interaction_score` removes that
      confound but the real-vs-random gap mostly disappears with it
      (0.155 vs. 0.132 mean, within one std). Kept in the codebase for
      its narrower, still-true properties (self-similarity, edit
      sensitivity); **not evidence this generalizes to structural
      relatedness** — that likely needs actual learned weights (Phase 4),
      not a fixed untrained encoding. Full experiment trail in
      [docs/EXPERIMENTS.md](EXPERIMENTS.md).
- [x] Benchmark: forward-pass vs static signal fold —
      `graph_overhead_bench` + EXPERIMENTS.md (~10.5× on 8-node sample)
- [x] Lazy leaf resolution: `lazyleaf.rs` + `Graph::resolve_leaf_body`
- [x] Glyph fingerprint v0: `glyph.rs` (deterministic shape-class proxy;
      **not** trained PIXEL — trained PIXEL re-scoped to Phase 4+ in
      PHASE_2_COMPLETE / ADR 0003 progress note)
- [x] Byte-level cost mitigation: `bytemerge.rs` deterministic merge + tests
- [x] Execution-typed node runs ledger-logged:
      `Graph::log_execution_nodes`

**Phase 2 status:** **COMPLETE** — see [phases/PHASE_2_COMPLETE.md](phases/PHASE_2_COMPLETE.md).

## Phase 3 — Self-Modification Engine (gated by ADR 0002) ✅ DONE

**Completed 2026-07-08. See [ADR 0004](docs/architecture/0004-phase3-tamper-evident-ledger.md) for full details.**

- [x] `CryptoChainLog` chaining primitive — SHA-256 sequence integrity
- [x] StateSlotStore — ChronosLedger-inspired mutable state + lineage
- [x] Per-record signing scheme — LexGenSeal-inspired SHA-256 hashing
- [x] Cryptographic hash (SHA-256) replacing DefaultHasher
- [x] Rule-based mutation proposers (5 core rules) as versioned artifacts
- [x] Dual-objective fitness evaluator (latency + memory)
- [x] Bounded compute/time budget enforcement (BudgetTracker)
- [x] Automatic rollback on regression (no human loop)
- [x] Ships with self-modification **disabled by default**
- [x] ExecutionTrace for deterministic replay proofs

**Phase 3 exit criteria: ALL MET** ✅
- All 5 ADR 0002 rails have dedicated passing tests (45+ total)
- Ledger entries produced for every accept/reject event
- Self-modification remains off by default (config.enabled = false)
- End-to-end integration test proves all rails work together
- Tamper-detection verified, deterministic replay proven

**Code:** 2,600+ lines | **Tests:** 45+ cases | **Branch:** phase-3-ledger-engine

## Phase 4 — Training / Calibration Loop ✅ COMPLETE

**Completed 2026-07-09.** Certificate: [phases/PHASE_4_COMPLETE.md](phases/PHASE_4_COMPLETE.md).  
**Scope:** [ADR 0006](architecture/0006-phase4-calibration-task.md).

- [x] Real calibration task design (ADR 0006): doc-graph NodeKind classifier
- [x] Implementation: `ntg/calib/` + `phase4_calib` binary
- [x] Unit tests (features, fixtures, split, metrics, calibrate, ledger, self-mod)
- [x] End-to-end run on fixtures (hold-out)
- [x] End-to-end run on real `docs/` — **WIN** on balanced metrics (bal≈0.61)
- [x] Hold-out + class-imbalance handling
- [x] Optional topology self-mod probe (`--self-mod`, **off by default**, ledgered)
- [x] `docs/phases/PHASE_4_COMPLETE.md` + deep dive

**Phase 4 exit criteria: MET** — real task E2E + results recorded (win and residual F1 limits honest).

## Phase 5 — Optimization ✅ COMPLETE

**Completed 2026-07-09.** Certificate: [phases/PHASE_5_COMPLETE.md](phases/PHASE_5_COMPLETE.md).  
Prep inventory: [PHASE5_PREP.md](PHASE5_PREP.md).

- [x] Precision-oriented calib (code cues + flood-reject thr) — real docs
      **WIN** bal≈0.70 F1≈0.28 (up from Phase 4 bal≈0.61 F1≈0.18)
- [x] Drive production path through CalibModel → GraphNode (`score_via_graph_node`,
      `to_runtime_layer`, path identity tested)
- [x] CPU parallelization for hot path: `forward_native_parallel` (existing) +
      `batch_predict_parallel` / `batch_score_parallel`
- [x] Re-run density_bench + graph_overhead_bench; record in EXPERIMENTS.md
- [x] GPU path **explicitly re-scoped to Phase 6+** — 64-d calib tensors; CPU
      TOBL already 12–20× vs scalar; transfer cost not justified yet
- [x] `docs/phases/PHASE_5_COMPLETE.md`

**Phase 5 exit criteria: MET.**

## Phase 6 — Integration

- [ ] WASM target (for browser/edge deployment scenarios)
- [ ] Head-to-head benchmark against aethyro.com's current inference path
      on at least one live tier's real workload
- [ ] Decision point (not before): does this engine actually beat current
      production inference on memory and/or speed, on real hardware, on a
      real workload? If not, say so and figure out why before productizing.

## Phase 7 — Validation

- [ ] Full benchmark suite vs. standard baselines (a full-precision
      equivalent at minimum; BitNet b1.58 itself as a stretch comparison
      if feasible)
- [ ] Honest write-up of results — wins and non-wins alike — as an update
      to LITERATURE.md and/or a new ADR, not a marketing document

## Phase 8 — Productization / go-to-market decision

- [ ] Only now: decide how this ships through aethyro.com — as an
      efficiency upgrade to existing tiers, a premium sovereignty tier, a
      Legal/Healthcare launch differentiator, or some combination — based
      on what Phases 1-7 actually proved, not on assumption
- [ ] Revenue (if any) earmarked toward the founder's stated "AI
      Workstation" hardware goal, same discipline as Firmament ADR 0003
- [ ] No capability claim in aethyro.com copy ships ahead of the tested
      code that makes it true (same rule as Firmament ADR 0003, applied here)
