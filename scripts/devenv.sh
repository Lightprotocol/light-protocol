# Command to deactivate the devenv. It sets the old environment variables.
deactivate () {
    PS1="${LIGHT_PROTOCOL_OLD_PS1}"
    RUSTUP_HOME="${LIGHT_PROTOCOL_OLD_RUSTUP_HOME}"
    CARGO_HOME="${LIGHT_PROTOCOL_OLD_CARGO_HOME}"
    NPM_CONFIG_PREFIX="${LIGHT_PROTOCOL_OLD_NPM_CONFIG_PREFIX}"
    PATH="${LIGHT_PROTOCOL_OLD_PATH}"
    unset LIGHT_PROTOCOL_DEVENV
    unset LIGHT_PROTOCOL_TOPLEVEL
}

# Stop early if already in devenv.
if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_DEVENV=1
else
    return
fi

# The root of the git repository.
LIGHT_PROTOCOL_TOPLEVEL="`git rev-parse --show-toplevel`"

# Shell prompt.
LIGHT_PROTOCOL_OLD_PS1="${PS1:-}"
PS1="[🧢 Light Protocol devenv] ${PS1:-}"

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
PATH="${LIGHT_PROTOCOL_TOPLEVEL}/.local/npm-global/bin:${PATH}"

# Define alias of `light` to use the CLI built from source.
alias light="${LIGHT_PROTOCOL_TOPLEVEL}/cli/test_bin/run"
