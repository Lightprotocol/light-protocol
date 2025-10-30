#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_pnpm() {
    if ! is_installed "pnpm" || [ ! -f "${PREFIX}/bin/pnpm" ]; then
        echo "Installing pnpm..."
        local version=$(get_version "pnpm")
        local suffix=$(get_suffix "pnpm")
        local url="https://github.com/pnpm/pnpm/releases/download/v${version}/pnpm-${suffix}"
        download "$url" "${PREFIX}/bin/pnpm"
        chmod +x "${PREFIX}/bin/pnpm"
        log "pnpm"
    else
        echo "pnpm already installed, skipping..."
    fi
}

install_pnpm
