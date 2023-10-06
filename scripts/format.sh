#!/usr/bin/env sh

set -e

npx nx run-many --target=format --all
npx nx run-many --target=lint:fix --all

cargo +nightly fmt --all
cargo clippy --all -- -A clippy::result_large_err -D warnings
