#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

download_gnark_keys() {
    local key_type=${1:-light}
    ROOT_DIR="$(git rev-parse --show-toplevel)"
    PROVER_DIR="${ROOT_DIR}/prover/server"
    KEYS_DIR="${ROOT_DIR}/prover/server/proving-keys"

    case "$key_type" in
        "light")
            RUN_MODE="forester-test"
            ;;
        "full")
            RUN_MODE="full"
            ;;
        *)
            echo "Invalid key type: $key_type (expected 'light' or 'full')"
            exit 1
            ;;
    esac

    if ! is_installed "gnark_keys" || [ ! -d "${KEYS_DIR}" ] || [ -z "$(ls -A "${KEYS_DIR}" 2>/dev/null)" ]; then
        echo "Downloading gnark keys (run-mode: ${RUN_MODE})..."
        cd "${PROVER_DIR}" || {
            echo "Error: Failed to change directory to ${PROVER_DIR}" >&2
            exit 1
        }
        if go run . download \
            --run-mode="${RUN_MODE}" \
            --keys-dir="${KEYS_DIR}" \
            --max-retries=10; then
            log "gnark_keys"
        else
            exit_code=$?
            echo "Error: Failed to download gnark keys (exit code: ${exit_code})" >&2
            exit ${exit_code}
        fi
    else
        echo "Gnark keys already exist, skipping download..."
    fi
}

download_gnark_keys "$@"
