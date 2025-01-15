#!/bin/bash

# Base directory for keypairs
KEYPAIR_DIR="./target/tree-keypairs"

# Command template
CMD_TEMPLATE="cargo xtask create-state-tree --mt-pubkey {SMT} --nfq-pubkey {NFQ} --cpi-pubkey {CPI} --index {INDEX} --network devnet"

# Collect sorted key files for each type
SMT_KEYS=($(ls $KEYPAIR_DIR/smt*.json | sort))
NFQ_KEYS=($(ls $KEYPAIR_DIR/nfq*.json | sort))
CPI_KEYS=($(ls $KEYPAIR_DIR/cpi*.json | sort))

# Ensure equal number of keys for each type
if [[ ${#SMT_KEYS[@]} -ne ${#NFQ_KEYS[@]} || ${#NFQ_KEYS[@]} -ne ${#CPI_KEYS[@]} ]]; then
    echo "Error: Mismatched number of SMT, NFQ, and CPI key files."
    exit 1
fi

# Execute the command for each triple
for i in "${!SMT_KEYS[@]}"; do
    SMT_KEY="${SMT_KEYS[i]}"
    NFQ_KEY="${NFQ_KEYS[i]}"
    CPI_KEY="${CPI_KEYS[i]}"
    INDEX=$((i + 2))

    # Replace placeholders in the command template
    CMD=${CMD_TEMPLATE//\{SMT\}/"$SMT_KEY"}
    CMD=${CMD//\{NFQ\}/"$NFQ_KEY"}
    CMD=${CMD//\{CPI\}/"$CPI_KEY"}
    CMD=${CMD//\{INDEX\}/"$INDEX"}

    echo "Executing: $CMD"
    eval "$CMD"

done

echo "All commands executed."
