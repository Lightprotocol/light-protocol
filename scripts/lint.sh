#!/usr/bin/env sh

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo fmt --all -- --check
cargo clippy \
      --workspace \
      --exclude photon-api \
      -- -A clippy::result_large_err \
         -A clippy::empty-docs \
         -A clippy::to-string-trait-impl \
      -D warnings