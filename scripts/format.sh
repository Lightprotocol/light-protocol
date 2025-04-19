#!/usr/bin/env bash

set -e

# npx nx run-many --target=format --all
# npx nx run-many --target=lint:fix --all

# cargo +nightly fmt --all
cargo clippy \
        --workspace \
        --no-deps \
        --exclude name-service \
        --exclude photon-api \
        --exclude name-service \
        --exclude light-system-program-pinocchio \
        -- -A clippy::result_large_err \
           -A clippy::empty-docs \
           -A clippy::to-string-trait-impl \
           -A unexpected-cfgs \
           -A clippy::doc_lazy_continuation \
        -D warnings
# We import the same crates with different features in light-system-program-pinocchio than in account-compression
# clippy cannot handle this. -> check light-system-program-pinocchio separately
cargo clippy --package light-system-program-pinocchio --all-targets -- -D warnings

# Make sure that tests compile
cargo test-sbf -p system-test --no-run
cargo test-sbf -p system-cpi-test --no-run
cargo test-sbf -p e2e-test --no-run
cargo test-sbf -p compressed-token-test --no-run
cargo test-sbf -p token-escrow --no-run
cargo test-sbf -p sdk-test --no-run
cargo test-sbf -p sdk-anchor-test --no-run
