#!/usr/bin/env bash

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${PWD}")"

# Versions extracted from source files (automatic)
export RUST_VERSION=$(grep 'channel' "${REPO_ROOT}/rust-toolchain.toml" | sed 's/.*"\(.*\)".*/\1/' | cut -d'.' -f1,2)
export GO_VERSION=$(grep '^go ' "${REPO_ROOT}/prover/server/go.mod" | awk '{print $2}')
export PNPM_VERSION=$(grep 'packageManager' "${REPO_ROOT}/package.json" | sed 's/.*pnpm@\([^"]*\).*/\1/')

# Versions to bump manually (edit below)
export NODE_VERSION="22.16.0"
export SOLANA_VERSION="2.2.15"
export ANCHOR_VERSION="0.31.1"
export JQ_VERSION="1.8.0"
export PHOTON_VERSION="0.51.0"
export PHOTON_COMMIT="94b3688b08477668bb946a689b0267319f5c1ae1"
export REDIS_VERSION="8.0.1"

export ANCHOR_TAG="anchor-v${ANCHOR_VERSION}"
export JQ_TAG="jq-${JQ_VERSION}"
