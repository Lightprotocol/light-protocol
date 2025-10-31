#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/shared.sh"

install_redis() {
    if ! is_installed "redis" || [ ! -f "${PREFIX}/bin/redis-server" ] || [ ! -f "${PREFIX}/bin/redis-cli" ]; then
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
stop-writes-on-bgsave-error no
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
REDIS_DIR="$(cd "$(dirname "$(dirname "$0")")" && pwd)"
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
REDIS_DIR="$(cd "$(dirname "$(dirname "$0")")" && pwd)"
REDIS_PID="${REDIS_DIR}/var/redis.pid"

if [ ! -f "$REDIS_PID" ]; then
    echo "Redis PID file not found. Redis may not be running."
    exit 1
fi

PID=$(cat "$REDIS_PID")
if kill -0 "$PID" 2>/dev/null; then
    echo "Stopping Redis (PID: $PID)..."
    kill "$PID"

    for i in {1..10}; do
        if ! kill -0 "$PID" 2>/dev/null; then
            rm -f "$REDIS_PID"
            echo "Redis stopped successfully"
            exit 0
        fi
        sleep 1
    done

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
REDIS_DIR="$(cd "$(dirname "$(dirname "$0")")" && pwd)"
REDIS_PID="${REDIS_DIR}/var/redis.pid"

if [ -f "$REDIS_PID" ] && kill -0 "$(cat "$REDIS_PID")" 2>/dev/null; then
    PID=$(cat "$REDIS_PID")
    echo "Redis is running (PID: $PID)"
    echo "Connection: redis://localhost:6379"

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
    else
        echo "Redis already installed, skipping..."
    fi
}

install_redis
