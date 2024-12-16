#!/usr/bin/env bash

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly fmt --all -- --check
cargo clippy \
      --workspace \
      --exclude photon-api \
      --exclude name-service \
      -- -A clippy::result_large_err \
         -A clippy::empty-docs \
         -A clippy::to-string-trait-impl \
         -A clippy::doc_lazy_continuation \
      -D warnings
