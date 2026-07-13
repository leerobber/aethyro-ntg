# 0009: SovereignBrain multi-organ stack + multi-axis fitness

**Status:** Accepted (implemented + measured; hardening in progress).  
**Depends on:** ADR 0002 (safety rails), 0003 (SIS), 0004 (ledger), 0006 (calib), Phase A–E genomic pipeline, ADR 0008 Phase F recommendations.  
**Code:** `kernel/src/genomic/sovereign_brain.rs`, `sovereign_fitness.rs`, `language_organ.rs`, `ntg/mutation/multi_axis.rs`, `bin/sovereign_*`.

## Decision

Adopt a **SovereignBrain** as the multi-chromosome cognitive substrate for the genomic stack, with:

1. **Working set** (bounded active addresses) + **LTM motifs** (unbounded session / durable store).
2. **LanguageOrgan** (SIS/docparse graph + optional Phase 4 calib) co-activated via `activate_from_text`.
3. **Multi-axis fitness** for selection: task ↑, structural cost ↓, biological consistency ↑, safety ↑.
4. **Real axes** preferred over structure proxies: Phase D references + LD coverage, calib holdout, ledger verify.
5. **Self-mod off by default** outside explicit research selection loops; every accept/reject ledger-logged when using `SovereignFitnessContext`.

## Consequences

- Product hosts must not enable open-ended topology mutation without multi-axis gates + ledger.
- Biology freeze-at-ingest is a **relative** fidelity metric (coverage under prune), not proof of population genetics correctness alone.
- Further organs should implement the shared `Organ` trait rather than one-off APIs.
- STATUS.md and EXPERIMENTS.md are authoritative for measured claims.

## Measured evidence (see EXPERIMENTS.md)

- Synthetic: train accepts, prune rejects under real bio; utility rises with KAIROS.
- Real multi-chr VCF campaign (chr22+chr1): same accept/reject pattern; ledger verifies.
- Language calib fixtures: test_bal often >0.9 on fixture holdout (not a production claim).
