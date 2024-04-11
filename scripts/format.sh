#!/usr/bin/env sh

set -e

npx nx run-many --target=format --all
npx nx run-many --target=lint:fix --all

cargo fmt --all
cargo clippy \
      --workspace \
      --exclude macro-circom \
      --all -- -A clippy::result_large_err -D warnings