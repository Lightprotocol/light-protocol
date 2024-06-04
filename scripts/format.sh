#!/usr/bin/env sh

set -e

npx nx run-many --target=format --all
npx nx run-many --target=lint:fix --all

cargo fmt --all
cargo clippy \
      --workspace \
      --exclude photon-api \
      -- -A clippy::result_large_err \
         -A clippy::empty-docs \
         -A clippy::to-string-trait-impl \
      -D warnings
