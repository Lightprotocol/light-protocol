#!/usr/bin/env bash

# Command to deactivate the devenv. It sets the old environment variables.
deactivate () {
    # Only try to stop redis if the command exists
    if command -v redis-stop >/dev/null 2>&1; then
        redis-stop 2>/dev/null || true
    fi

    PS1="${LIGHT_PROTOCOL_OLD_PS1}"
    RUSTUP_HOME="${LIGHT_PROTOCOL_OLD_RUSTUP_HOME}"
    CARGO_HOME="${LIGHT_PROTOCOL_OLD_CARGO_HOME}"
    NPM_CONFIG_PREFIX="${LIGHT_PROTOCOL_OLD_NPM_CONFIG_PREFIX}"
    PATH="${LIGHT_PROTOCOL_OLD_PATH}"
    [ -n "${LIGHT_PROTOCOL_OLD_RUST_PATH}" ] && PATH="${LIGHT_PROTOCOL_OLD_RUST_PATH}"
    [ -n "${LIGHT_PROTOCOL_OLD_CPATH}" ] && CPATH="${LIGHT_PROTOCOL_OLD_CPATH}"
    unset LIGHT_PROTOCOL_DEVENV
    unset LIGHT_PROTOCOL_TOPLEVEL
    unset GOROOT
    unset RUSTUP_HOME
    unset CARGO_HOME
    unset LIGHT_PROTOCOL_OLD_RUST_PATH
    unset CARGO_FEATURES
}

# Stop early if already in devenv.
if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_DEVENV=1
else
    return 2>/dev/null || exit 0
fi

# The root of the git repository.
LIGHT_PROTOCOL_TOPLEVEL="$(git rev-parse --show-toplevel 2>/dev/null)"
if [ -z "$LIGHT_PROTOCOL_TOPLEVEL" ]; then
    echo "Error: Not in a git repository" >&2
    return 1 2>/dev/null || exit 1
fi

# Verify the .local directory exists
if [ ! -d "${LIGHT_PROTOCOL_TOPLEVEL}/.local" ]; then
    echo "Error: .local directory not found. Please run ./scripts/install.sh first" >&2
    return 1 2>/dev/null || exit 1
fi

# Shell prompt (only set if PS1 exists - not in CI)
if [ -n "${PS1:-}" ]; then
    LIGHT_PROTOCOL_OLD_PS1="${PS1}"
    PS1="[🧢 Light Protocol devenv] ${PS1}"
fi

# Ensure that our rustup environment is used.
LIGHT_PROTOCOL_OLD_RUSTUP_HOME="${RUSTUP_HOME:-}"
RUSTUP_HOME="${LIGHT_PROTOCOL_TOPLEVEL}/.local/rustup"
LIGHT_PROTOCOL_OLD_CARGO_HOME="${CARGO_HOME:-}"
CARGO_HOME="${LIGHT_PROTOCOL_TOPLEVEL}/.local/cargo"

# Ensure that our npm prefix is used.
LIGHT_PROTOCOL_OLD_NPM_CONFIG_PREFIX="${NPM_CONFIG_PREFIX:-}"
NPM_CONFIG_PREFIX="${LIGHT_PROTOCOL_TOPLEVEL}/.local/npm-global"

# Always use our binaries first.
LIGHT_PROTOCOL_OLD_PATH="${PATH}"
PATH="${LIGHT_PROTOCOL_TOPLEVEL}/.local/bin:${PATH}"
PATH="${LIGHT_PROTOCOL_TOPLEVEL}/.local/cargo/bin:${PATH}"
PATH="${LIGHT_PROTOCOL_TOPLEVEL}/.local/go/bin:${PATH}"
PATH="${LIGHT_PROTOCOL_TOPLEVEL}/.local/npm-global/bin:${PATH}"

# Remove the original Rust-related PATH entries
PATH=$(echo "$PATH" | tr ':' '\n' | grep -vE "/.rustup/|/.cargo/" | tr '\n' ':' | sed 's/:$//')

# Define alias of `light` to use the CLI built from source (only if not in CI)
if [ -z "${CI:-}" ]; then
    alias light="${LIGHT_PROTOCOL_TOPLEVEL}/cli/test_bin/run"
fi

# Define GOROOT for Go.
export GOROOT="${LIGHT_PROTOCOL_TOPLEVEL}/.local/go"

# Ensure Rust binaries are in PATH
PATH="${CARGO_HOME}/bin:${PATH}"

# Export all critical environment variables
export PATH
export RUSTUP_HOME
export CARGO_HOME
export NPM_CONFIG_PREFIX
export LIGHT_PROTOCOL_TOPLEVEL
export LIGHT_PROTOCOL_DEVENV
export SBF_OUT_DIR=target/deploy

# Set Redis URL if not already set
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"

# Enable v2_ix feature by default in devenv
export CARGO_FEATURES="${CARGO_FEATURES:-v2_ix}"

# macOS-specific settings
if [[ "$(uname)" == "Darwin" ]]; then
    LIGHT_PROTOCOL_OLD_CPATH="${CPATH:-}"
    export CPATH="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include:${CPATH:-}"
fi

# Validate critical tools are available (only warn, don't fail)
if [ -z "${LIGHT_PROTOCOL_SKIP_VALIDATION:-}" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Warning: cargo not found in PATH. Run ./scripts/install.sh to install Rust." >&2
    fi
    if ! command -v go >/dev/null 2>&1; then
        echo "Warning: go not found in PATH. Run ./scripts/install.sh to install Go." >&2
    fi
    if ! command -v node >/dev/null 2>&1; then
        echo "Warning: node not found in PATH. Run ./scripts/install.sh to install Node.js." >&2
    fi
fi
