# Aethyro NTG Engine — technical design

See [ADR 0001](architecture/0001-vision-and-pivot.md) for why this exists
and [LITERATURE.md](LITERATURE.md) for what's proven vs. what's this
project's own claim. This document describes the architecture the phases
in [ROADMAP.md](ROADMAP.md) build toward. Nothing below is implemented
except where a phase in the roadmap is marked done — check ROADMAP.md
for current status, don't assume this document describes shipped code.

## Layers

```
┌─────────────────────────────────────────────────────────┐
│  aethyro.com platform (existing, live)                   │
│  Personal / CPA / Dev / Research tiers + Legal/Healthcare │
│  (waitlist-only today)                                    │
└───────────────────────┬───────────────────────────────────┘
                         │ FFI / C ABI (Phase 1.3) + tools/ingest.py
┌───────────────────────┴───────────────────────────────────┐
│  Native Runtime (implemented)                             │
│  - Runtime::forward_native_parallel + AccelManager        │
│  - GraphNode.weights: SparseBitSlicedTernary              │
└───────────────────────┬───────────────────────────────────┘
┌───────────────────────┴───────────────────────────────────┐
│  Self-Modification Engine (Phase 3, ADR 0002) — OFF default│
│  - rule-based topology mutation proposals                 │
│  - fitness (latency + memory) + budget + rollback         │
│  - accept/reject ledger-logged                            │
└───────────────────────┬───────────────────────────────────┘
┌───────────────────────┴───────────────────────────────────┐
│  Graph Structure (Phase 2)                                │
│  - structural Node + edges + adj_list                     │
│  - docparse / pathparse / fsevents / leafsignal           │
│  - Graph::forward_pass (LeafSignal aggregate)             │
└───────────────────────┬───────────────────────────────────┘
┌───────────────────────┴───────────────────────────────────┐
│  Ternary storage + compute (Phase 1)                      │
│  - scalar golden matmul_scalar                            │
│  - PackedTernary / BitSliced / SparseBitSliced            │
│  - SIMD dispatcher + TOBL FFI                             │
└─────────────────────────────────────────────────────────┘
        │
        └── TamperEvidentLedger (Phase 3)
            SHA-256 chain + SignedEntry + StateSlotStore + ExecutionTrace
            (mmap file format / full ChronosLedger parity still optional)
```

**Current implementation truth:** see [STATUS.md](STATUS.md).

## Ternary Tensor Core (Phase 1)

**Encoding.** Weights are quantized to `{-1, 0, +1}` using an
absmean-style threshold: `threshold = mean(|w|) * 0.5`, then
`w > threshold -> +1`, `w < -threshold -> -1`, else `0`. This mirrors the
*shape* of BitNet b1.58's quantization (scale the cutoff by the tensor's
own average magnitude) without claiming to replicate Microsoft's exact
`BitLinear` training procedure — this project has not (yet) attempted
native ternary training, only post-hoc quantization for the reference
implementation. That distinction should stay explicit in any future
claim about training vs. inference.

**Storage.** Phase 1.1 stores one `i8` per value (simple, easy to test
against). Phase 1.2 moves to bit-packing (2 bits per ternary value, 4
values per byte) for the actual memory-density win — the whole point of
choosing ternary. Every packed representation must produce bit-identical
results to the Phase 1.1 scalar reference; that equivalence is itself a
required test, not an assumption.

**Compute.** `matmul_scalar` is the golden reference: naive triple-loop,
f32 accumulation, deterministic (no RNG, no floating-point-order
nondeterminism across runs). Phase 1.2 adds an `AVX2`/`NEON` SIMD path
behind runtime feature detection (`is_x86_feature_detected!`), always
falling back to the scalar path on unsupported hardware — portability
before optimization, matching CONTRIBUTING.md's "measure, don't assume."

**Errors.** Library functions return `Result<_, NtgError>`, they do not
panic on malformed input (shape mismatches, invalid ternary values).
Panicking library code is a production liability; this was an explicit
fix versus an earlier external draft that used bare `assert_eq!` inside
library functions.

## Graph Structure (Phase 2)

A directed graph of compute nodes (each backed by a `TernaryTensor`
operation) connected by edges representing data flow. Two properties
matter more than raw expressiveness at this stage:

- **Deterministic forward pass.** Given a fixed topology and fixed
  input, output must be reproducible — required for ADR 0002's replay
  guarantee once topology becomes mutable.
- **Structural mutation as first-class operations.** `add_node`,
  `remove_node`, `add_edge`, `remove_edge` are the primitive operations
  the self-modification engine (Phase 3) calls — they are not an
  afterthought bolted onto a static-graph design.

This phase deliberately does not attempt novel graph *learning*
algorithms — it borrows directly from the published dynamic-topology
research in LITERATURE.md rather than inventing new graph theory. The
novelty budget for this project is spent on the *combination*
(ternary + dynamic topology + audit ledger), not on any one layer being
independently new.

## Self-Modification Engine (Phase 3)

A rule-based mutation proposer (e.g., `AddNodeRule`, `RemoveEdgeRule`)
generates a candidate topology change. Every candidate is evaluated
against a **real, measured fitness signal** — actual task performance or
actual resource cost on target hardware, never a cheap proxy — before
acceptance. This entire subsystem is gated by every rule in
[ADR 0002](architecture/0002-safety-rails-for-self-modification.md): off
by default, bounded budget per cycle, automatic rollback on regression,
deterministic replay, and every event logged to the audit ledger.

## Audit Ledger

**Corrected 2026-07-08** (see [ADR 0002](architecture/0002-safety-rails-for-self-modification.md)
and [docs/EXPERIMENTS.md](EXPERIMENTS.md) for the full finding): this
section previously claimed GH05T3's ChronosLedger was already a
"32-byte mmap binary format, hash-chained for tamper-evidence." Reading
the actual source found that's only half true — ChronosLedger really is
a 32-byte mmap format, but it is a real-time **mutable** agent-state
store (slots overwritten in place), with no hashing and no
tamper-evidence at all. The real ledger combines three pieces:
ChronosLedger's state-slot model (fast state, `parent_offset` lineage),
`LexGenSeal`'s per-record SHA256 signing approach
(`backend/oss/core/seal.py` — real, but not chained across records),
and `kernel/src/ntg/chain.rs`'s `ChainLog` (the chaining/sequence-
integrity piece that didn't exist anywhere until it was built here).
Every accepted or rejected modification, with its fitness score and
resource cost, becomes a ledger entry once Phase 3 wires these three
together — not a new format invented from scratch, but not a drop-in
port of ChronosLedger alone either.

## FFI / Observability (Phase 1.3+)

A C ABI surface (`#[no_mangle] extern "C"`) for orchestrator integration,
plus a `Stats`/`TernaryCapability`-style struct reporting what's
available (scalar-only vs. SIMD, version) and what happened (op counts,
timing) — feeding the ledger without adding hidden side effects to the
compute path itself.

## Where aethyro.com fits (Phase 6+)

Per [ADR 0001](architecture/0001-vision-and-pivot.md), this engine's
first real product target is the existing, *live* aethyro.com tiers
(Personal, CPA, Dev, Research) as an efficiency upgrade — not a new
vertical's greenfield sales motion. Integration means: the current
inference path those tiers use gets an NTG-backed option, benchmarked
head-to-head on real hardware, shipped only if the comparison is
genuinely favorable (and reported honestly if it isn't). The
provable-sovereignty/audit-ledger story becomes the pitch for a premium
tier and for the still-unbuilt Legal/Healthcare "coming soon" slots,
once there's a real, tested capability behind that pitch — not before.
