#!/usr/bin/env bash
#
# Smart cargo build-sbf wrapper that uses per-program target directories
# to avoid cache invalidation from different feature combinations.
#
# Usage:
#   From workspace root: ./scripts/build-sbf.sh (builds all programs)
#   From program dir:    ./scripts/build-sbf.sh (builds current program)
#   Or add to PATH and use: build-sbf
#

set -e

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Build a single program with its own target dir
build_program() {
    local prog="$1"
    shift
    local name=$(basename "$prog")
    echo "==> Building $prog"
    CARGO_TARGET_DIR="$REPO_ROOT/target-$name" cargo build-sbf --manifest-path "$prog/Cargo.toml" "$@"
}

# If running from workspace root, build each program separately
if [ -f "Cargo.toml" ] && grep -q '^\[workspace\]' "Cargo.toml" 2>/dev/null; then
    echo "Building programs with separate target directories..."
    for prog in programs/*/; do
        if [ -f "$prog/Cargo.toml" ]; then
            build_program "$prog" "$@"
        fi
    done
else
    # Single program build - use program name for target dir
    name=$(basename "$PWD")
    export CARGO_TARGET_DIR="$REPO_ROOT/target-$name"
    echo "==> Building $name (target: $CARGO_TARGET_DIR)"
    exec cargo build-sbf "$@"
fi
