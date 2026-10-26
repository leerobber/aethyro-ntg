#!/usr/bin/env bash
# Verify a deployed hostframe-backend from outside.
#
#   scripts/verify-deploy.sh https://your-service.up.railway.app
#
# Checks /health, POSTs the sample payload to /ingest, then reads /metrics.
# Prints a dated PASS/FAIL line you can paste into docs/STATUS.md.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
URL="${1:?usage: verify-deploy.sh <base-url>}"
URL="${URL%/}"
fail=0

check() { # name, command...
  local name="$1"; shift
  if out="$("$@" 2>&1)"; then printf 'PASS  %s\n' "$name"; else printf 'FAIL  %s: %s\n' "$name" "$out"; fail=1; fi
}

check "GET /health" curl -fsS --max-time 15 "$URL/health"
check "POST /ingest" bash -c "curl -fsS --max-time 15 -X POST -H 'content-type: application/json' -d @'$ROOT/hostframe-backend/test_payload.json' '$URL/ingest' | grep -q '\"success\":true'"
check "GET /metrics" bash -c "curl -fsS --max-time 15 '$URL/metrics' | grep -q total_batches_ingested"

stamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
if [ "$fail" -eq 0 ]; then
  echo "verified $URL at $stamp (health, ingest, metrics)"
else
  echo "NOT verified: $URL at $stamp"; exit 1
fi
