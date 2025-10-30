#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_jq() {
    if ! is_installed "jq" || [ ! -f "${PREFIX}/bin/jq" ]; then
        echo "Installing jq..."
        local version=$(get_version "jq")
        local suffix=$(get_suffix "jq")
        local url="https://github.com/jqlang/jq/releases/download/${version}/${suffix}"
        download "$url" "${PREFIX}/bin/jq"
        log "jq"
    else
        echo "jq already installed, skipping..."
    fi
}

install_jq
