#!/usr/bin/env bash

set -euo pipefail

PREFIX="${PWD}/.local"
INSTALL_LOG="${PREFIX}/.install_log"

# Versions
VERSIONS=(
    "go:1.21.7"
    "node:20.9.0"
    "pnpm:9.5.0"
    "solana:2.2.15"
    "anchor:anchor-v0.29.0"
    "jq:jq-1.7.1"
    "photon:0.50.0"
    "redis:8.0.1"
)

# Architecture-specific suffixes
SUFFIXES=(
    "go_Darwin_x86_64:darwin-amd64"
    "go_Darwin_arm64:darwin-arm64"
    "go_Linux_x86_64:linux-amd64"
    "go_Linux_aarch64:linux-arm64"
    "node_Darwin_x86_64:darwin-x64"
    "node_Darwin_arm64:darwin-arm64"
    "node_Linux_x86_64:linux-x64"
    "node_Linux_aarch64:linux-arm64"
    "pnpm_Darwin_x86_64:macos-x64"
    "pnpm_Darwin_arm64:macos-arm64"
    "pnpm_Linux_x86_64:linuxstatic-x64"
    "pnpm_Linux_aarch64:linuxstatic-arm64"
    "solana_Darwin_x86_64:x86_64-apple-darwin"
    "solana_Darwin_arm64:aarch64-apple-darwin"
    "solana_Linux_x86_64:x86_64-unknown-linux-gnu"
    "solana_Linux_aarch64:aarch64-unknown-linux-gnu"
    "anchor_Darwin_x86_64:macos-amd64"
    "anchor_Darwin_arm64:macos-arm64"
    "anchor_Linux_x86_64:linux-amd64"
    "anchor_Linux_aarch64:linux-arm64"
    "jq_Darwin_x86_64:jq-osx-amd64"
    "jq_Darwin_arm64:jq-macos-arm64"
    "jq_Linux_x86_64:jq-linux-amd64"
    "jq_Linux_aarch64:jq-linux-arm64"
)

OS=$(uname)
ARCH=$(uname -m)

log() { echo "$1" >> "$INSTALL_LOG"; }
is_installed() { grep -q "^$1$" "$INSTALL_LOG" 2>/dev/null; }

get_version() {
    local key=$1
    for item in "${VERSIONS[@]}"; do
        IFS=':' read -r k v <<< "$item"
        if [ "$k" = "$key" ]; then
            echo "$v"
            return
        fi
    done
    echo "unknown"
}

get_suffix() {
    local key="${1}_${OS}_${ARCH}"
    for item in "${SUFFIXES[@]}"; do
        IFS=':' read -r k v <<< "$item"
        if [ "$k" = "$key" ]; then
            echo "$v"
            return
        fi
    done
    echo "unknown"
}

download() {
    curl -sSL --retry 5 --retry-delay 10 -o "$2" "$1"
    chmod +x "$2"
}

install_go() {
    if ! is_installed "go"; then
        echo "Installing Go..."
        local version=$(get_version "go")
        local suffix=$(get_suffix "go")
        local url="https://go.dev/dl/go${version}.${suffix}.tar.gz"
        download "$url" "${PREFIX}/go.tar.gz"
        tar -xzf "${PREFIX}/go.tar.gz" -C "${PREFIX}"
        rm "${PREFIX}/go.tar.gz"
        log "go"
    fi
}

install_rust() {
    if ! is_installed "rust"; then
        echo "Installing Rust..."
        export RUSTUP_HOME="${PREFIX}/rustup"
        export CARGO_HOME="${PREFIX}/cargo"
        curl --retry 5 --retry-delay 10 --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
        rustup install 1.86 nightly
        export PATH="${PREFIX}/cargo/bin:${PATH}"
        rustup component add --toolchain 1.86-x86_64-unknown-linux-gnu clippy
        cargo install cargo-expand --locked
        cargo install --git https://github.com/helius-labs/photon.git --rev cf58facb4e0521843e3afd21d09d8e7e7f772140 --locked
        log "rust"
    fi
}

install_node() {
    if ! is_installed "node"; then
        echo "Installing Node.js..."
        local version=$(get_version "node")
        local suffix=$(get_suffix "node")
        local url="https://nodejs.org/dist/v${version}/node-v${version}-${suffix}.tar.gz"
        download "$url" "${PREFIX}/node.tar.gz"
        tar -xzf "${PREFIX}/node.tar.gz" -C "${PREFIX}" --strip-components 1
        rm "${PREFIX}/node.tar.gz"
        log "node"
    fi
}

