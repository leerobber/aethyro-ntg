# Phase 4 curriculum — Calibration loop

## Learning objectives

1. Train ternary NodeKind classifier on real train docs.
2. Generalize to holdout docs (balanced accuracy).
3. Discriminate code-like vs prose labels.
4. Persist model via NTG_CALIB_V1 wire format.

## Data (real)

| Split | Source |
|-------|--------|
| Train | ~80% of live engineering docs, **stratified by fenced code** so Execution labels appear in both splits |
| Holdout | ~20% remainder (same stratification) |
| Labels | Parser `NodeKind` only — structural ground truth |
| Excluded | entire `docs/schooling/**` (curriculum + generated runs — no self-train pollution) |

## Teaching / learning

- `train_model_full(samples, 50)` on **all** train-split documents (no internal hold-out leak).
- Record thr, train bal/f1/rec/prec, nonzero weights.

## Advanced exam (composite grade)

| Weight | Skill |
|-------:|-------|
| 30% | Holdout bal quality mapped from [0.50→0.75] → [0→1] |
| 15% | Holdout F1 quality mapped /0.30 |
| 20% | Code-like → Execution |
| 15% | Prose → Content |
| 20% | Item pass rate (bal lift≥0.55, rec≥0.15, wire, schema, flood guard) |

Composite must be **≥ 75%**. Hard item bar for bal is **≥ 0.55** (clear lift vs majority 0.50), not raw 0.75 — on ~2% minority class raw bal≥0.75 is a stretch goal tracked via the quality map.

## Pass / fail

Composite &lt; 75% ⇒ FAIL full redo (retrain + re-exam).
