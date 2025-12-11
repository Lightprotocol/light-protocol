#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

# Cross-platform sed in-place (macOS vs Linux)
sed_inplace() {
    if [ "$OS" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

install_photon() {
    local expected_version="${PHOTON_VERSION}"
    local expected_commit="${PHOTON_COMMIT}"
    local install_marker="photon:${expected_version}:${expected_commit}"

    # Validate required variables
    if [ -z "${expected_version}" ] || [ -z "${expected_commit}" ]; then
        echo "ERROR: PHOTON_VERSION or PHOTON_COMMIT not set in versions.sh"
        exit 1
    fi

    export CARGO_HOME="${PREFIX}/cargo"
    export PATH="${PREFIX}/cargo/bin:${PATH}"

    # Ensure directories and log file exist
    mkdir -p "${PREFIX}/cargo/bin"
    touch "$INSTALL_LOG"

    # Check if exact version+commit combo is already installed
    if grep -q "^${install_marker}$" "$INSTALL_LOG" 2>/dev/null; then
        # Double-check binary actually exists
        if [ -f "${PREFIX}/cargo/bin/photon" ]; then
            echo "Photon ${expected_version} (commit ${expected_commit}) already installed, skipping..."
            return 0
        fi
        # Binary missing despite log entry - remove stale log entry
        sed_inplace "/^photon:/d" "$INSTALL_LOG" 2>/dev/null || true
    fi

    # Remove any old photon entries from log (different version/commit)
    sed_inplace "/^photon:/d" "$INSTALL_LOG" 2>/dev/null || true
    sed_inplace "/^photon$/d" "$INSTALL_LOG" 2>/dev/null || true

    echo "Installing Photon indexer ${expected_version} (commit ${expected_commit})..."
    RUSTFLAGS="-A dead-code" cargo install --git https://github.com/helius-labs/photon.git --rev ${expected_commit} --locked --force

    # Verify installation succeeded
    if [ ! -f "${PREFIX}/cargo/bin/photon" ]; then
        echo "ERROR: Photon installation failed - binary not found"
        exit 1
    fi

    # Log the exact version+commit installed
    echo "${install_marker}" >> "$INSTALL_LOG"
    echo "Photon ${expected_version} (commit ${expected_commit}) installed successfully"
}

install_photon
