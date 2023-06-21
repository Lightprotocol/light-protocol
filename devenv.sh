# Command to deactivate the devenv. It sets the old environment variables.
deactivate () {
    PS1="${LIGHT_PROTOCOL_OLD_PS1}"
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

# Always use our third-party binaries first.
LIGHT_PROTOCOL_OLD_PATH="${PATH}"
PATH="$(git rev-parse --show-toplevel)/.local/bin:$PATH"