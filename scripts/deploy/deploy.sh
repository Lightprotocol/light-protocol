#!/bin/bash
# assumes that programs have been build with build-verifiable.sh
# Creates buffer accounts
# Buffer account addresses can be used in multisig action

set -e

# Hardcoded program IDs (format: "program_name:program_id")
PROGRAMS=(
    "account_compression:compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq"
    "light_compressed_token:cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
    "light_system_program_pinocchio:SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7"
    "light_registry:Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"
)

BUFFER_KEYPAIR_PATH="target/buffer"

create_buffer_account() {
    local max_retries=5
    local attempt=1

    local program_name="$1"
    local program_id="$2"

    while (( attempt <= max_retries )); do
        echo "Attempt $attempt of $max_retries..."
        echo "Writing buffer for $program_name (program ID: $program_id)"
        if solana program write-buffer target/deploy/"$program_name".so --buffer "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json"; then
            echo "Command succeeded on attempt $attempt."
            return 0
        else
            echo "Command failed on attempt $attempt."
            ((attempt++))
            sleep 2
        fi
    done

    echo "Command failed after $max_retries attempts."
    return 1
}

# Iterate over each program
for entry in "${PROGRAMS[@]}"; do
    program_name="${entry%%:*}"
    program_id="${entry##*:}"

    echo "Processing $program_name with program ID: $program_id"

    if [[ ! -f "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json" ]]; then
        solana-keygen new --outfile "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json" --no-bip39-passphrase
    fi

    create_buffer_account "$program_name" "$program_id"

    buffer_pubkey=$(solana-keygen pubkey "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json")
    echo "Buffer pubkey for $program_name: $buffer_pubkey"

    solana program set-buffer-authority "$buffer_pubkey" --new-buffer-authority 7PeqkcCXeqgsp5Mi15gjJh8qvSLk7n3dgNuyfPhJJgqY
    echo "Buffer created and authority set for $program_name"
    echo "---"
done
