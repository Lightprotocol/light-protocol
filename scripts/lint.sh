#!/usr/bin/env sh

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all
cargo fmt --all -- --check && cargo clippy --workspace --exclude macro-circom --all -- -A clippy::result_large_err -D warnings