install_pnpm() {
    if ! is_installed "pnpm"; then
        echo "Installing pnpm..."
        local version=$(get_version "pnpm")
        local suffix=$(get_suffix "pnpm")
        local url="https://github.com/pnpm/pnpm/releases/download/v${version}/pnpm-${suffix}"
        download "$url" "${PREFIX}/bin/pnpm"
        chmod +x "${PREFIX}/bin/pnpm"
        log "pnpm"
    fi
}

install_solana() {
    if ! is_installed "solana"; then
        echo "Installing Solana..."
        local version=$(get_version "solana")
        local suffix=$(get_suffix "solana")
        local url="https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-${suffix}.tar.bz2"
        download "$url" "${PREFIX}/solana-release.tar.bz2"
        tar -xjf "${PREFIX}/solana-release.tar.bz2" -C "${PREFIX}/bin" --strip-components 2
        rm "${PREFIX}/solana-release.tar.bz2"
        log "solana"
    fi
}

install_anchor() {
    if ! is_installed "anchor"; then
        echo "Installing Anchor..."
        local version=$(get_version "anchor")
        local suffix=$(get_suffix "anchor")
        local url="https://github.com/Lightprotocol/binaries/releases/download/${version}/anchor-${suffix}"
        download "$url" "${PREFIX}/bin/anchor"
        log "anchor"
    fi
}

install_jq() {
    if ! is_installed "jq"; then
        echo "Installing jq..."
        local version=$(get_version "jq")
        local suffix=$(get_suffix "jq")
        local url="https://github.com/jqlang/jq/releases/download/${version}/${suffix}"
        download "$url" "${PREFIX}/bin/jq"
        log "jq"
    fi
}

download_gnark_keys() {
    if ! is_installed "gnark_keys"; then
        echo "Downloading gnark keys..."
        ROOT_DIR="$(git rev-parse --show-toplevel)"
        "${ROOT_DIR}/prover/server/scripts/download_keys.sh" "$1"
        log "gnark_keys"
    fi
}

install_dependencies() {
    if ! is_installed "dependencies"; then
        echo "Installing dependencies..."
        export PATH="${PREFIX}/bin:${PATH}"
        pnpm install
        log "dependencies"
    fi
}


