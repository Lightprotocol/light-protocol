# Command to deactivate the devenv. It sets the old environment variables.
deactivate () {
    PS1="${LIGHT_PROTOCOL_OLD_PS1}"
    RUSTUP_HOME="${LIGHT_PROTOCOL_OLD_RUSTUP_HOME}"
    CARGO_HOME="${LIGHT_PROTOCOL_OLD_CARGO_HOME}"
    PATH="${LIGHT_PROTOCOL_OLD_PATH}"
    unset LIGHT_PROTOCOL_DEVENV
}

# Stop early if already in devenv.
if [ -z "${LIGHT_PROTOCOL_DEVENV}" ]; then
    LIGHT_PROTOCOL_DEVENV=1
else
    return
fi

# Shell prompt.
LIGHT_PROTOCOL_OLD_PS1="${PS1}"
PS1="[🧢 Light Protocol devenv] ${PS1}"

# Always use our rustup environment and third-party binaries first.
LIGHT_PROTOCOL_OLD_RUSTUP_HOME="${RUSTUP_HOME}"
RUSTUP_HOME="$(git rev-parse --show-toplevel)/.local/rustup"
LIGHT_PROTOCOL_OLD_CARGO_HOME="${CARGO_HOME}"
CARGO_HOME="$(git rev-parse --show-toplevel)/.local/cargo"

source "$(git rev-parse --show-toplevel)/.local/cargo/env"

LIGHT_PROTOCOL_OLD_PATH="${PATH}"
PATH="$(git rev-parse --show-toplevel)/.local/bin:$PATH"
