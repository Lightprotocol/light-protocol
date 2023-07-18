# Command to deactivate the devenv. It sets the old environment variables.
deactivate () {
    PS1="${LIGHT_PROTOCOL_OLD_PS1}"
    RUSTUP_HOME="${LIGHT_PROTOCOL_OLD_RUSTUP_HOME}"
    CARGO_HOME="${LIGHT_PROTOCOL_OLD_CARGO_HOME}"
    NPM_CONFIG_PREFIX="${LIGHT_PROTOCOL_OLD_NPM_CONFIG_PREFIX}"
    PATH="${LIGHT_PROTOCOL_OLD_PATH}"
    unset LIGHT_PROTOCOL_DEVENV
}

# Stop early if already in devenv.
if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_DEVENV=1
else
    return
fi

# Shell prompt.
LIGHT_PROTOCOL_OLD_PS1="${PS1:-}"
PS1="[🧢 Light Protocol devenv] ${PS1:-}"

# Ensure that our rustup environment is used.
LIGHT_PROTOCOL_OLD_RUSTUP_HOME="${RUSTUP_HOME:-}"
RUSTUP_HOME="`git rev-parse --show-toplevel`/.local/rustup"
LIGHT_PROTOCOL_OLD_CARGO_HOME="${CARGO_HOME:-}"
CARGO_HOME="`git rev-parse --show-toplevel`/.local/cargo"

# Ensure that our npm prefix is used.
LIGHT_PROTOCOL_OLD_NPM_CONFIG_PREFIX="${NPM_CONFIG_PREFIX:-}"
NPM_CONFIG_PREFIX="`git rev-parse --show-toplevel`/.local/npm-global"

# Always use our binaries first.
LIGHT_PROTOCOL_OLD_PATH="${PATH}"
PATH="`git rev-parse --show-toplevel`/.local/bin:$PATH"
PATH="`git rev-parse --show-toplevel`/.local/cargo/bin:$PATH"
PATH="`git rev-parse --show-toplevel`/.local/npm-global/bin:$PATH"
