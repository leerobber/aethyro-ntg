# Phase 4 COMPLETE Certificate

**Sign-off date:** 2026-07-09  
**Phase N+1 may begin:** **YES** (Phase 5 — Optimization)

## Scope

Training / calibration loop (ROADMAP Phase 4, ADR 0006): real task,
measured win/non-win, optional self-mod under ADR 0002 (off by default).

## Deliverables map

| Item | Path |
|------|------|
| Task design | `docs/architecture/0006-phase4-calibration-task.md` |
| Calibration engine | `kernel/src/ntg/calib/mod.rs` |
| Runner | `kernel/src/bin/phase4_calib.rs` |
| Class imbalance fix | balanced epochs, minority oversample, F1 thr, hold-out |
| Self-mod probe | `optional_self_mod_probe` (flag `--self-mod`, default off) |
| Experiments | `docs/EXPERIMENTS.md` Phase 4 entries |

## Test proof

```bash
cd kernel
cargo test                    # 201+ unit + integration suites green
cargo run --release --bin phase4_calib -- --docs ../docs
cargo run --release --bin phase4_calib -- --docs ../docs --self-mod
```

Unit coverage: features, fixtures, split, metrics, calibrate, ledger
snapshot, self-mod disabled/enabled.

## Measurements (real `docs/`, 2026-07-09)

```
n≈2212 train/test split 80/20, exec≈40 content≈2172
test_bal=0.611 vs base_bal=0.500  rec=0.25  prec≈0.14  f1≈0.18
fp≈12  → WIN on balanced metrics
self-mod default: disabled (rail 1)
self-mod --self-mod: proposed AddNode, rejected by fitness, ledgered
```

## Explicit non-goals (deferred)

| Item | Target |
|------|--------|
| High Execution precision / production F1 | Phase 5+ feature work |
| Topology self-mod that permanently mutates caller graph for gain | Phase 5 research |
| GPU/NPU calibration | Phase 5 |
| aethyro.com production head-to-head | Phase 6 |

## Deep dive — what advances the project next

**Highest leverage after Phase 4:** improve **minority precision** (code
features / threshold calibration) and measure **TOBL/sparse speed** on
the same labeled pipeline (Phase 5), not more rails. The closed loop
exists; intelligence quality (F1) is the bottleneck, not safety plumbing.

Self-mod probe proves ledger + fitness reject path works when enabled;
do not turn self-mod on by default.

## Sign-off

**COMPLETE.** Phase 5 may begin under PHASE_GATE_PROTOCOL.md.
