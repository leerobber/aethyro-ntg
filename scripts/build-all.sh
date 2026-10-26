#!/usr/bin/env bash
# Build (and by default test) every crate in the repo with one command.
#
# The root workspace contains only `kernel`; dashboard, backend/keymaster and
# hostframe-backend are deliberately excluded (see Cargo.toml). This script is
# the single entry point that covers all four.
#
#   scripts/build-all.sh            # release build + tests + clippy where enforced
#   scripts/build-all.sh --no-test  # release build only
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN_TESTS=1
[ "${1:-}" = "--no-test" ] && RUN_TESTS=0

step() { printf '\n==> %s\n' "$*"; }

step "kernel: build"
(cd "$ROOT/kernel" && cargo build --release)
if [ "$RUN_TESTS" = 1 ]; then
  step "kernel: test"
  (cd "$ROOT/kernel" && cargo test --release)
  step "kernel: clippy -D warnings"
  (cd "$ROOT/kernel" && cargo clippy -- -D warnings)
fi

step "keymaster-backend: build"
(cd "$ROOT/backend/keymaster" && cargo build --release)
if [ "$RUN_TESTS" = 1 ]; then
  step "keymaster-backend: test + clippy"
  (cd "$ROOT/backend/keymaster" && cargo test --release && cargo clippy -- -D warnings)
fi

step "dashboard: build"
(cd "$ROOT/dashboard" && cargo build --release)
[ "$RUN_TESTS" = 1 ] && (cd "$ROOT/dashboard" && cargo test --release)

step "hostframe-backend: build"
(cd "$ROOT/hostframe-backend" && cargo build --release)
[ "$RUN_TESTS" = 1 ] && (cd "$ROOT/hostframe-backend" && cargo test --release)

printf '\nbuild-all: OK\n'
