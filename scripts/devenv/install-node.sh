#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_node() {
    if ! is_installed "node" || [ ! -f "${PREFIX}/bin/node" ] || [ ! -f "${PREFIX}/bin/npm" ]; then
        echo "Installing Node.js..."
        local version=$(get_version "node")
        local suffix=$(get_suffix "node")
        local url="https://nodejs.org/dist/v${version}/node-v${version}-${suffix}.tar.gz"
        download "$url" "${PREFIX}/node.tar.gz"
        tar -xzf "${PREFIX}/node.tar.gz" -C "${PREFIX}" --strip-components 1
        rm "${PREFIX}/node.tar.gz"
        log "node"
    else
        echo "Node.js already installed, skipping..."
    fi
}

install_node
