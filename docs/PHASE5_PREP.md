# Phase 5 prep — options implemented now that pay off later

Built immediately after Phase 4 COMPLETE so Optimization work starts
from durable hooks, not greenfield rewrites.

## Shipped in this drop

| Option | Why it helps later |
|--------|-------------------|
| **`CalibModel` wire format + meta** | Reproducible weights/threshold; `# key=value` train notes; CI artifacts; A/B models |
| **`--write-model` / `--eval-model`** | Offline eval without retrain; deploy frozen models |
| **`--write-sparse` / `NTG_SPARSE_V1`** | COO dump for TOBL tooling without re-export code |
| **`--write-report`** | One-line JSON for EXPERIMENTS automation / dashboards |
| **`--compare-model`** | A/B two frozen models (epoch sweeps, feature experiments) |
| **`--predict` / `--sparse-score`** | Dense vs sparse TOBL path identity on live labels |
| **`--json` + `metrics_to_json`** | Machine-readable metrics for CI gates |
| **`to_sparse_weights()` / `to_graph_node()`** | Warm-start `GraphNode.weights` from calib |
| **`to_runtime_layer()`** | One-node Runtime ready for forward experiments |
| **`features_to_sparse()`** | Label → sparse activations for AccelManager path |
| **`score_label_sparse` / dense match** | Proves TOBL score == i8 score (tested) |
| **`GraphNode::from_ternary_weights`** | Generic dense→sparse node constructor |
| **`tools/dev.sh`** | `model`, `model-ab`, `check`, benches for LOQ/WSL |
| **CI: model roundtrip smoke** | Write/eval/predict/sparse artifacts on every push |
| **Capability v9** | Hosts feature-detect `phase4_calibration_supported` |

## CLI cheat sheet

```bash
cd kernel
cargo run --release --bin phase4_calib -- --docs ../docs --json \
  --write-model ../artifacts/models/ntg.calib \
  --write-sparse ../artifacts/models/ntg.sparse \
  --write-report ../artifacts/models/ntg.json

cargo run --release --bin phase4_calib -- --docs ../docs \
  --eval-model ../artifacts/models/ntg.calib

cargo run --release --bin phase4_calib -- \
  --eval-model ../artifacts/models/ntg.calib \
  --predict 'fn main() { }' --sparse-score

# A/B two models:
cargo run --release --bin phase4_calib -- --docs ../docs \
  --eval-model ../artifacts/models/a.calib \
  --compare-model ../artifacts/models/b.calib --json

# from repo root:
./tools/dev.sh all
./tools/dev.sh model
./tools/dev.sh model-ab
./tools/dev.sh check
```

## Status

**Phase 5 COMPLETE** — see [phases/PHASE_5_COMPLETE.md](phases/PHASE_5_COMPLETE.md).  
All hooks below were used in the Phase 5 delivery (precision + runtime path +
benches). GPU was re-scoped with measurement rationale.

## What Phase 6 should use first

1. Frozen `CalibModel` artifacts (`--write-model`) loaded by host / FFI / WASM  
2. `score_via_graph_node` / `batch_predict_parallel` as the product scoring API  
3. `--compare-model` for regression when features change  

## Still not done (correctly deferred)

- Rewrite storage encodings  
- Enable self-mod by default  
- Claim production readiness from fixture wins alone  
- GPU without a large-tensor profile justifying transfer cost 
