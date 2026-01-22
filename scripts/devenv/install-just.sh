#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_just() {
    if ! is_installed "just" || [ ! -f "${PREFIX}/bin/just" ]; then
        echo "Installing just..."
        curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to "${PREFIX}/bin"
        log "just"
    else
        echo "just already installed, skipping..."
    fi
}

install_just
