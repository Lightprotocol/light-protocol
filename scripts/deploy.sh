# assumes that programs have been build with build-verifiable.sh
# Creates buffer accounts
# Buffer account addresses can be used in multisig action

# Array of program names
libraries=("account_compression" "light_compressed_token" "light_system_program_pinocchio" "light_registry")

BUFFER_KEYPAIR_PATH="target/buffer"


create_buffer_account() {
    local max_retries=5
    local attempt=1

    local program_name="$1"
    local program_name_keypair="$2"

    while (( attempt <= max_retries )); do
        echo "Attempt $attempt of $max_retries..."
        echo "$BUFFER_KEYPAIR_PATH-$program_name_keypair.json"
        if solana program write-buffer target/deploy/"$program_name".so --buffer "$BUFFER_KEYPAIR_PATH-$program_name_keypair-keypair.json"; then
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



# Iterate over each program name and build it
for program_name in "${libraries[@]}"; do
    if [[ ! -f "$BUFFER_KEYPAIR_PATH" ]]; then
        solana-keygen new --outfile "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json" --no-bip39-passphrase
    fi
    create_buffer_account "$program_name" "$program_name"
    buffer_pubkey=$(solana-keygen pubkey "$BUFFER_KEYPAIR_PATH-$program_name-keypair.json")
    echo "Buffer pubkey for $program_name: $buffer_pubkey"
    solana program set-buffer-authority "$buffer_pubkey" --new-buffer-authority 7PeqkcCXeqgsp5Mi15gjJh8qvSLk7n3dgNuyfPhJJgqY
    echo "Buffer created and authority set for $program_name"
done
