#!/usr/bin/env bash

. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

export RUST_MIN_STACK=8388608
export RUSTFLAGS="-D warnings"

ROOT_DIR=$(git rev-parse --show-toplevel)

cargo llvm-cov \
  --all-targets --workspace \
  --exclude light-concurrent-merkle-tree \
  --exclude photon-api \
  --exclude forester \
  --html \
  --output-dir "${ROOT_DIR}/target/llvm-cov" \
  --open
cargo llvm-cov \
  --all-targets \
  --package light-concurrent-merkle-tree \
  --html \
  --output-dir "${ROOT_DIR}/target/llvm-cov-cmt" \
  --open
