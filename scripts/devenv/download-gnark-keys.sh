#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

download_gnark_keys() {
    local key_type=${1:-light}
    ROOT_DIR="$(git rev-parse --show-toplevel)"

    if [ ! -d "${ROOT_DIR}/prover/server/proving-keys" ] || [ -z "$(ls -A "${ROOT_DIR}/prover/server/proving-keys" 2>/dev/null)" ]; then
        echo "Downloading gnark keys..."
        "${ROOT_DIR}/prover/server/scripts/download_keys.sh" "$key_type"
        log "gnark_keys"
    else
        echo "Gnark keys already exist, skipping download..."
    fi
}

download_gnark_keys "$@"
