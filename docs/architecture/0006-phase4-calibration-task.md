# 0006: Phase 4 calibration task — doc-graph node kind classifier

**Status:** Accepted + **implemented** (2026-07-09).  
**Phase:** 4 COMPLETE — see `docs/phases/PHASE_4_COMPLETE.md`.  
**Depends on:** Phases 0–3 COMPLETE certificates.

## Context

Phase 4 requires a **real** (not toy-only) small-scale task that exercises:

ternary core → graph structure → measured score → (optional) ledgered change  

without claiming product-ready intelligence.

## Decision

### Task

**Name:** Doc-graph `NodeKind` classifier  

**Input:** Markdown documents parsed with `docparse` into a `Graph`  
(real repo docs under `docs/` when present; fixtures otherwise).  

**Label:** Each node’s true `NodeKind` — `Execution` (fenced code) vs
`Content` (everything else). Labels come from the parser (ground truth
from structure, not hand labels).  

**Features (deterministic, offline):**
1. `encode_fixed` ternary of UTF-8 label bytes (padded/truncated to K=64)
2. Coarse counts from `LeafSignal` folded into ternary slots
3. `GlyphFingerprint` fields folded into ternary slots  

**Model:** Ternary weight vector `w ∈ {-1,0,1}^K` with score  
`s = Σ w_i · x_i`. Predict `Execution` if `s ≥ threshold` else `Content`.  

**Baseline:** Always predict `Content` (majority class on real docs).  

**Calibration:** For each epoch, for each node: if misclassified, update  
`w_i ← clamp(w_i + x_i, -1, 1)` (perceptron-style ternary).  

**Self-mod:** Off by default. Optional ledger log of weight snapshots
via existing ledger (audit only, not topology mutation in v1).  

### Metrics

| Metric | Definition |
|--------|------------|
| accuracy | correct / total nodes |
| baseline_accuracy | majority Content |
| delta | after − baseline |
| latency_us | wall-clock for train+eval |
| win | `delta > 0` and after > baseline |

### Non-goals (Phase 4 v1)

- Learned PIXEL, GPU/NPU  
- Topology self-mod (can be Phase 4.1 after weight calib works)  
- Production aethyro.com head-to-head  

## Consequences

- Module: `kernel/src/ntg/calib/`  
- Binary: `phase4_calib`  
- Results: always written to EXPERIMENTS.md regardless of win/loss  
- Phase 4 COMPLETE only when exit criteria + PHASE_4_COMPLETE.md exist  

## Sources

ROADMAP Phase 4; PHASE_GATE_PROTOCOL.md; ADR 0001 combination bet.
