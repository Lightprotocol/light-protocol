#!/usr/bin/env bash

set -euo pipefail

PREFIX="${PWD}/.local"
INSTALL_LOG="${PREFIX}/.install_log"

# Versions
VERSIONS=(
    "go:1.21.7"
    "node:20.9.0"
    "pnpm:9.5.0"
    "solana:1.18.22"
    "anchor:anchor-v0.29.0"
    "jq:jq-1.7.1"
    "photon:0.45.0"
)

# Architecture-specific suffixes
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
    "anchor_Darwin_x86_64:macos-amd64"
    "anchor_Darwin_arm64:macos-arm64"
    "anchor_Linux_x86_64:linux-amd64"
    "anchor_Linux_aarch64:linux-arm64"
    "jq_Darwin_x86_64:jq-osx-amd64"
    "jq_Darwin_arm64:jq-macos-arm64"
    "jq_Linux_x86_64:jq-linux-amd64"
    "jq_Linux_aarch64:jq-linux-arm64"
)

OS=$(uname)
ARCH=$(uname -m)

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

install_go() {
    if ! is_installed "go"; then
        echo "Installing Go..."
        local version=$(get_version "go")
        local suffix=$(get_suffix "go")
        local url="https://go.dev/dl/go${version}.${suffix}.tar.gz"
        download "$url" "${PREFIX}/go.tar.gz"
        tar -xzf "${PREFIX}/go.tar.gz" -C "${PREFIX}"
        rm "${PREFIX}/go.tar.gz"
        log "go"
    fi
}

install_rust() {
    if ! is_installed "rust"; then
        echo "Installing Rust..."
        export RUSTUP_HOME="${PREFIX}/rustup"
        export CARGO_HOME="${PREFIX}/cargo"
        curl --retry 5 --retry-delay 10 --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path        
        export PATH="${PREFIX}/cargo/bin:${PATH}"
        rustup component add clippy rustfmt
        cargo install cargo-expand wasm-pack
        cargo install photon-indexer --version $(get_version "photon") --locked
        log "rust"
    fi
}

install_node() {
    if ! is_installed "node"; then
        echo "Installing Node.js..."
        local version=$(get_version "node")
        local suffix=$(get_suffix "node")
        local url="https://nodejs.org/dist/v${version}/node-v${version}-${suffix}.tar.gz"
        download "$url" "${PREFIX}/node.tar.gz"
        tar -xzf "${PREFIX}/node.tar.gz" -C "${PREFIX}" --strip-components 1
        rm "${PREFIX}/node.tar.gz"
        log "node"
    fi
}

install_pnpm() {
    if ! is_installed "pnpm"; then
        echo "Installing pnpm..."
        local version=$(get_version "pnpm")
        local suffix=$(get_suffix "pnpm")
        local url="https://github.com/pnpm/pnpm/releases/download/v${version}/pnpm-${suffix}"
        download "$url" "${PREFIX}/bin/pnpm"
        chmod +x "${PREFIX}/bin/pnpm"
        log "pnpm"
    fi
}

install_solana() {
    if ! is_installed "solana"; then
        echo "Installing Solana..."
        local version=$(get_version "solana")
        local suffix=$(get_suffix "solana")
        local url="https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-${suffix}.tar.bz2"
        download "$url" "${PREFIX}/solana-release.tar.bz2"
        tar -xjf "${PREFIX}/solana-release.tar.bz2" -C "${PREFIX}/bin" --strip-components 2
        rm "${PREFIX}/solana-release.tar.bz2"
        log "solana"
    fi
}

install_anchor() {
    if ! is_installed "anchor"; then
        echo "Installing Anchor..."
        local version=$(get_version "anchor")
        local suffix=$(get_suffix "anchor")
        local url="https://github.com/Lightprotocol/binaries/releases/download/${version}/anchor-${suffix}"        
        download "$url" "${PREFIX}/bin/anchor"
        log "anchor"
    fi
}

install_jq() {
    if ! is_installed "jq"; then
        echo "Installing jq..."
        local version=$(get_version "jq")
        local suffix=$(get_suffix "jq")
        local url="https://github.com/jqlang/jq/releases/download/${version}/${suffix}"
        download "$url" "${PREFIX}/bin/jq"
        log "jq"
    fi
}

download_gnark_keys() {
    if ! is_installed "gnark_keys"; then
        echo "Downloading gnark keys..."
        ROOT_DIR="$(git rev-parse --show-toplevel)"
        "${ROOT_DIR}/light-prover/scripts/download_keys.sh"
        log "gnark_keys"
    fi
}

install_dependencies() {
    if ! is_installed "dependencies"; then
        echo "Installing dependencies..."
        export PATH="${PREFIX}/bin:${PATH}"
        pnpm install
        log "dependencies"
    fi
}

main() {
    mkdir -p "${PREFIX}/bin"

    install_go
    install_rust
    install_node
    install_pnpm
    install_solana
    install_anchor
    install_jq
    download_gnark_keys
    install_dependencies

    rm -f "$INSTALL_LOG"

    echo "✨ Light Protocol development dependencies installed"
}

main