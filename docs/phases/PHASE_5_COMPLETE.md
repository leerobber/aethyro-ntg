# Phase 5 COMPLETE Certificate

**Sign-off date:** 2026-07-09  
**Phase N+1 may begin:** **YES** (Phase 6 — Integration)

## Scope

Optimization (ROADMAP Phase 5): precision-oriented calibration, production
path from frozen `CalibModel` into `GraphNode` / Runtime, CPU parallel batch
scoring, re-measured benches. GPU deferred with measurement rationale.

## Deliverables map

| Item | Path |
|------|------|
| Precision features + thr | `kernel/src/ntg/calib/mod.rs` (indent/line-shape cues, flood-reject thr) |
| GraphNode warm-start | `CalibModel::to_graph_node`, `score_via_graph_node`, `GraphNode::from_ternary_weights` |
| Runtime layer hook | `CalibModel::to_runtime_layer` |
| Parallel batch score | `batch_predict_parallel` / `batch_score_parallel` |
| Model I/O + A/B CLI | `phase4_calib --write-model/--eval-model/--compare-model/--write-sparse/--json` |
| Dev workflows | `tools/dev.sh` (`model`, `model-ab`, `check`) |
| Prep inventory | `docs/PHASE5_PREP.md` |
| GPU re-scope | ROADMAP + this cert (not justified for 64-d calib tensors) |

## Test proof

```bash
cd kernel
cargo test
# 209+ lib unit tests + integration (phase1 SIMD/FFI, phase3 rails, self_parse)

cargo run --release --bin phase4_calib -- --docs ../docs
cargo run --release --bin phase4_calib -- --docs ../docs --self-mod
cargo run --release --bin density_bench
cargo run --release --bin graph_overhead_bench
```

Unit coverage added/kept: sparse≡dense score, graph-node path identity,
batch parallel ≡ serial, model wire+meta, indented-code feature.

## Measurements (2026-07-09 host)

### Precision calib — real `docs/` (22 markdown files)

```
n=2299 train=1839 test=460 exec=45 thr=11
base_bal=0.500
test_acc=0.954 test_bal=0.704 test_f1=0.276 test_rec=0.444 test_prec=0.200
delta_bal=+0.204
confusion: tp=4 tn=435 fp=16 fn=5
result: WIN
path_identity dense==graph_node: true
```

**vs Phase 4 certificate (same task family):** bal 0.61 → **0.70**, F1 0.18 →
**0.28**, rec 0.25 → **0.44**, prec 0.14 → **0.20**. Still not production-grade
precision; honest residual gap remains.

### density_bench (post Phase 5, n=262144)

| density | scalar µs | bit-sliced | sparse | BS/S | SP/S |
|--------:|----------:|-----------:|-------:|-----:|-----:|
| 1% | 80.2 | 6.5 | 3.9 | ~12.4× | ~20.8× |
| 10% | 80.1 | 6.5 | 13.4 | ~12.3× | ~6.0× |
| 50% | 80.1 | 6.5 | 13.4 | ~12.3× | ~6.0× |

### graph_overhead_bench

```
graph_forward_pass_us≈0.20  static_fold_us≈0.02  overhead_ratio≈10×
```

(Unchanged character vs Phase 2: structural overhead, not TOBL.)

### Self-mod (rail check)

`--self-mod`: proposed AddNode, **rejected** by fitness, ledgered; default off.

## Explicit non-goals / re-scopes

| Item | Disposition |
|------|-------------|
| GPU/NPU path | **Re-scoped to Phase 6+** — calib features are 64-d; density_bench already shows 12–20× CPU TOBL gains. GPU PCIe overhead would dominate at this size. Revisit when production tensors are large enough to justify transfer. |
| Structural `Graph::forward_pass` multi-thread | Not the hot path; **native** `forward_native_parallel` + **batch_predict_parallel** cover CPU parallelization for compute. |
| Production F1 ≥ 0.7 | Deferred — need richer labels / more code corpora (Phase 6+ data work). |
| Self-mod on by default | Forbidden (ADR 0002). |
| aethyro.com head-to-head | Phase 6 |

## Deep dive — highest leverage next

**Phase 6 Integration:** freeze a model artifact from real docs (`--write-model`),
load it in an external host / WASM or FFI consumer, and run a **real workload
head-to-head** vs aethyro.com inference on memory and latency. Intelligence
quality (F1) can improve in parallel, but product decision requires integration
measurement, not more internal micro-benches.

Do **not** build GPU until a profiled production tensor size exceeds the
CPU TOBL knee already measured.

## Sign-off

**COMPLETE.** Phase 6 may begin under PHASE_GATE_PROTOCOL.md.
