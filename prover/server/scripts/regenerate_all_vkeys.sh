#!/bin/bash

# Script to regenerate all .vkey.rs files from .key files
# Usage: ./regenerate_all_vkeys.sh

set -e

PROVING_KEYS_DIR="./proving-keys"
VKEY_OUTPUT_DIR="../../program-libs/verifier/src/verifying_keys"

echo "Regenerating all verification keys..."
echo "========================================"

# Counter for progress
total=$(find "$PROVING_KEYS_DIR" -name "*.key" | wc -l | tr -d ' ')
current=0

for key_file in "$PROVING_KEYS_DIR"/*.key; do
    current=$((current + 1))

    # Extract the base name without extension
    base_name=$(basename "$key_file" .key)

    # Determine the output path
    output_file="$VKEY_OUTPUT_DIR/${base_name}.rs"

    echo "[$current/$total] Processing: $base_name"

    # Run cargo xtask
    cargo xtask generate-vkey-rs \
        --input-path "$key_file" \
        --output-path "$output_file"
done

echo ""
echo "========================================"
echo "Completed! Regenerated $total verification keys."