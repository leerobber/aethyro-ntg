# Aethyro-NTG — Genomic Data Pipeline + Ternary Neural Network Kernel

A Rust workspace combining a population-genetics processing pipeline (VCF
parsing, linkage disequilibrium computation, synthetic genome generation)
with a ternary/quantized neural network kernel (SIMD matmul, self-modifying
graph topology with an audit ledger).

---

## Verified status (2026-07-28)

| Metric | Value | How verified |
|--------|-------|---------------|
| Build | Clean, 0 errors, clippy `-D warnings` passes | `cargo build --release && cargo clippy --release -- -D warnings` |
| Tests | 493 passing, 0 failing | `cargo test --release`, counted directly from output |
| Lines of Rust (kernel) | ~32,400 | `find kernel/src kernel/tests kernel/benches -name "*.rs" \| xargs wc -l` |
| Unsafe blocks | ~26 | `grep -rn "^\s*unsafe " kernel/src` (FFI boundary + SIMD intrinsics) |
| Phase Status | Phase F (L0) COMPLETE, Deployment infrastructure ready | Phase F self-awareness telemetry + GCP backend |

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

**KAIROS / VITASCALE** (`kernel/src/genomic/vitascale/`): agent-lifecycle framework
with staged capability gating (Zygote → Neonate → ... → Adult). Phase F complete:
- Self-awareness telemetry: endocrine model (8 hormones), regime detection, emotional state
- Lifecycle transitions: Adulthood gate validation, mutation authorization
- Agent hierarchy: 4-tier structure supporting 10K–500K agents
- Benchmarking: throughput/latency measurement for large-scale populations

**Hostframe backend** (`hostframe-backend/`): Production GCP Cloud Run service for
telemetry ingestion. BigQuery integration with materialized views for dashboards.
See `hostframe-backend/README.md` for deployment instructions.

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

### Build and test (kernel)

```bash
cd kernel
cargo build --release     # Build
cargo test --release      # Run test suite (493 tests)
cargo clippy --release -- -D warnings  # Lint (must pass clean)
```

### Deploy Hostframe backend (optional)

```bash
cd hostframe-backend
./scripts/deploy.sh <gcp-project> [gcp-region]
# Or manually:
# 1. docker build -t gcr.io/<project>/hostframe-backend:latest .
# 2. docker push ...
# 3. cd terraform && terraform apply
```

## Documentation

### Core
- [CLAUDE.md](./CLAUDE.md) — project handoff & current phase status
- [ARCHITECTURE.md](./ARCHITECTURE.md) — system design
- [CONTRIBUTING.md](./CONTRIBUTING.md) — contribution guidelines
- `docs/architecture/` — architecture decision records (ADRs)

### Phase F: Self-Awareness & Deployment
- [kernel/src/genomic/vitascale/](./kernel/src/genomic/vitascale/) — KAIROS lifecycle & telemetry
- [hostframe-backend/README.md](./hostframe-backend/README.md) — Telemetry ingestion service
- [hostframe-backend/terraform/](./hostframe-backend/terraform/) — GCP infrastructure as code

Historical per-phase session logs from earlier development have been moved
to `docs/history/` to keep the repository root readable; they're kept for
reference but may contain claims that were never re-verified the way this
README and `VERIFICATION_REPORT.md` have been.
