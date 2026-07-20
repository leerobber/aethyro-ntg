# Aethyro-NTG: Phase 1.2-1.3 + Phase 3 — Status After Correction

**Original date:** 2026-07-08
**Corrected:** 2026-07-19 (this rewrite)
**Scope:** SIMD/FFI ternary matmul core + mutation/ledger infrastructure

---

## Why this file was rewritten

The original version of this document (title: "Breakthrough Implementation
Summary", status: "MAJOR MILESTONE ACHIEVED") was written **before the code
it describes ever compiled**. When this branch was actually built and
tested on 2026-07-19:

- The crate had **19 compile errors**. Nothing in it had ever run.
- Once fixed to compile, **5 of its own tests failed**, including the two
  tests meant to prove the AVX2 SIMD kernel matched the scalar reference
  ("bit-parity") -- the AVX2 implementation actually returned all zeros or
  wrong values for every case those tests covered.
- The "self-tuning" dispatcher's profiling function was a hardcoded stub
  that always returned `passed_correctness: true` and `latency_us: 0.0`
  for every path, and its selection logic never read profiling data at
  all -- it just unconditionally preferred AVX2 if the CPU claimed
  support, with no correctness check.
- The ledger component's headline claim -- "tamper-evident," "suitable for
  regulated, air-gapped deployment" -- doesn't hold for what's actually
  implemented: unkeyed SHA-256 hash-chaining with no persistence and no
  external anchor, which anyone with write access could recompute from a
  tampered version and have it verify cleanly.

All of the above have since been fixed and verified by actually running
the code (see commit history on this branch, 2026-07-19). This document
now states only what's been verified. See "What changed" below for the
corrected picture, and "What was false and why" for what the original
version got wrong and why.

---

## Current, verified status

**Build:** `cargo build --release` — clean, 0 errors.
**Tests:** 163/163 passing, verified by running them:
- 142 unit tests (`cargo test --lib`)
- 11 tests in `phase1_2_3_simd_ffi.rs`
- 7 tests in `phase3_integration.rs`
- 3 tests in `self_parse.rs`

**AVX2 SIMD kernel:** Produces bit-identical results to the scalar
reference, verified across the original tests (which only use `k < 32`,
i.e. never actually exercise the vectorized loop) plus a new randomized
test that specifically covers `k >= 32` with multiple matrix shapes
(`avx2_matches_scalar_for_k_at_and_above_simd_width`).

**SIMD dispatcher:** Now actually profiles each available path (real
correctness check against scalar + real wall-clock timing) and selects
the fastest path that passed correctness, falling back to Scalar
otherwise. Previously this was cosmetic -- see below.

**Mutation ledger:** Renamed from `TamperEvidentLedger` to `MutationLedger`
to match what it actually does (see "Ledger: what's real" below). All 5
ADR 0002 safety-rail tests pass.

**No performance numbers are claimed here.** Correctness is verified;
actual speedup on real hardware has not been benchmarked and shouldn't be
asserted until it has been, on real hardware, with the numbers recorded.

---

## What was false in the original document, and why

### "Impact: 2-6x speedup on real hardware (measured honestly)"
Never measured. The AVX2 kernel that this claim was about didn't produce
correct output at all at the time this was written -- there was nothing
valid to benchmark. No speedup number should be stated until a real
benchmark is run against the now-correct implementation.

### "Bit-parity proven (scalar == SIMD byte-for-byte)"
Not proven at the time. The two tests that were supposed to prove this
(`avx2_matmul_simple`, `avx2_matches_scalar`) both use matrices with
`k < 32`, which never executes the AVX2 kernel's vectorized loop at all --
only a scalar tail path that didn't exist yet either. This is fixed now,
with a genuinely vectorized-path test added.

### "Self-Profiling Dispatch — Each path benchmarked automatically, Best
performer selected"
`profile_simd_path()` was a hardcoded stub (its own comment said "For
now, return a placeholder profile"). `profile_all()` computed a result
and discarded it (comment: "we'd ideally mutate here... use interior
mutability"). `select_best()` never consulted any profiling data --
it just hardcoded "prefer AVX2 if the CPU claims support." There was no
tuning signal anywhere in the loop. This is now real: `profile_simd_path`
does an actual correctness check and timing run, and `select_best` reads
the results.

### "Real Ledger (Not Theory) — Discovered false claim: 'ChronosLedger is
tamper-evident' → it's not / Built missing piece: genuine hash-chaining"
This document claims to have identified a false tamper-evidence claim in
a prior component and replaced it with a real one. What was actually
built has the same category of problem: unkeyed SHA-256 hash-chaining,
entirely in-memory, no persistence, no external anchor. Anyone who can
edit the in-memory structure can recompute the whole chain from their
edited version and it verifies fine, because there's no secret anywhere
in the scheme. This is genuinely useful for catching *accidental*
corruption or a bug that drops/reorders entries within one process run --
it is not tamper evidence against a deliberate adversary, and calling it
that (as the original doc, the ADR, and the type name `TamperEvidentLedger`
all did) was a real, substantive overclaim, not a wording nitpick.

### "Production-ready for regulated deployments" / "Regulatory
compliance? ✅ Ready" / "Air-gapped deployment? ✅ Ready"
No regulatory framework was consulted, nothing was deployed or audited,
and the underlying cryptographic properties (see above) don't support a
compliance claim regardless. Removed.

### "Dual-Objective Optimization... speedup = (baseline_lat / new_lat)
AND (baseline_mem / new_mem)" describing the SIMD dispatcher
This describes a feature that exists in the **mutation fitness
evaluator** (`FitnessScore` genuinely tracks latency and memory, and
`dominates()`/`is_improvement()` genuinely require both to hold -- this
part checks out, see below) but was misattributed here to the **SIMD
dispatcher**, which selects purely on latency. Corrected by removing the
claim from this context.

### "% test coverage: 100%" (Quality Metrics table, both phases)
No coverage tool (tarpaulin, llvm-cov, grcov, etc.) was ever run against
this code. "100%" was asserted, not measured. Removed.

### "Ready to Ship" / "Both branches are production-ready"
The code didn't compile. Removed.

---

## What was true, and is preserved

- **Dual-objective mutation fitness** (`FitnessScore { latency_us,
  memory_bytes }`, `dominates()`, `is_improvement()`) is real and tested.
  Memory is approximated as `node_count * 256 bytes` -- a heuristic, not
  a real allocator/RSS measurement -- which is a reasonable simplification
  but worth stating plainly rather than implying precise measurement.
- **Zero-copy FFI boundary** (no allocation inside the FFI calls,
  caller-owned buffers) -- verified by reading `ffi/mod.rs`; the FFI
  integration tests pass.
- **All 5 ADR 0002 safety rails** have dedicated passing tests: off by
  default, bounded budget, auto-rollback, deterministic replay, every
  mutation ledger-logged. Verified by running
  `phase3_integration.rs` — 7/7 pass.
- **Determinism / replay comparison** (`ExecutionTrace::compare`) is real
  and tested.
- **Self-modification disabled by default** (`SelfModConfig::enabled =
  false`) is real and tested.

---

## Code structure (accurate as of this rewrite)

```
aethyro-ntg/kernel/src/ntg/
├── ternary.rs           scalar reference matmul (the correctness oracle)
├── packed.rs             bit-packing storage
├── simd/
│   ├── mod.rs            dispatcher entry point
│   ├── dispatcher.rs      real profiling-driven path selection
│   ├── avx2.rs            AVX2 kernel, now correct + cross-validated
│   ├── neon.rs            NEON kernel (not exercised on this x86_64 build)
│   └── profiler.rs        real correctness + timing measurement
├── ffi/
│   ├── mod.rs, stats.rs, bindings.rs
├── graph.rs               topology
├── mutation/
│   ├── mod.rs, rules.rs, evaluator.rs, budget.rs
└── ledger/
    ├── mod.rs             MutationLedger (renamed from TamperEvidentLedger)
    ├── crypto.rs          SHA-256 primitives, corrected doc claims
    ├── chain.rs           CryptoChainLog (sequence self-consistency)
    ├── signed_entry.rs    per-record content hash
    ├── stateblots.rs      StateSlotStore (fixed a genesis-sentinel bug)
    └── replay.rs          ExecutionTrace
```

---

## Next steps

1. Run a real benchmark of the fixed AVX2 kernel against scalar on actual
   hardware, and record the honest number -- whatever it is.
2. Decide, deliberately, whether real tamper evidence (a keyed
   MAC/signature, or an external append-only anchor) is worth building,
   or whether "detects accidental corruption within one process run" is
   the actual design goal -- and document whichever is true.
3. `docs/architecture/0004-phase3-tamper-evident-ledger.md` needs the
   same correction pass as this file; not yet done as of this rewrite.
