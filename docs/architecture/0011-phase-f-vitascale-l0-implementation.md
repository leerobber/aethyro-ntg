# 0011: Phase F — VITASCALE L0 Implementation

| Field | Value |
|-------|-------|
| **Status** | Accepted — implementation in progress |
| **Date** | 2026-07-27 |
| **Depends on** | ADR 0010 (VITASCALE Hostframe spec), Phases 0–6.18 COMPLETE |
| **Implements** | ADR 0010 §4.1 L0 (invent-now software layer) |
| **Honesty rule** | Engineering host on iron — no claim of biological life or consciousness |

---

## Context

ADR 0010 specified the VITASCALE Hostframe architecture in 1292 lines of spec.
Phases 0–6.18 completed the foundational substrate (ternary kernel, Quad-Brain, safety rails,
audit ledger, Pulsewire/life-course/KAIROS Stage-0 scaffolding).

Phase F ships the ADR 0010 L0 layer as working Rust code with green tests.

---

## What Phase F Delivers (L0 only)

| Component | Module | What it does |
|-----------|--------|-------------|
| `TissueLive` | `organ_live.rs` | Extends `Tissue` with bus-signal hook + health probe |
| `Sensefield` + `NeuroSignal` | `awareness_bus.rs` | In-process pub/sub with bounded queues and drop-with-counter |
| `NeurocyteHandle` + `NanoBrain` | `nano_agent.rs` | Control handle for every live tissue; round-robin `tick` scheduling |
| `Crown` + `CrownView` | `assembly.rs` | Hierarchical tissue owner; single `SovereignFitnessContext`; Gestalt |
| `PressureMesh` + `federate()` | `fitness_federation.rs` | Aggregates nano reports for dashboard + immune veto only |
| `Oculus` | `oculus_organ.rs` | Multi-stream awareness fusion; coverage SLA |
| `Phageguard` | `immune_organ.rs` | Detect + quarantine (Load vs Deterministic); ledger phage events |
| `IronChassis` | `body/host_cpu.rs` | L0 CPU body adapter; stubs for L2/L3 |
| `vitascale_demo` | `bin/vitascale_demo.rs` | End-to-end demonstration binary |
| Ledger extensions | `ntg/ledger/` | `log_phage_event`, `write_agent_slot`, `agent_lineage` |

## What Phase F Does NOT Deliver (L1+ deferred)

| Deferred | Reason |
|----------|--------|
| Homeostasis controller | L1 — needs L0 bus green first |
| Omniradar HTML dashboard | L1 — cold-path consumer of L0 bus |
| eBPF optional reader | L1 — Linux-only feature-gated |
| Snapshot restore (Phageguard repair v1) | L1 — checkpoint dirs need L0 quarantine first |
| Neuromorphic / wetware chassis | L2/L3 — no hardware |
| Parallel Neurocyte ticks | L1+ — shared `&mut` requires MPSC ring |

---

## Key Decisions (from ADR 0010)

**KD13 — No process-global ring.** `SovereignFitnessContext` holds optional `PulseHandles`; zero
cost when `None`.

**KD14 — Crown owns tissues.** `NeurocyteHandle` is a control handle, not a second tissue owner.

**KD15 — Single selection authority.** `SovereignFitnessContext::select_*` is the only path that
scores axes, accepts/rejects mutants, and writes the selection ledger. `PressureMesh` surfaces veto
and ephemeral bias only.

**KD16 — Observability/determinism split.**
- `selection_veto` ← only Deterministic detectors (ledger fail, safety=0, utility regression, test inject)
- Load quarantine ← ring pressure, heartbeat silence — freezes proposals only, never blocks `select_*`

---

## Test Strategy

All L0 components get unit tests. Key scenarios:
- `Sensefield` bounded-queue drop counting
- `NeurocyteHandle` round-robin tick ordering
- `Crown` owns one `SovereignFitnessContext`; double-score is impossible
- `Phageguard` Deterministic vs Load quarantine class split
- `Oculus` coverage formula: `count(streams on SLA) / count(registered)`
- `federate()` → veto only from Deterministic Phageguard; stress from reports
- Ledger `log_phage_event` → `verify_full_ledger()` still Ok
- `vitascale_tick` does not alter `ctx.evaluator` permanently when bias applied

---

## PR Sequence

| PR | Scope |
|----|-------|
| This PR | All L0 components + demo bin + ledger extensions + tests |
| Follow-up (L1) | Homeostasis + Omniradar dashboard + checkpoint restore |
| Follow-up (L2) | Neuromorphic chassis adapter |
