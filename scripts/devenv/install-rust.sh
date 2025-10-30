#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_rust() {
    if ! is_installed "rust" || [ ! -d "${PREFIX}/rustup" ] || [ ! -d "${PREFIX}/cargo" ] || [ ! -f "${PREFIX}/cargo/bin/cargo" ]; then
        echo "Installing Rust..."
        export RUSTUP_HOME="${PREFIX}/rustup"
        export CARGO_HOME="${PREFIX}/cargo"
        curl --retry 5 --retry-delay 10 --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
        export PATH="${PREFIX}/cargo/bin:${PATH}"
        rustup install ${RUST_VERSION} nightly
        rustup component add --toolchain ${RUST_VERSION} clippy
        log "rust"
    else
        echo "Rust already installed, skipping..."
    fi
}

install_rust
