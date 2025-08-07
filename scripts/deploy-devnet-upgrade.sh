# assumes that programs have been build with build-verifiable.sh
# Creates buffer accounts
# Buffer account addresses can be used in multisig action

# Array of program names
libraries=("account_compression" "light_compressed_token" "light_system_program_pinocchio" "light_registry")
program_ids=("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq" "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m" "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7" "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX")

BUFFER_KEYPAIR_PATH="target/buffer"


create_buffer_account() {
    local max_retries=5
    local attempt=1

    local program_name="$1"
    local program_id="$2"

    while (( attempt <= max_retries )); do
        echo "Attempt $attempt of $max_retries..."
        echo "$BUFFER_KEYPAIR_PATH/$program_name-keypair.json"
        echo "Program ID for $program_name: $program_id"
        if solana program deploy target/deploy/"$program_name".so --program-id $program_id --buffer "$BUFFER_KEYPAIR_PATH/$program_name-keypair.json" --upgrade-authority ../../.config/solana/id.json; then
            echo "Command succeeded on attempt $attempt."
            return 0
        else
            echo "Command failed on attempt $attempt."
            ((attempt++))
            sleep 2
        fi
        ((attempt++))
    done

    echo "Command failed after $max_retries attempts."
    return 1
}



# Iterate over each program and create buffer accounts
for i in "${!program_ids[@]}"; do
    program_id="${program_ids[$i]}"
    program_name="${libraries[$i]}"

    if [[ ! -f "$BUFFER_KEYPAIR_PATH/$program_name-keypair.json" ]]; then
        solana-keygen new --outfile "$BUFFER_KEYPAIR_PATH/$program_name-keypair.json" --no-bip39-passphrase
    fi
    create_buffer_account "$program_name" "$program_id"
done
