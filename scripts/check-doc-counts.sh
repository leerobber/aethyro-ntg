#!/usr/bin/env bash
# Keep documented test counts equal to the executed count.
#
# Docs mark the number as <!--tests-->N<!--/tests-->. This script compares every
# marker against the passed-test total in a `cargo test` log (and fails if a
# listed doc has no marker or if any test failed).
#
#   scripts/check-doc-counts.sh /path/to/cargo-test.log           # check
#   scripts/check-doc-counts.sh /path/to/cargo-test.log --write   # update markers
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${1:?usage: check-doc-counts.sh <cargo-test-log> [--write]}"
WRITE="${2:-}"
DOCS=(README.md CLAUDE.md docs/STATUS.md)

read -r passed failed < <(awk '/^test result:/ { p += $4; f += $6 } END { print p + 0, f + 0 }' "$LOG")
if [ "$passed" -eq 0 ] || [ "$failed" -ne 0 ]; then
  echo "check-doc-counts: log shows $passed passed / $failed failed; refusing to compare" >&2
  exit 1
fi

status=0
for doc in "${DOCS[@]}"; do
  file="$ROOT/$doc"
  found=$(grep -o '<!--tests-->[0-9]*<!--/tests-->' "$file" | sed 's/<!--tests-->\([0-9]*\)<!--\/tests-->/\1/' || true)
  if [ -z "$found" ]; then
    echo "MISSING marker in $doc (expected <!--tests-->N<!--/tests-->)"; status=1; continue
  fi
  if [ "$WRITE" = "--write" ]; then
    sed -i "s#<!--tests-->[0-9]*<!--/tests-->#<!--tests-->$passed<!--/tests-->#g" "$file"
    echo "updated $doc -> $passed"
    continue
  fi
  for n in $found; do
    if [ "$n" != "$passed" ]; then
      echo "STALE in $doc: documents $n, executed $passed"; status=1
    fi
  done
done

[ "$status" -eq 0 ] && echo "check-doc-counts: OK ($passed tests)"
exit "$status"
