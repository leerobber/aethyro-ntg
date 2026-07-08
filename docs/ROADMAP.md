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

### 1.1 Scalar reference ✅ implemented, pending CI verification
- [x] `Ternary` enum, `encode()` (absmean threshold), `matmul_scalar()`
- [x] `NtgError` — `Result`-based, no panics on bad input
- [x] Unit tests: roundtrip, invalid-value rejection, threshold
      separation, empty input, zero-vector matmul, hand-computed matmul
      reference, shape-mismatch rejection
- [ ] Confirm green on GitHub Actions (this sandbox has no local
      cargo/rustc — CI is the real gate, same as Firmament)
- [ ] Record actual measured baseline (op count, wall-time on CI runner)
      in this file once confirmed green

### 1.2 Portable SIMD
- [ ] `kernel/src/ntg/simd/mod.rs` — runtime feature-detected dispatch
      (`is_x86_feature_detected!`), always falling back to 1.1's scalar path
- [ ] AVX2 path, NEON path
- [ ] Bit-packing: 4 ternary values per byte (2 bits each)
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

## Phase 2 — Graph Structure

- [ ] `kernel/src/ntg/graph.rs`: node/edge representation, `add_node`,
      `remove_node`, `add_edge`, `remove_edge` as first-class operations
- [ ] Deterministic forward pass over a fixed topology
- [ ] Property tests: same topology + same input -> same output, every
      time, across repeated runs
- [ ] Benchmark: forward-pass cost vs. an equivalent static (non-graph)
      ternary computation, to quantify the graph-structure overhead honestly

**Phase 2 exit criteria:** deterministic forward pass proven under test,
green CI, overhead cost measured and recorded.

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
