#!/bin/bash

# Script to regenerate all .vkey.rs files from .key files
# Usage: ./regenerate_all_vkeys.sh

set -e

PROVING_KEYS_DIR="./proving-keys"
VKEY_OUTPUT_DIR="../../program-libs/verifier/src/verifying_keys"

echo "Regenerating all verification keys..."
echo "========================================"

# Counter for progress
total=$(find "$PROVING_KEYS_DIR" -name "*.vkey" | wc -l | tr -d ' ')
current=0

for key_file in "$PROVING_KEYS_DIR"/*.vkey; do
    current=$((current + 1))

    # Extract the base name without extension
    base_name=$(basename "$key_file" .vkey)

    # Determine the output path
    output_file="$VKEY_OUTPUT_DIR/${base_name}.rs"

    echo "[$current/$total] Processing: $base_name"

    # Run xtask from repo root
    (cd ../.. && cargo run --package xtask -- generate-vkey-rs \
        --input-path "prover/server/$key_file" \
        --output-path "program-libs/verifier/src/verifying_keys/${base_name}.rs")
done

echo ""
echo "========================================"
echo "Completed! Regenerated $total verification keys."