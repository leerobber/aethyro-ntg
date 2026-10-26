# Aethyro-NTG — Genomic Data Pipeline + Ternary Neural Network Kernel

A Rust workspace combining a population-genetics processing pipeline (VCF
parsing, linkage disequilibrium computation, synthetic genome generation)
with a ternary/quantized neural network kernel (SIMD matmul, self-modifying
graph topology with an audit ledger).

---

## Verified status (2026-10-26)

| Metric | Value | How verified |
|--------|-------|---------------|
| Build | Clean, 0 errors; clippy `-D warnings` clean on all targets | `cargo build --release && cargo clippy --all-targets -- -D warnings` |
| Tests | <!--tests-->670<!--/tests--> passing, 0 failing, 0 ignored | `cargo test --release`, summed from the test log; CI fails if this number drifts (`scripts/check-doc-counts.sh`) |
| Lines of Rust (kernel) | 34,965 src + 1,615 tests + 194 benches | `find kernel/src kernel/tests kernel/benches -name "*.rs" \| xargs wc -l` |
| Unsafe | 19 `unsafe` blocks + 5 `unsafe fn` in `kernel/src` | `grep -rnE "unsafe\s*\{" kernel/src` (FFI boundary + SIMD intrinsics) |
| Whole stack | dashboard + kernel_host + keymaster smoke passes | `scripts/smoke-whole.sh`; also the `whole-stack` CI job |
| Phase Status | Phase F (L0) IN PROGRESS: scaffold complete, no verified live deployment | Phase vocabulary is defined in [CLAUDE.md](CLAUDE.md); full measurements in [docs/BASELINE_2026-10-26.md](docs/BASELINE_2026-10-26.md) |

No performance benchmark numbers are stated here unless they were actually
measured and are reproducible by running the code in this repo — see
"What's not verified" below for claims removed because they weren't.

---

## What this actually does

**Genomic pipeline** (`kernel/src/genomic/`, ~11K lines):
- Streaming VCF parsing with bit-packed (2-bit) genotype storage via `Source` trait abstraction
- Linkage disequilibrium (Pearson r²) computation, including a bit-parallel
  popcount-based fast path cross-validated against a scalar reference
  (`ld_compute.rs` — this is the most solid, tested part of the codebase)
- Synthetic genome sampling that preserves real allele frequencies and LD
  structure from real 1000 Genomes data
- Population genetics utilities: Hardy-Weinberg, Kimura fixation probability,
  quality control checks
- Unified `Brain` trait enabling swappable brain implementations (ChromosomeBrain, SovereignBrain)

**NTG kernel** (`kernel/src/ntg/`, ~12K lines):
- Ternary weight representation and scalar/SIMD matmul
- Ternary x ternary (popcount) is **not faster** than a SIMD f32 GEMM
  (0.15x to 1.05x). Ternary weights with int8 activations
  (`ntg::storage::ternary_i8_gemm`, 2-bit packed, AVX-512 VNNI / AVX2) are 3x to
  24x faster than the best f32 on one core, at a 2 to 7 point accuracy cost on a
  small trained network. On the real pretrained BitNet b1.58 2B-4T it keeps
  accuracy (perplexity within 0.15%, ARC-Easy unchanged within noise) and runs
  the decode matmuls 7.7x to 8.5x faster than bf16 on one core; prefill is
  slower than torch. Against `bitnet.cpp` on the same VM the whole-model decode is 1.3x (one thread) to 3.5x (four) slower here, with kernel-level parity unknown (`docs/results/bitnet_cpp/`). One shared
  VM, caveats in `docs/EXPERIMENTS.md` (2026-10-26). The earlier "143.6x" was
  against a naive scalar loop.
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
This README's numbers are a snapshot verified 2026-10-26 (see
[docs/BASELINE_2026-10-26.md](docs/BASELINE_2026-10-26.md)); the test count is
checked against real test output in CI. `docs/STATUS.md` is more authoritative
on readiness and what's actually product-ready vs. research-stage.

## Quick start

### Build and test (everything)

```bash
scripts/build-all.sh          # kernel + keymaster + dashboard + hostframe: build, test, clippy
scripts/smoke-whole.sh        # start dashboard + kernel_host + keymaster, hit the API
```

### Kernel only

```bash
cd kernel
cargo build --release
cargo test --release                    # <!--tests-->670<!--/tests--> tests
cargo clippy --all-targets -- -D warnings
```

### Deploy Hostframe backend (optional; no verified live deployment yet)

Shortest path to a verified deployment: [docs/DEPLOY.md](docs/DEPLOY.md) (`scripts/verify-deploy.sh <url>` checks it from outside). Heavier GCP route below.

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
