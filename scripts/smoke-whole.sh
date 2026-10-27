#!/usr/bin/env bash
# Whole-stack smoke: dashboard (spawns kernel_host) + keymaster-backend +
# hostframe-backend.
#
# Required checks fail the script; optional dependencies (Ollama) are skipped
# and reported, never failed. hostframe-backend's BigQuery client is currently
# a stub (in-memory metrics; startup only needs GCP_PROJECT), so it runs here
# with a placeholder project.
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
  (cd "$ROOT/hostframe-backend" && cargo build --release) || { echo "hostframe build failed"; exit 1; }
fi

# The kernel is a workspace member, so its binary lands in the repo-root target/.
export KERNEL_HOST_BIN="${KERNEL_HOST_BIN:-$ROOT/target/release/kernel_host}"
# KEYMASTER_BACKEND_URL is deliberately NOT pointed at keymaster-backend: that service speaks a
# different protocol (docs/architecture/0012-keymaster-contract.md). The dashboard-spawned
# kernel_host runs local-only; the external-backend contract is checked below with a mock.

PORT="$KM_PORT" "$ROOT/backend/keymaster/target/release/keymaster-backend" >"$LOGDIR/keymaster.log" 2>&1 &
PIDS+=($!)
HF_PORT="${HF_PORT:-8081}"
PORT="$HF_PORT" GCP_PROJECT="${GCP_PROJECT:-smoke-test}" \
  "$ROOT/hostframe-backend/target/release/hostframe-backend" >"$LOGDIR/hostframe.log" 2>&1 &
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

wait_for "http://127.0.0.1:$HF_PORT/health" 30 && pass "hostframe /health" || fail "hostframe /health"
out="$(curl -fsS -X POST -H 'content-type: application/json' -d @"$ROOT/hostframe-backend/test_payload.json" "http://127.0.0.1:$HF_PORT/ingest")" \
  && echo "$out" | grep -q '"success":true' && pass "hostframe /ingest -> $(echo "$out" | head -c 80)" || fail "hostframe /ingest ($out)"
out="$(curl -fsS "http://127.0.0.1:$HF_PORT/metrics")" && echo "$out" | grep -q 'total_batches' \
  && pass "hostframe /metrics -> $(echo "$out" | head -c 80)" || fail "hostframe /metrics ($out)"

post() { curl -fsS -X POST -H 'content-type: application/json' -d "$2" "$BASE$1"; }

out="$(post /api/classify '{"text":"fn main() { println!(\"hi\"); }"}')" && [ -n "$out" ] \
  && pass "/api/classify -> $(echo "$out" | head -c 80)" || fail "/api/classify"
out="$(post /api/score '{"text":"fn main() { println!(\"hi\"); }"}')" && [ -n "$out" ] \
  && pass "/api/score -> $(echo "$out" | head -c 80)" || fail "/api/score"
out="$(curl -fsS "$BASE/api/verify_ledger")" && [ -n "$out" ] \
  && pass "/api/verify_ledger -> $(echo "$out" | head -c 80)" || fail "/api/verify_ledger"

# Drive routed traffic so the Quad-Brain has something real to observe.
for i in $(seq 1 30); do
  post /api/classify "{\"text\":\"sample number $i: fn f() { let x = $i; }\"}" >/dev/null || break
done
out="$(curl -fsS "$BASE/api/quad_brain")" || out=""
if echo "$out" | python3 -c '
import json, sys
d = json.load(sys.stdin)
assert d["cycles"] >= 30, d["cycles"]
assert d["synchronized"] is True
assert d["beta"]["strategies"], "beta learned no strategies"
assert d["delta"]["forecasts"] == d["cycles"]
assert 0.0 <= d["delta"]["forecast_quality"] <= 1.0
' 2>/dev/null; then
  pass "/api/quad_brain -> $(echo "$out" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("cycles=%d regime=%s strategies=%d" % (d["cycles"], d["delta"]["regime"], len(d["beta"]["strategies"])))')"
else
  fail "/api/quad_brain ($out)"
fi
out="$(post /api/hierarchy '{"nano":10000}')" \
  && echo "$out" | grep -q '"total_agents":10111' && echo "$out" | grep -q '"nano":10000' \
  && pass "/api/hierarchy 10000 nano -> $(echo "$out" | head -c 90)" || fail "/api/hierarchy ($out)"

# ---- external-backend contract (ADR 0012): kernel_host <-> any {intent,payload} backend
cat >"$LOGDIR/mock_backend.py" <<'PY'
import http.server, json, sys
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get("content-length", 0))))
        out = json.dumps({"mock": True, "echo": body}).encode()
        self.send_response(200); self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out))); self.end_headers(); self.wfile.write(out)
    def log_message(self, *a): pass
srv = http.server.HTTPServer(("127.0.0.1", 0), H)
open(sys.argv[1], "w").write(str(srv.server_address[1]))
srv.serve_forever()
PY
python3 "$LOGDIR/mock_backend.py" "$LOGDIR/mock.port" &
PIDS+=($!)
for _ in $(seq 1 20); do [ -s "$LOGDIR/mock.port" ] && break; sleep 0.25; done
MOCK_PORT="$(cat "$LOGDIR/mock.port" 2>/dev/null)"
REQS='{"intent":"summarize","payload":{"text":"hi"}}
{"intent":"_status"}
{"intent":"_quit"}'
check_json() { python3 -c "import json,sys; d=[json.loads(l) for l in sys.stdin.read().splitlines() if l.strip()]; $1" 2>/dev/null; }
if [ -n "$MOCK_PORT" ]; then
  out="$(printf '%s\n' "$REQS" | KEYMASTER_BACKEND_URL="http://127.0.0.1:$MOCK_PORT" timeout 60 "$KERNEL_HOST_BIN" 2>/dev/null)"
  echo "$out" | check_json 'r=d[0]; assert r["backend"]=="external" and r["result"]["mock"] is True and r["result"]["echo"]=={"intent":"summarize","payload":{"text":"hi"}}' \
    && pass "kernel_host external contract: {intent,payload} out, JSON back as result" || fail "kernel_host external contract ($out)"
else
  fail "kernel_host external contract (mock backend did not start)"
fi
out="$(printf '%s\n' "$REQS" | KEYMASTER_BACKEND_URL="http://127.0.0.1:1" timeout 60 "$KERNEL_HOST_BIN" 2>/dev/null)"
echo "$out" | check_json 'r=d[0]; assert r["backend"]=="external" and r["result"]["external"] is True and "error" in r["result"]; assert len(d)==3 and d[2]["status"]=="goodbye"' \
  && pass "kernel_host external failure: error payload, process keeps serving" || fail "kernel_host external failure ($out)"

if curl -fsS -o /dev/null --max-time 2 "${OLLAMA_URL:-http://127.0.0.1:11434}/api/tags" 2>/dev/null; then
  post /api/chat '{"message":"What is NTG?"}' >/dev/null && pass "/api/chat" || fail "/api/chat"
else
  skip "/api/chat (Ollama not reachable; optional)"
fi

echo
if [ "$FAILS" -eq 0 ]; then echo "smoke-whole: PASS"; else echo "smoke-whole: $FAILS required check(s) FAILED (logs: $LOGDIR)"; exit 1; fi
