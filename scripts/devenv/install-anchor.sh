#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_anchor() {
    local version=$(get_version "anchor")
    local avm_installed=false
    local anchor_correct_version=false

    export CARGO_HOME="${PREFIX}/cargo"
    export PATH="${PREFIX}/cargo/bin:${PATH}"

    if ! is_installed "anchor"; then
        # Check if avm is installed
        if [ -f "${PREFIX}/cargo/bin/avm" ]; then
            avm_installed=true
        fi

        # Check if correct Anchor version is installed
        if [ -f "${PREFIX}/cargo/bin/anchor" ]; then
            if anchor_version=$(${PREFIX}/cargo/bin/anchor --version 2>/dev/null); then
                if echo "$anchor_version" | grep -q "$version"; then
                    anchor_correct_version=true
                fi
            fi
        fi

        if [ "$avm_installed" = false ]; then
            echo "Installing avm (Anchor Version Manager)..."
            cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
        fi

        if [ "$anchor_correct_version" = false ]; then
            echo "Installing Anchor ${version}..."
            avm install ${version}
            avm use ${version}
            log "anchor"
        else
            echo "Anchor ${version} already installed, skipping..."
        fi
    else
        echo "Anchor ${version} already installed, skipping..."
    fi
}

install_anchor
