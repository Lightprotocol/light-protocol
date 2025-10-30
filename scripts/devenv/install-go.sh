#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_go() {
    if ! is_installed "go" || [ ! -d "${PREFIX}/go" ] || [ ! -f "${PREFIX}/go/bin/go" ]; then
        echo "Installing Go..."
        local version=$(get_version "go")
        local suffix=$(get_suffix "go")
        local url="https://go.dev/dl/go${version}.${suffix}.tar.gz"
        download "$url" "${PREFIX}/go.tar.gz"
        tar -xzf "${PREFIX}/go.tar.gz" -C "${PREFIX}"
        rm "${PREFIX}/go.tar.gz"
        log "go"
    else
        echo "Go already installed, skipping..."
    fi
}

install_go
