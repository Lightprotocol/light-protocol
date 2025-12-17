#!/bin/bash

# Base directory for keypairs
KEYPAIR_DIR="../light-keypairs/batched-tree-keypairs"

# Command template
CMD_TEMPLATE="cargo run --bin xtask create-batch-state-tree --mt-pubkey {BMT} --nfq-pubkey {OQ} --cpi-pubkey {CPI} --index {INDEX} --network mainnet"

# Collect sorted key files for each type
BMT_KEYS=($(ls $KEYPAIR_DIR/bmt*.json | sort))
OQ_KEYS=($(ls $KEYPAIR_DIR/oq*.json | sort))
CPI_KEYS=($(ls $KEYPAIR_DIR/cpi*.json | sort))

# Ensure equal number of keys for each type
if [[ ${#BMT_KEYS[@]} -ne ${#OQ_KEYS[@]} || ${#OQ_KEYS[@]} -ne ${#CPI_KEYS[@]} ]]; then
    echo "Error: Mismatched number of BMT, OQ, and CPI key files."
    exit 1
fi

# Execute the command for each triple
for i in "${!BMT_KEYS[@]}"; do
    BMT_KEY="${BMT_KEYS[i]}"
    OQ_KEY="${OQ_KEYS[i]}"
    CPI_KEY="${CPI_KEYS[i]}"
    INDEX=$((i + 1))

    # Replace placeholders in the command template
    CMD=${CMD_TEMPLATE//\{BMT\}/"$BMT_KEY"}
    CMD=${CMD//\{OQ\}/"$OQ_KEY"}
    CMD=${CMD//\{CPI\}/"$CPI_KEY"}
    CMD=${CMD//\{INDEX\}/"$INDEX"}

    echo "Executing: $CMD"
    eval "$CMD"

done

echo "All commands executed."
