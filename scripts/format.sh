#!/usr/bin/env sh

set -e

npx nx run-many --target=format --all
cd system-programs && cargo fmt --all && cargo clippy --all -- -A clippy::result_large_err -D warnings && cd -;