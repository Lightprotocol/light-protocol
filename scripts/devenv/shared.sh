#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/versions.sh"

export PREFIX="${PWD}/.local"
export INSTALL_LOG="${PREFIX}/.install_log"

# Ensure PREFIX directory exists
mkdir -p "${PREFIX}/bin"

VERSIONS=(
    "go:${GO_VERSION}"
    "node:${NODE_VERSION}"
    "pnpm:${PNPM_VERSION}"
    "solana:${SOLANA_VERSION}"
    "anchor:${ANCHOR_VERSION}"
    "jq:${JQ_TAG}"
    "photon:${PHOTON_VERSION}"
    "redis:${REDIS_VERSION}"
)

SUFFIXES=(
    "go_Darwin_x86_64:darwin-amd64"
    "go_Darwin_arm64:darwin-arm64"
    "go_Linux_x86_64:linux-amd64"
    "go_Linux_aarch64:linux-arm64"
    "node_Darwin_x86_64:darwin-x64"
    "node_Darwin_arm64:darwin-arm64"
    "node_Linux_x86_64:linux-x64"
    "node_Linux_aarch64:linux-arm64"
    "pnpm_Darwin_x86_64:macos-x64"
    "pnpm_Darwin_arm64:macos-arm64"
    "pnpm_Linux_x86_64:linuxstatic-x64"
    "pnpm_Linux_aarch64:linuxstatic-arm64"
    "solana_Darwin_x86_64:x86_64-apple-darwin"
    "solana_Darwin_arm64:aarch64-apple-darwin"
    "solana_Linux_x86_64:x86_64-unknown-linux-gnu"
    "solana_Linux_aarch64:aarch64-unknown-linux-gnu"
    "jq_Darwin_x86_64:jq-osx-amd64"
    "jq_Darwin_arm64:jq-macos-arm64"
    "jq_Linux_x86_64:jq-linux-amd64"
    "jq_Linux_aarch64:jq-linux-arm64"
)

export OS=$(uname)
export ARCH=$(uname -m)

log() { echo "$1" >> "$INSTALL_LOG"; }
is_installed() { grep -q "^$1$" "$INSTALL_LOG" 2>/dev/null; }

get_version() {
    local key=$1
    for item in "${VERSIONS[@]}"; do
        IFS=':' read -r k v <<< "$item"
        if [ "$k" = "$key" ]; then
            echo "$v"
            return
        fi
    done
    echo "unknown"
}

get_suffix() {
    local key="${1}_${OS}_${ARCH}"
    for item in "${SUFFIXES[@]}"; do
        IFS=':' read -r k v <<< "$item"
        if [ "$k" = "$key" ]; then
            echo "$v"
            return
        fi
    done
    echo "unknown"
}

download() {
    curl -sSL --retry 5 --retry-delay 10 -o "$2" "$1"
    chmod +x "$2"
}
