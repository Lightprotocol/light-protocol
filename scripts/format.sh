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

# Regenerate READMEs with cargo-rdme
echo "Regenerating READMEs..."
if ! command -v cargo-rdme &> /dev/null; then
    cargo install cargo-rdme
fi
for toml in $(find program-libs sdk-libs -name '.cargo-rdme.toml' -type f); do
    crate_dir=$(dirname "$toml")
    echo "Regenerating README in $crate_dir..."
    (cd "$crate_dir" && cargo rdme --no-fail-on-warnings)
done