install_redis() {
    if ! is_installed "redis"; then
        echo "Installing Redis..."
        local version=$(get_version "redis")
        local url="http://download.redis.io/releases/redis-${version}.tar.gz"

        if ! command -v make >/dev/null 2>&1; then
            echo "Warning: 'make' not found. Redis installation requires build tools."
            if [ "$OS" = "Darwin" ]; then
                echo "Please install Xcode command line tools: xcode-select --install"
            elif [ "$OS" = "Linux" ]; then
                echo "Please install build essentials (Ubuntu: apt-get install build-essential)"
            fi
            echo "Skipping Redis installation..."
            return
        fi

        curl -sSL --retry 5 --retry-delay 10 "$url" | tar -xz -C "${PREFIX}"
        cd "${PREFIX}/redis-${version}"

        make PREFIX="${PREFIX}" install >/dev/null 2>&1

        cd "${PREFIX}"
        rm -rf "redis-${version}"

        REDIS_PERSISTENT_DIR="${PREFIX}/var/redis"
        mkdir -p "${REDIS_PERSISTENT_DIR}"
        mkdir -p "${PREFIX}/etc"

        cat > "${PREFIX}/etc/redis.conf" << EOF
port 6379
bind 127.0.0.1
save 900 1
save 300 10
save 60 10000
stop-writes-on-bgsave-error yes
rdbcompression yes
rdbchecksum yes
dbfilename dump.rdb
dir ${REDIS_PERSISTENT_DIR}
maxmemory 256mb
maxmemory-policy allkeys-lru
EOF

        mkdir -p "${PREFIX}/bin"
        cat > "${PREFIX}/bin/redis-start" << 'EOF'
#!/bin/bash
REDIS_DIR="$(dirname "$(dirname "$(readlink -f "$0")")")"
REDIS_CONF="${REDIS_DIR}/etc/redis.conf"
REDIS_PID="${REDIS_DIR}/var/redis.pid"
REDIS_LOG="${REDIS_DIR}/var/redis.log"

mkdir -p "${REDIS_DIR}/var"

if [ -f "$REDIS_PID" ] && kill -0 "$(cat "$REDIS_PID")" 2>/dev/null; then
    echo "Redis is already running (PID: $(cat "$REDIS_PID"))"
    exit 0
fi

echo "Starting Redis server..."
"${REDIS_DIR}/bin/redis-server" "$REDIS_CONF" \
    --daemonize yes \
    --pidfile "$REDIS_PID" \
    --logfile "$REDIS_LOG"

if [ $? -eq 0 ]; then
    echo "Redis started successfully"
    echo "  - PID: $(cat "$REDIS_PID")"
    echo "  - Config: $REDIS_CONF"
    echo "  - Log: $REDIS_LOG"
    echo "  - Connection: redis://localhost:6379"
else
    echo "Failed to start Redis"
    exit 1
fi
EOF

        cat > "${PREFIX}/bin/redis-stop" << 'EOF'
#!/bin/bash
REDIS_DIR="$(dirname "$(dirname "$(readlink -f "$0")")")"
REDIS_PID="${REDIS_DIR}/var/redis.pid"

if [ ! -f "$REDIS_PID" ]; then
    echo "Redis PID file not found. Redis may not be running."
    exit 1
fi

PID=$(cat "$REDIS_PID")
if kill -0 "$PID" 2>/dev/null; then
    echo "Stopping Redis (PID: $PID)..."
    kill "$PID"

    # Wait for graceful shutdown
    for i in {1..10}; do
        if ! kill -0 "$PID" 2>/dev/null; then
            rm -f "$REDIS_PID"
            echo "Redis stopped successfully"
            exit 0
        fi
        sleep 1
    done

    # Force kill if necessary
    echo "Forcing Redis shutdown..."
    kill -9 "$PID" 2>/dev/null
    rm -f "$REDIS_PID"
    echo "Redis stopped"
else
    echo "Redis process not found"
    rm -f "$REDIS_PID"
fi
EOF

        cat > "${PREFIX}/bin/redis-status" << 'EOF'
#!/bin/bash
REDIS_DIR="$(dirname "$(dirname "$(readlink -f "$0")")")"
REDIS_PID="${REDIS_DIR}/var/redis.pid"

if [ -f "$REDIS_PID" ] && kill -0 "$(cat "$REDIS_PID")" 2>/dev/null; then
    PID=$(cat "$REDIS_PID")
    echo "Redis is running (PID: $PID)"
    echo "Connection: redis://localhost:6379"

    # Test connection
    if command -v "${REDIS_DIR}/bin/redis-cli" >/dev/null 2>&1; then
        if "${REDIS_DIR}/bin/redis-cli" ping >/dev/null 2>&1; then
            echo "Status: HEALTHY"
        else
            echo "Status: UNHEALTHY (process running but not responding)"
        fi
    fi
else
    echo "Redis is not running"
fi
EOF

        chmod +x "${PREFIX}/bin/redis-start" "${PREFIX}/bin/redis-stop" "${PREFIX}/bin/redis-status"
        mkdir -p "${PREFIX}/etc" "${PREFIX}/var"
        log "redis"
    fi
}



main() {
    mkdir -p "${PREFIX}/bin"

    # Parse command line arguments
    local key_type="light"
    local reset_log=true
    local skip_components=""
    
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
            *)
                echo "Unknown option: $1"
                echo "Usage: $0 [--full-keys] [--no-reset] [--skip-components <comma-separated-list>]"
                echo "Components that can be skipped: go,rust,node,pnpm,solana,anchor,jq,keys,dependencies,redis"
                exit 1
                ;;
        esac
    done

    if $reset_log; then
        rm -f "$INSTALL_LOG"
    fi

    # Helper function to check if component should be skipped
    should_skip() {
        local component=$1
        [[ ",$skip_components," == *",$component,"* ]]
    }

    # Install components unless explicitly skipped
    should_skip "go" || install_go
    should_skip "rust" || install_rust
    should_skip "node" || install_node
    should_skip "pnpm" || install_pnpm
    should_skip "solana" || install_solana
    should_skip "anchor" || install_anchor
    should_skip "jq" || install_jq
    should_skip "keys" || download_gnark_keys "$key_type"
    should_skip "dependencies" || install_dependencies
    should_skip "redis" || install_redis

    echo "✨ Light Protocol development dependencies installed"
    if [ -n "$skip_components" ]; then
        echo "   Skipped components: $skip_components"
    fi
}

main "$@"
