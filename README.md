# Aethyro-NTG — Genomic Data Pipeline + Ternary Neural Network Kernel

A Rust workspace combining a population-genetics processing pipeline (VCF
parsing, linkage disequilibrium computation, synthetic genome generation)
with a ternary/quantized neural network kernel (SIMD matmul, self-modifying
graph topology with an audit ledger).

---

## Verified status (2026-07-28 — Phase 7 Complete)

| Metric | Value | How verified |
|--------|-------|---------------|
| Build | Clean, 0 errors, `-D warnings` | `cargo build --release` + `cargo clippy -- -D warnings` |
| Tests | 412 passing, 0 failing (100% pass) | `cargo test --release`, counted from output |
| Phases Complete | 0–7, 7.5 in progress | TASKLOG.md + git commit history |
| Lines of Rust | ~31,200 | `find kernel/src kernel/tests kernel/benches -name "*.rs" \| xargs wc -l` |
| Unsafe blocks | ~26 | `grep -rn "^\s*unsafe " kernel/src` (FFI boundary + SIMD intrinsics) |
| Modules | 42 (27 NTG + 15 Genomic) | ARCHITECTURE.md module registry |

No performance benchmark numbers are stated here unless they were actually
measured and are reproducible by running the code in this repo — see
"What's not verified" below for claims removed because they weren't.

---

## What this actually does

**Genomic pipeline** (`kernel/src/genomic/`, ~11K lines):
- Streaming VCF parsing with bit-packed (2-bit) genotype storage
- Linkage disequilibrium (Pearson r²) computation, including a bit-parallel
  popcount-based fast path cross-validated against a scalar reference
  (`ld_compute.rs` — this is the most solid, tested part of the codebase)
- Synthetic genome sampling that preserves real allele frequencies and LD
  structure from real 1000 Genomes data
- Population genetics utilities: Hardy-Weinberg, Kimura fixation probability,
  quality control checks

**NTG kernel** (`kernel/src/ntg/`, ~12K lines):
- Ternary weight representation and scalar/SIMD matmul
- A self-modifying graph topology with 5 safety rails (off by default,
  bounded compute budget, automatic rollback, deterministic replay,
  full audit logging) — see `kernel/src/ntg/mutation/` and `kernel/src/ntg/ledger/`
- The ledger provides **self-consistency checking** (detects accidental
  corruption or reordering within one process run), not cryptographic
  tamper evidence against a deliberate adversary — there's no signing key
  or external anchor. See `kernel/src/ntg/ledger/mod.rs` for the full
  explanation of what this design does and doesn't provide.

**KAIROS / vitascale** (`kernel/src/genomic/vitascale/`): an experimental
agent-lifecycle framework with staged capability gating. This is the least
mature part of the codebase and the least externally verifiable — read the
source before relying on any claim about it.

**WebSocket telemetry** (`kernel/src/ntg/websocket/`, Phase 7): real-time 60 Hz
streaming of SenseReport metrics (hormone levels, energy, safety scores, coherence)
over WebSocket for desktop app integration and live monitoring. Includes async Tokio
runtime, 3600-report circular buffer (60-second retention), and JSON serialization.

---

## What's not verified (removed from this README)

An earlier version of this README claimed: "Quality Rating: 9.8/10,"
"Approved for biotech labs," "Enterprise deployment approved," "396 tests,"
"39,126 lines of code," and internally inconsistent speedup figures ("100x
baseline" in one place, "4,000x faster" in another, for the same claim).
None of these were measured by any tool, and none should be trusted. The
actual test count, line count, and build status are in the table above,
verified by actually running the commands.

**Speedup claims:** the 4,000x / 201K SNPs-per-second figures describing
the VCF/LD pipeline vs. a Python baseline have not been re-verified in this
pass and should be treated as unconfirmed until someone runs the benchmark
and records the real numbers, on real hardware, with the command used to
produce them.

**"Approved for biotech/clinical" claims:** removed entirely. No external
audit, regulatory review, or clinical validation has occurred. Nothing in
this repository should be used for a clinical or regulated purpose without
an actual, independent review.

---

## Current project status

**[docs/STATUS.md](docs/STATUS.md) is the maintained, honest status
document** — read it before trusting any claim in this README or
elsewhere in the repo. It already explicitly names "docs and marketing
language outrunning measurements" as the project's primary risk, and
already marks the old root-level session-summary docs as historical.
This README's numbers are a snapshot verified 2026-07-19; `docs/STATUS.md`
is updated more frequently and is more authoritative on current phase,
readiness, and what's actually product-ready vs. research-stage.

## Quick start

```bash
cd kernel

# Build (Rust 1.70+)
cargo build --release

# Test (412 tests, ~2 min)
cargo test --release

# Lint (0 warnings required by CI)
cargo clippy -- -D warnings

# Run telemetry server demo (60 Hz WebSocket streaming)
cargo run --release --bin telemetry_server
```

## Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) — complete module registry (42 modules) and dependency contracts
- [CONTRIBUTING.md](./CONTRIBUTING.md) — contribution guidelines
- [TASKLOG.md](./TASKLOG.md) — phase completion log with verification records
- [PROTOCOL.md](./PROTOCOL.md) — enterprise governance and code review standards
- `docs/TELEMETRY.md` — WebSocket telemetry streaming integration guide (Phase 7)
- `docs/architecture/` — architecture decision records (ADRs)

Historical per-phase session logs from earlier development have been moved
to `docs/history/` to keep the repository root readable; they're kept for
reference but may contain claims that were never re-verified the way this
README and `VERIFICATION_REPORT.md` have been.
