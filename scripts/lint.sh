#!/usr/bin/env sh

set -e

npx nx run-many --target=format:check --all

npx nx run-many --target=lint --all

cd system-programs && cargo fmt --all -- --check && cargo clippy --all -- -A clippy::result_large_err -D warnings && cd -;