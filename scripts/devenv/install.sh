#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

main() {
    mkdir -p "${PREFIX}/bin"

    local key_type="light"
    local reset_log=false
    local skip_components=""
    local force_reinstall=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --full-keys)
                key_type="full"
                shift
                ;;
            --no-reset)
                reset_log=false
                shift
                ;;
            --skip-components)
                if [ -z "$2" ] || [[ "$2" == --* ]]; then
                    echo "Error: --skip-components requires a value"
                    exit 1
                fi
                skip_components="$2"
                shift 2
                ;;
            --force-reinstall)
                force_reinstall=true
                shift
                ;;
            *)
                echo "Unknown option: $1"
                echo "Usage: $0 [--full-keys] [--no-reset] [--skip-components <comma-separated-list>] [--force-reinstall]"
                echo "Components that can be skipped: go,rust,node,pnpm,solana,anchor,jq,photon,keys,dependencies,redis"
                exit 1
                ;;
        esac
    done

    if $reset_log || $force_reinstall; then
        rm -f "$INSTALL_LOG"
    fi

    should_skip() {
        local component=$1
        [[ ",$skip_components," == *",$component,"* ]]
    }

    should_skip "go" || bash "${SCRIPT_DIR}/install-go.sh"
    should_skip "rust" || bash "${SCRIPT_DIR}/install-rust.sh"
    should_skip "photon" || bash "${SCRIPT_DIR}/install-photon.sh"
    should_skip "node" || bash "${SCRIPT_DIR}/install-node.sh"
    should_skip "pnpm" || bash "${SCRIPT_DIR}/install-pnpm.sh"
    should_skip "solana" || bash "${SCRIPT_DIR}/install-solana.sh"
    should_skip "anchor" || bash "${SCRIPT_DIR}/install-anchor.sh"
    should_skip "jq" || bash "${SCRIPT_DIR}/install-jq.sh"
    should_skip "keys" || bash "${SCRIPT_DIR}/download-gnark-keys.sh" "$key_type"
    should_skip "dependencies" || bash "${SCRIPT_DIR}/install-dependencies.sh"
    should_skip "redis" || bash "${SCRIPT_DIR}/install-redis.sh"

    echo "✨ Light Protocol development dependencies installed"
    if [ -n "$skip_components" ]; then
        echo "   Skipped components: $skip_components"
    fi
}

main "$@"
