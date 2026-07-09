# NTG Schooling Protocol (binding)

**Status:** Binding learning process for doctorate-style mastery of Phases 0–5.  
**Pass bar:** **75.0%** on each phase advanced exam.  
**Fail rule:** score **&lt; 75%** ⇒ **FAIL — full redo** (restudy + re-exam), up to max attempts.

## Principles

1. **Real data only.** No mythical corpora. Sources:
   - Live `docs/**/*.md` on disk
   - Closed-form ternary matmul / encode problems with pen-verified answers
   - Real repo paths and COMPLETE certificates
   - Live kernel APIs (ledger, calib, GraphNode)
2. **Study then exam.** Every phase has a teaching pass (practice / train split) before the advanced holdout exam.
3. **Religious documentation.** Every item pass/fail, attempt number, dataset id, and aggregate stats are written to notebooks under `docs/schooling/runs/`.
4. **Steady campaigns.** Default **5 independent runs** so mean/min/max are evidence, not a one-shot fluke.
5. **No soft mastery.** Failing a phase after max redos is recorded as FAIL; do not claim the phase is mastered.

## Run command

```bash
cd kernel
cargo run --release --bin ntg_school -- \
  --docs ../docs \
  --out ../docs/schooling/runs \
  --runs 5 \
  --max-attempts 5
```

Single phase:

```bash
cargo run --release --bin ntg_school -- --docs ../docs --phase 1 --runs 3
```

## Outputs

| Artifact | Path |
|----------|------|
| Master notebook | `docs/schooling/runs/MASTER_NOTEBOOK.md` |
| Per-run notebook | `docs/schooling/runs/RUN_0N_NOTEBOOK.md` |
| Per-run JSON | `docs/schooling/runs/RUN_0N_results.json` |
| Dataset manifest | `docs/schooling/runs/DATASET_MANIFEST.md` |
| Curriculum | `docs/schooling/curriculum/PHASE_N.md` |

## Scoring

- **Item exams (phases 0–3):** score = items_passed / items_total.
- **Composite exams (phases 4–5):** weighted blend of holdout metrics + skill checks (see curriculum). Still must be ≥ 75%.

## Redo policy

On FAIL:

1. Re-run full **study** for that phase (no skipping).
2. Re-run full **advanced exam**.
3. Increment attempt counter (documented).
4. Stop after `max_attempts` (default 5) and record permanent FAIL for that campaign run.

## Relation to phase COMPLETE certificates

Schooling is **orthogonal** to build COMPLETE certificates:

- COMPLETE certs = engineering deliverables shipped.
- Schooling = measured mastery on real data after the fact (and ongoing).

A phase can be COMPLETE as engineering and still FAIL schooling if the exam bar is not met — that is intentional; fix features/data and re-run school.
