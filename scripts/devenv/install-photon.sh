#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_photon() {
    local expected_version=$(get_version "photon")
    local photon_installed=false
    local photon_correct_version=false

    export CARGO_HOME="${PREFIX}/cargo"
    export PATH="${PREFIX}/cargo/bin:${PATH}"

    if ! is_installed "photon"; then
        if [ -f "${PREFIX}/cargo/bin/photon" ]; then
            photon_installed=true
            if photon_version=$(${PREFIX}/cargo/bin/photon --version 2>/dev/null); then
                if echo "$photon_version" | grep -q "$expected_version"; then
                    photon_correct_version=true
                fi
            fi
        fi

        if [ "$photon_installed" = false ] || [ "$photon_correct_version" = false ]; then
            echo "Installing Photon indexer (version $expected_version)..."
            RUSTFLAGS="-A dead-code" cargo install --git https://github.com/helius-labs/photon.git --rev ${PHOTON_COMMIT} --locked --force
            log "photon"
        else
            echo "Photon already installed with correct version, skipping..."
        fi
    else
        echo "Photon already installed with correct version, skipping..."
    fi
}

install_photon
