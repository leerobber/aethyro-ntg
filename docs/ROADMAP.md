# Aethyro NTG Engine — build roadmap

See [DESIGN.md](DESIGN.md) for the architecture these phases build, and
[ADR 0001](architecture/0001-vision-and-pivot.md) for why there's no
fixed calendar here — phases are gated on real green CI and real
measurements, not dates. T-shirt sizes below are rough estimates for
planning, not commitments.

**Non-negotiable rule across every phase:** docs updated + full CI green
before starting the next phase. No exceptions, no "we'll fix the tests
later." A phase that isn't green isn't done, regardless of how much code
exists for it.

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
- [ ] Record actual measured baseline (op count, wall-time on CI runner)

### 1.2 Bit-packing ✅ implemented (SIMD intrinsics still pending)
- [x] `PackedTernary`: 2 bits/value, 4 values/byte, `Result`-based
- [x] Tests: roundtrip, density claim (16 values -> 4 bytes, checked not
      asserted), non-multiple-of-4 lengths, out-of-bounds, invalid input
- [ ] `kernel/src/ntg/simd/mod.rs` — runtime feature-detected dispatch
      (`is_x86_feature_detected!`) over `PackedTernary`, AVX2 + NEON
      paths, always falling back to the scalar path. **Not done yet** —
      bit-packing alone isn't "SIMD," it's the storage format SIMD would
      operate on. Do not claim this item done until real intrinsics land.
- [ ] Test requirement: SIMD output must be bit-identical to the 1.1
      scalar reference on every existing test case, not just "close"
- [ ] Benchmark vs. 1.1 scalar baseline; record the real delta (or the
      honest absence of one) in this file

### 1.3 FFI + observability
- [ ] `#[no_mangle] extern "C"` surface for orchestrator integration
- [ ] `Stats` struct (op count, timing) feeding future ledger ingestion
- [ ] Memory-safety review of the FFI boundary specifically (this is
      where `unsafe` first enters the codebase — treat it accordingly)
- [ ] Integration test calling the FFI surface from a non-Rust caller

**Phase 1 exit criteria:** scalar, SIMD, and FFI paths all green in CI,
SIMD/FFI outputs proven bit-identical to the scalar reference, and a real
measured performance delta recorded (positive or not) before Phase 2
starts.

## Phase 2 — Graph Structure (+ SIS front-end, ADR 0003)

- [x] `kernel/src/ntg/graph.rs`: node/edge representation, `add_node`,
      `remove_node`, `add_edge`, `remove_edge` as first-class operations
- [x] Deterministic forward iteration order (`children()` is
      insertion-ordered) — proven under test
- [x] Typed nodes: `NodeKind::Content` / `NodeKind::Execution` (ADR 0003)
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
- [ ] Ledger module — **deliberately not built this pass.** ADR 0001
      already decided to reuse GH05T3's ChronosLedger rather than
      reinvent one; that code hasn't been examined/ported into this repo
      yet. Building a simplified stand-in now would violate that
      decision. This stays blocked on an actual port, tracked in Phase 3.
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
      shifts. Full experiment trail in
      [docs/EXPERIMENTS.md](EXPERIMENTS.md).
- [ ] Benchmark: forward-pass cost vs. an equivalent static (non-graph)
      ternary computation, to quantify the graph-structure overhead honestly
- [ ] Lazy leaf resolution: byte-exact content + precomputed per-glyph
      geometry fingerprint, materialized only when a leaf is read/executed
      (ByT5/CANINE + PIXEL-lite — see ADR 0003). **Not implemented** —
      docparse.rs stores leaf content as a plain `String` today; the
      glyph-fingerprint side-channel is still just a design in ADR 0003,
      not code. Do not claim otherwise.
- [ ] Byte-level cost mitigation (MrT5-style dynamic merging or
      equivalent) — measured, not assumed to be sufficient
- [ ] Execution-typed node runs are ledger-logged under the same ADR
      0002 rails as topology mutation — blocked on Phase 3's ledger

**Phase 2 exit criteria:** deterministic forward pass proven under test,
green CI, overhead cost measured and recorded, AND the ADR 0003 items
above have their own passing tests (typed nodes ✅, doc parsing ✅, path
parsing, lazy leaf resolution, measured byte-level cost mitigation) before
Phase 3 starts.

## Phase 3 — Self-Modification Engine (gated by ADR 0002)

- [ ] Port/adapt ChronosLedger's mmap binary format from GH05T3 into
      this repo as the audit ledger (see ADR 0002 rule 5) — do not design
      a new ledger format from scratch
- [ ] Rule-based mutation proposers (`AddNodeRule`, `RemoveEdgeRule`, etc.)
      as versioned, auditable artifacts (ADR 0002)
- [ ] Fitness evaluator using a real measured signal (task performance or
      resource cost on actual hardware) — not a proxy
- [ ] Bounded compute/time budget enforcement per modification cycle
- [ ] Automatic rollback on regression
- [ ] Ships with self-modification **disabled by default** — every ADR
      0002 rail must have a passing test before the default is even
      discussed, let alone flipped

**Phase 3 exit criteria:** all five ADR 0002 rails have dedicated,
passing tests; ledger entries are produced for every accept/reject
event; self-modification remains off by default at the end of this phase
regardless of how well it performs.

## Phase 4 — Training / Calibration Loop

- [ ] A real (not synthetic) small-scale training or calibration task to
      exercise the full stack: ternary core -> graph -> (optionally)
      bounded self-modification
- [ ] Report results the way `aetherflux-zero` already does in this
      founder's portfolio: a real measured win reported as a win, a real
      non-win reported honestly as a non-win — no exceptions

**Phase 4 exit criteria:** at least one full end-to-end run against a
real task, with results recorded regardless of outcome.

## Phase 5 — Optimization

- [ ] GPU path (if justified by Phase 1-4 measurements — not assumed
      necessary in advance)
- [ ] Parallelization across available CPU cores for the graph forward pass
- [ ] Re-run Phase 1-4 benchmarks post-optimization; record deltas

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
