#!/usr/bin/env bash

. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

npx nx run-many --target=test --all --parallel=false &&\
    RUSTFLAGS="-D warnings" \
            cargo test --all-targets --workspace \
            --exclude light-concurrent-merkle-tree \
            --exclude photon-api && \
    cargo test-sbf -p account-compression-test &&\
    cargo test-sbf -p compressed-token-test &&\
    cargo test-sbf -p e2e-test &&\
    cargo test-sbf -p registry-test &&\
    cargo test-sbf -p sdk-test-program &&\
    cargo test-sbf -p system-cpi-test &&\
    cargo test-sbf -p system-test