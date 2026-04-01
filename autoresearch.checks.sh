#!/bin/bash
set -euo pipefail

last_fail_count=$(cat .autoresearch_last_fail_count 2>/dev/null || echo 999)
if [ "$last_fail_count" != "0" ]; then
  echo "Fast suite not green yet; skipping slow checks."
  exit 0
fi

export RUSTFLAGS="${RUSTFLAGS:--D warnings}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"

cargo test -p light-account-checks --all-features >/tmp/ar-check-1.log 2>&1 || { tail -n 80 /tmp/ar-check-1.log; exit 1; }
cargo test -p light-sdk-macros --all-features >/tmp/ar-check-2.log 2>&1 || { tail -n 80 /tmp/ar-check-2.log; exit 1; }
cargo test-sbf -p pinocchio-nostd-test >/tmp/ar-check-3.log 2>&1 || { tail -n 80 /tmp/ar-check-3.log; exit 1; }
cargo test-sbf -p sdk-pinocchio-v1-test >/tmp/ar-check-4.log 2>&1 || { tail -n 80 /tmp/ar-check-4.log; exit 1; }
cargo test-sbf -p sdk-pinocchio-v2-test >/tmp/ar-check-5.log 2>&1 || { tail -n 80 /tmp/ar-check-5.log; exit 1; }

echo "Slow migration checks passed."
