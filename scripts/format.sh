#!/usr/bin/env bash

set -e

# JS formatting
cd js/stateless.js && pnpm format && cd ../..
cd js/compressed-token && pnpm format && cd ../..

# Rust formatting
cargo +nightly fmt --all
cargo clippy \
        --workspace \
        --no-deps \
        --all-features \
        --exclude photon-api \
        --exclude name-service \
        -- -A clippy::result_large_err \
           -A clippy::empty-docs \
           -A clippy::to-string-trait-impl \
           -A unexpected-cfgs \
           -A clippy::doc_lazy_continuation \
        -D warnings
