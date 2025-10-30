#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_dependencies() {
    if ! is_installed "dependencies" || [ ! -d "node_modules" ] || [ -z "$(ls -A node_modules 2>/dev/null)" ]; then
        echo "Installing dependencies..."
        export PATH="${PREFIX}/bin:${PATH}"
        pnpm install
        log "dependencies"
    else
        echo "Dependencies already installed, skipping..."
    fi
}

install_dependencies
