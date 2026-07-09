# Phase 5 curriculum — Optimization & production path

## Learning objectives

1. Dense score ≡ GraphNode sparse score (production path).
2. Batch parallel predict ≡ serial.
3. Runtime warm-start from CalibModel.
4. Holdout balanced quality still ≥ bar after optimization path.

## Data (real)

Same train/holdout docs corpus as Phase 4 (real markdown).

## Teaching / learning

- Phase 4 calibrate on train.
- Practice `score_via_graph_node` and `to_runtime_layer`.

## Advanced exam (composite)

| Weight | Skill |
|-------:|-------|
| 35% | Path identity rate on holdout previews |
| 25% | Holdout balanced accuracy |
| 20% | Batch parallel identity |
| 20% | Item pass rate (sparse path, runtime, capability v10+) |

Composite must be **≥ 75%**.

## Pass / fail

Composite &lt; 75% ⇒ FAIL full redo.
