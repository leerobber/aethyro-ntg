#!/usr/bin/env bash
# Print the true executed-test count for the kernel crate (not #[test] markers).
# Output: "<passed> passed, <failed> failed, <ignored> ignored"
# Docs should reference this number, not hand-type it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/kernel"
cargo test --release 2>&1 | awk '
  /^test result:/ { p += $4; f += $6; i += $8 }
  END { printf "%d passed, %d failed, %d ignored\n", p, f, i }'
