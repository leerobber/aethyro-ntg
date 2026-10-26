#!/usr/bin/env bash
# Whole-stack smoke: dashboard (spawns kernel_host) + keymaster-backend.
#
# Required checks fail the script; optional dependencies (Ollama) are skipped
# and reported, never failed. hostframe-backend needs live BigQuery at startup,
# so it is build-checked by build-all.sh and not started here.
#
#   scripts/smoke-whole.sh            # builds release binaries first
#   SKIP_BUILD=1 scripts/smoke-whole.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DASH_PORT="${DASH_PORT:-4000}"
KM_PORT="${KM_PORT:-8080}"   # dashboard probes keymaster on 127.0.0.1:8080
LOGDIR="$(mktemp -d)"
FAILS=0
PIDS=()

cleanup() { for p in "${PIDS[@]:-}"; do [ -n "$p" ] && kill "$p" 2>/dev/null; done; }
trap cleanup EXIT

pass() { printf 'PASS  %s\n' "$*"; }
fail() { printf 'FAIL  %s\n' "$*"; FAILS=$((FAILS + 1)); }
skip() { printf 'SKIP  %s\n' "$*"; }

wait_for() { # url, tries
  for _ in $(seq 1 "${2:-60}"); do
    curl -fsS -o /dev/null "$1" 2>/dev/null && return 0
    sleep 1
  done
  return 1
}

if [ -z "${SKIP_BUILD:-}" ]; then
  (cd "$ROOT/kernel" && cargo build --release --bin kernel_host) || { echo "kernel_host build failed"; exit 1; }
  (cd "$ROOT/backend/keymaster" && cargo build --release) || { echo "keymaster build failed"; exit 1; }
  (cd "$ROOT/dashboard" && cargo build --release) || { echo "dashboard build failed"; exit 1; }
fi

# The kernel is a workspace member, so its binary lands in the repo-root target/.
export KERNEL_HOST_BIN="${KERNEL_HOST_BIN:-$ROOT/target/release/kernel_host}"
export KEYMASTER_BACKEND_URL="${KEYMASTER_BACKEND_URL:-http://127.0.0.1:$KM_PORT}"

PORT="$KM_PORT" "$ROOT/backend/keymaster/target/release/keymaster-backend" >"$LOGDIR/keymaster.log" 2>&1 &
PIDS+=($!)
PORT="$DASH_PORT" REPO_ROOT="$ROOT" DASHBOARD_STATIC_DIR="$ROOT/dashboard/static" \
  "$ROOT/dashboard/target/release/aethyro-dashboard" >"$LOGDIR/dashboard.log" 2>&1 &
PIDS+=($!)

BASE="http://127.0.0.1:$DASH_PORT"

wait_for "$BASE/api/health" 60 && pass "dashboard /api/health" || { fail "dashboard /api/health (logs: $LOGDIR)"; exit 1; }
wait_for "http://127.0.0.1:$KM_PORT/health" 30 && pass "keymaster /health" || fail "keymaster /health"

health="$(curl -fsS "$BASE/api/health")"
echo "$health" | grep -q '"kernel_host":"up"' && pass "health: kernel_host up" || fail "health: kernel_host not up ($health)"
echo "$health" | grep -q '"keymaster_backend":"up"' && pass "health: keymaster seen by dashboard" || fail "health: dashboard cannot see keymaster ($health)"

post() { curl -fsS -X POST -H 'content-type: application/json' -d "$2" "$BASE$1"; }

out="$(post /api/classify '{"text":"fn main() { println!(\"hi\"); }"}')" && [ -n "$out" ] \
  && pass "/api/classify -> $(echo "$out" | head -c 80)" || fail "/api/classify"
out="$(post /api/score '{"text":"fn main() { println!(\"hi\"); }"}')" && [ -n "$out" ] \
  && pass "/api/score -> $(echo "$out" | head -c 80)" || fail "/api/score"
out="$(curl -fsS "$BASE/api/verify_ledger")" && [ -n "$out" ] \
  && pass "/api/verify_ledger -> $(echo "$out" | head -c 80)" || fail "/api/verify_ledger"

if curl -fsS -o /dev/null --max-time 2 "${OLLAMA_URL:-http://127.0.0.1:11434}/api/tags" 2>/dev/null; then
  post /api/chat '{"message":"What is NTG?"}' >/dev/null && pass "/api/chat" || fail "/api/chat"
else
  skip "/api/chat (Ollama not reachable; optional)"
fi

echo
if [ "$FAILS" -eq 0 ]; then echo "smoke-whole: PASS"; else echo "smoke-whole: $FAILS required check(s) FAILED (logs: $LOGDIR)"; exit 1; fi
