#!/usr/bin/env sh

set -e

npx nx run-many --target=format --all
npx nx run-many --target=lint:fix --all

cargo +nightly-2024-02-01 fmt --all

for rust_toolchain in stable nightly-2024-02-01; do
    cargo +"$rust_toolchain" clippy \
      --workspace \
      --exclude macro-circom \
      --all -- -A clippy::result_large_err -D warnings
done
