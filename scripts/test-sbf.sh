#!/usr/bin/env bash
#
# Smart cargo test-sbf wrapper that uses per-program target directories
# to avoid cache invalidation from different feature combinations.
#
# Usage:
#   From workspace root: ./scripts/test-sbf.sh (tests all programs)
#   From program dir:    ./scripts/test-sbf.sh (tests current program)
#

set -e

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Test a single program with its own target dir
test_program() {
    local prog="$1"
    shift
    local name=$(basename "$prog")
    echo "==> Testing $prog"
    CARGO_TARGET_DIR="$REPO_ROOT/target-$name" cargo test-sbf --manifest-path "$prog/Cargo.toml" "$@"
}

# If running from workspace root, test each program separately
if [ -f "Cargo.toml" ] && grep -q '^\[workspace\]' "Cargo.toml" 2>/dev/null; then
    echo "Testing programs with separate target directories..."
    for prog in programs/*/; do
        if [ -f "$prog/Cargo.toml" ]; then
            test_program "$prog" "$@"
        fi
    done
else
    # Single program test - use program name for target dir
    name=$(basename "$PWD")
    export CARGO_TARGET_DIR="$REPO_ROOT/target-$name"
    echo "==> Testing $name (target: $CARGO_TARGET_DIR)"
    exec cargo test-sbf "$@"
fi
