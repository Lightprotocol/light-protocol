#!/usr/bin/env bash

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Check no_std compatibility for light-zero-copy crate
echo "Checking no_std compatibility for light-zero-copy..."
cargo check -p light-zero-copy --no-default-features
