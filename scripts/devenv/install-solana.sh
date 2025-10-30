#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_solana() {
    if ! is_installed "solana" || [ ! -f "${PREFIX}/bin/solana" ] || [ ! -f "${PREFIX}/bin/solana-keygen" ]; then
        echo "Installing Solana..."
        local version=$(get_version "solana")
        local suffix=$(get_suffix "solana")
        local url="https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-${suffix}.tar.bz2"
        download "$url" "${PREFIX}/solana-release.tar.bz2"
        tar -xjf "${PREFIX}/solana-release.tar.bz2" -C "${PREFIX}/bin" --strip-components 2
        rm "${PREFIX}/solana-release.tar.bz2"
        log "solana"
    else
        echo "Solana already installed, skipping..."
    fi
}

install_solana
