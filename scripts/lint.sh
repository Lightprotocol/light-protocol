#!/usr/bin/env sh

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly-2024-02-01 fmt --all -- --check

for rust_toolchain in stable nightly-2024-02-01; do
    cargo +"$rust_toolchain" clippy \
      --workspace \
      --exclude macro-circom \
      --all -- -A clippy::result_large_err -D warnings
done
