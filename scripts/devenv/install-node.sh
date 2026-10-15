#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_node() {
    local npm_cli="${PREFIX}/lib/node_modules/npm/bin/npm-cli.js"
    local needs_install=false

    if ! is_installed "node" || [ ! -f "${PREFIX}/bin/node" ] || [ ! -f "${PREFIX}/bin/npm" ] || [ ! -f "${npm_cli}" ]; then
        needs_install=true
    elif ! "${PREFIX}/bin/node" "${npm_cli}" --version >/dev/null 2>&1; then
        needs_install=true
    fi

    if [ "${needs_install}" = true ]; then
        echo "Installing Node.js..."
        local version=$(get_version "node")
        local suffix=$(get_suffix "node")
        local url="https://nodejs.org/dist/v${version}/node-v${version}-${suffix}.tar.gz"
        rm -rf "${PREFIX}/include/node" \
            "${PREFIX}/lib/node_modules/corepack" \
            "${PREFIX}/lib/node_modules/npm"
        download "$url" "${PREFIX}/node.tar.gz"
        tar -xzf "${PREFIX}/node.tar.gz" -C "${PREFIX}" --strip-components 1
        rm "${PREFIX}/node.tar.gz"
        log "node"
    else
        echo "Node.js already installed, skipping..."
    fi
}

install_node
