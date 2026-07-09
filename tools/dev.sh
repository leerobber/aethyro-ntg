#!/usr/bin/env bash
# Developer shortcuts for aethyro-ntg (run from repo root or anywhere).
# Prefer tools/dev.ps1 on Windows PowerShell if bash complains about pipefail (CRLF).
set -eu
# pipefail is bash-specific; tolerate environments that choke (CRLF / non-bash).
set -o pipefail 2>/dev/null || true
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
K="$ROOT/kernel"
ART="${ARTIFACTS:-$ROOT/artifacts}"
cd "$K"

cmd="${1:-help}"
shift || true

case "$cmd" in
  test)
    cargo test "$@"
    ;;
  test-quiet)
    cargo test -q
    ;;
  clippy)
    cargo clippy -- -D warnings
    ;;
  check)
    cargo test -q
    cargo clippy -- -D warnings
    ;;
  phase4)
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --json "$@"
    ;;
  phase4-fixtures)
    cargo run --release --bin phase4_calib -- --json "$@"
    ;;
  phase4-self-mod)
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --self-mod --json "$@"
    ;;
  density)
    cargo run --release --bin density_bench "$@"
    ;;
  graph-overhead)
    cargo run --release --bin graph_overhead_bench "$@"
    ;;
  model)
    # train + write model + sparse + report, then re-eval + predict
    mkdir -p "$ART/models"
    MODEL="${MODEL_PATH:-$ART/models/ntg.calib}"
    SPARSE="${SPARSE_PATH:-$ART/models/ntg.sparse}"
    REPORT="${REPORT_PATH:-$ART/models/ntg.report.json}"
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" \
      --write-model "$MODEL" --write-sparse "$SPARSE" --write-report "$REPORT" --json
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --eval-model "$MODEL" --json
    cargo run --release --bin phase4_calib -- --eval-model "$MODEL" \
      --predict 'fn main() { println!("hi"); }' --sparse-score
    echo "# artifacts: $MODEL $SPARSE $REPORT"
    ;;
  model-ab)
    # train twice with different epochs, compare (A/B harness for later sweeps)
    mkdir -p "$ART/models"
    A="${MODEL_A:-$ART/models/a.calib}"
    B="${MODEL_B:-$ART/models/b.calib}"
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --epochs 20 --write-model "$A" --json
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --epochs 60 --write-model "$B" --json
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" \
      --eval-model "$A" --compare-model "$B" --json
    ;;
  school)
    # Doctorate schooling: real docs, study+exam, 75% gate, multi-run notebooks
    RUNS="${SCHOOL_RUNS:-5}"
    cargo run --release --bin ntg_school -- \
      --docs "$ROOT/docs" \
      --out "$ROOT/docs/schooling/runs" \
      --runs "$RUNS" \
      --max-attempts "${SCHOOL_MAX_ATTEMPTS:-5}"
    ;;
  school-phase)
    # usage: tools/dev.sh school-phase 4
    P="${1:-4}"
    cargo run --release --bin ntg_school -- \
      --docs "$ROOT/docs" \
      --out "$ROOT/docs/schooling/runs" \
      --phase "$P" \
      --runs "${SCHOOL_RUNS:-3}"
    ;;
  all)
    cargo test -q
    cargo run --release --bin density_bench
    cargo run --release --bin graph_overhead_bench
    cargo run --release --bin phase4_calib -- --docs "$ROOT/docs" --json
    cargo run --release --bin ntg_school -- --docs "$ROOT/docs" --out "$ROOT/docs/schooling/runs" --runs 3
    ;;
  help|*)
    cat <<EOF
usage: tools/dev.sh <cmd>   (or on Windows: .\\tools\\dev.ps1 <cmd>)

  test | test-quiet     cargo test
  clippy | check        clippy (-D warnings); check = test + clippy
  phase4                calib on docs/ + --json
  phase4-fixtures       calib on fixtures
  phase4-self-mod       calib + --self-mod
  density               density_bench
  graph-overhead        graph_overhead_bench
  model                 train → artifacts/models (MODEL_PATH override)
  model-ab              train 20 vs 60 epochs, --compare-model
  school                doctorate ntg_school (SCHOOL_RUNS default 5)
  school-phase N        school only phase N
  all                   test + density + overhead + phase4 + school(3)

Env: ARTIFACTS, MODEL_PATH, SPARSE_PATH, REPORT_PATH, MODEL_A, MODEL_B
     SCHOOL_RUNS, SCHOOL_MAX_ATTEMPTS

Windows PowerShell (recommended):
  .\\tools\\dev.ps1 school
  # or pure cargo:
  cd kernel
  cargo run --release --bin ntg_school -- --docs ../docs --out ../docs/schooling/runs --runs 5
EOF
    ;;
esac
