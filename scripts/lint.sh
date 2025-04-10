#!/usr/bin/env bash

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --exclude light-system-program-pinocchio  -- -D warnings
# We import the same crates with different features in light-system-program-pinocchio than in account-compression
# clippy cannot handle this. -> check light-system-program-pinocchio separately.
cargo clippy --package light-system-program-pinocchio --all-targets -- -D warnings
