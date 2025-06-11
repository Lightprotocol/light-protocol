#!/usr/bin/env bash

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
