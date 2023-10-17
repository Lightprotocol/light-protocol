#!/usr/bin/env sh

set -e

NX_CLOUD_DISTRIBUTED_EXECUTION=false npx nx run-many --target=format --all
NX_CLOUD_DISTRIBUTED_EXECUTION=false npx nx run-many --target=lint:fix --all

cargo +nightly fmt --all
cargo clippy --all -- -A clippy::result_large_err -D warnings
