#!/bin/bash
set -euo pipefail

start=$(python3 - <<'PY'
import time
print(time.time())
PY
)

commands=(
  "cargo check -p light-account-checks --tests --all-features"
  "cargo check -p light-system-program-pinocchio --tests"
  "cargo check -p light-compressed-token --tests"
  "cargo check -p light-sdk-pinocchio"
  "cargo check -p light-token-pinocchio"
)

failures=0
passing=0
for cmd in "${commands[@]}"; do
  echo ">>> $cmd"
  if bash -lc "$cmd" >/tmp/autoresearch-cmd.log 2>&1; then
    passing=$((passing + 1))
  else
    failures=$((failures + 1))
    tail -n 40 /tmp/autoresearch-cmd.log
  fi
done

echo "$failures" > .autoresearch_last_fail_count

suite_seconds=$(python3 - <<'PY' "$start"
import sys, time
start = float(sys.argv[1])
print(f"{time.time() - start:.3f}")
PY
)

echo "METRIC migration_failures=$failures"
echo "METRIC passing_commands=$passing"
echo "METRIC suite_seconds=$suite_seconds"
