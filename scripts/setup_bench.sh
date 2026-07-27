#!/usr/bin/env bash
# =============================================================================
# scripts/setup_bench.sh
# aethyro-ntg — One-shot GPU benchmark setup + runner.
#
# Handles:
#   • Blackwell RTX 5050 (sm_120) — needs PyTorch 2.6+ with CUDA 12.8
#   • Uses existing sovereign-core venv if present; creates ~/.bench-venv otherwise
#   • Upgrades torch to cu128 only when an NVIDIA GPU/driver is detected
#   • Skips CUDA upgrade cleanly on CPU-only or driverless machines
#
# Usage:
#   chmod +x scripts/setup_bench.sh
#   ./scripts/setup_bench.sh [--reps N] [--warmup N] [--skip-gpu]
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_PY="$SCRIPT_DIR/bench_cpu_gpu.py"

# ── Find Python environment ──────────────────────────────────────────────────────────────────
SOVEREIGN_VENV="$HOME/sovereign-core/.venv"
BENCH_VENV="$HOME/.bench-venv"

if [ -x "$SOVEREIGN_VENV/bin/python" ]; then
  VENV="$SOVEREIGN_VENV"
  echo "Using sovereign-core venv: $VENV"
elif [ -x "$BENCH_VENV/bin/python" ]; then
  VENV="$BENCH_VENV"
  echo "Using bench venv: $VENV"
else
  echo "Creating bench venv at $BENCH_VENV ..."
  python3 -m venv "$BENCH_VENV"
  VENV="$BENCH_VENV"
fi

PYTHON="$VENV/bin/python"
PIP="$VENV/bin/pip"

# ── Check CUDA availability ───────────────────────────────────────────────────────────────────────────
TORCH_VER=$( "$PYTHON" -c "import torch; print(torch.__version__)" 2>/dev/null || echo "none" )
CUDA_OK=$( "$PYTHON" -c "import torch; print(torch.cuda.is_available())" 2>/dev/null || echo "False" )

echo "PyTorch: $TORCH_VER  |  CUDA available: $CUDA_OK"

# ── Upgrade to cu128 only when an NVIDIA GPU+driver is present ─────────────────────────
if [ "$CUDA_OK" != "True" ]; then
  # Probe for an NVIDIA driver before attempting a 2 GB download.
  # nvidia-smi -L exits 0 and lists GPUs when the driver is loaded.
  HAS_NVIDIA=false
  if command -v nvidia-smi &>/dev/null && nvidia-smi -L &>/dev/null 2>&1; then
    HAS_NVIDIA=true
  fi

  if [ "$HAS_NVIDIA" = "true" ]; then
    echo ""
    echo "NVIDIA GPU detected but CUDA not available — upgrading PyTorch for Blackwell sm_120 (cu128)..."
    echo "This downloads ~2 GB; please wait."
    "$PIP" install --upgrade \
      torch \
      --index-url https://download.pytorch.org/whl/cu128 \
      -q
    echo "Done. Re-checking CUDA..."
    CUDA_OK=$( "$PYTHON" -c "import torch; print(torch.cuda.is_available())" 2>/dev/null || echo "False" )
    echo "CUDA available: $CUDA_OK"
  else
    echo "No NVIDIA GPU/driver detected — skipping cu128 upgrade, running CPU-only benchmark."
  fi
fi

# Ensure numpy
"$PIP" install -q numpy 2>/dev/null || true

# ── Run benchmark ────────────────────────────────────────────────────────────────────────────────
echo ""
"$PYTHON" "$BENCH_PY" "$@"
