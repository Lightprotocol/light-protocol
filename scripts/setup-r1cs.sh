#!/bin/bash

# Ensure we're working from the root directory of the monorepo
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
cd "$REPO_ROOT"

# Get phase 1 ptau file.
echo "Performing pre-steps..."
cd ..
git clone https://github.com/worldcoin/semaphore-mtb-setup
cd semaphore-mtb-setup && go build -v
wget https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_16.ptau
mkdir -p "$REPO_ROOT/ceremony"
mv powersOfTau28_hez_final_16.ptau "$REPO_ROOT/ceremony/16.ph1"
cd "$REPO_ROOT"

# Set the output directory
OUTPUT_DIR="$REPO_ROOT/ceremony/r1cs"

# Delete prior contents if the directory exists
if [ -d "$OUTPUT_DIR" ]; then
    rm -rf "$OUTPUT_DIR"/*
fi

# Create the r1cs directory
mkdir -p "$OUTPUT_DIR"

# Function to generate R1CS for a given circuit type and parameters
generate_r1cs() {
    local circuit=$1
    local inclusion_accounts=$2
    local non_inclusion_accounts=$3
    local depth=$4
    local output_file=$5

    ./light-prover/light-prover r1cs \
        --circuit "$circuit" \
        --inclusion-compressed-accounts "$inclusion_accounts" \
        --non-inclusion-compressed-accounts "$non_inclusion_accounts" \
        --inclusion-tree-depth "$depth" \
        --non-inclusion-tree-depth "$depth" \
        --output "$OUTPUT_DIR/$output_file"

    echo "Generated $output_file"
}

# Generate R1CS for inclusion circuits
for accounts in 1 2 3 4 8; do
    generate_r1cs "inclusion" "$accounts" "0" "26" "inclusion_26_${accounts}.r1cs"
done

# Generate R1CS for non-inclusion circuits
for accounts in 1 2; do
    generate_r1cs "non-inclusion" "0" "$accounts" "26" "non-inclusion_26_${accounts}.r1cs"
done

# Generate R1CS for combined circuits
for inclusion_accounts in 1 2 3 4; do
    for non_inclusion_accounts in 1 2; do
        generate_r1cs "combined" "$inclusion_accounts" "$non_inclusion_accounts" "26" "combined_26_${inclusion_accounts}_${non_inclusion_accounts}.r1cs"
    done
done

echo "All R1CS files have been generated in $OUTPUT_DIR"

# Run semaphore-mtb-setup for each R1CS file
SEMAPHORE_MTB_SETUP="../semaphore-mtb-setup/semaphore-mtb-setup"
PH2_OUTPUT_DIR="$REPO_ROOT/ceremony/ph2-files"
PH1_FILE="$REPO_ROOT/ceremony/16.ph1"

# Create the ph2-files directory if it doesn't exist
mkdir -p "$PH2_OUTPUT_DIR"

# Function to process each R1CS file
process_r1cs() {
    local r1cs_file=$1
    local base_name=$(basename "$r1cs_file" .r1cs)
    local ph2_file="$PH2_OUTPUT_DIR/${base_name}.ph2"

    $SEMAPHORE_MTB_SETUP p2n "$PH1_FILE" "$r1cs_file" "$ph2_file"
    echo "Processed $r1cs_file -> $ph2_file"
}

# Process all R1CS files
for r1cs_file in "$OUTPUT_DIR"/*.r1cs; do
    process_r1cs "$r1cs_file"
done

echo "All .ph2 files have been generated in $PH2_OUTPUT_DIR"
