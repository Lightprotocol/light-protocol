#!/usr/bin/env bash

# Ensure we're working from the root directory of the monorepo
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
SEMAPHORE_MTB_SETUP="../semaphore-mtb-setup/semaphore-mtb-setup"

cd "$REPO_ROOT"

# Get phase 1 ptau file.
# TODO: fix when extracting keys again
# echo "Performing pre-steps..."
# cd ..
# git clone https://github.com/worldcoin/semaphore-mtb-setup
# cd semaphore-mtb-setup && go build -v
# wget https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_17.ptau 17.ptau
# mkdir -p "$REPO_ROOT/ceremony"

# $SEMAPHORE_MTB_SETUP p1i 17.ptau "$REPO_ROOT/ceremony/17.ph1"

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
    local height=$4
    local output_file=$5

    ./prover/server/light-prover r1cs \
        --circuit "$circuit" \
        --inclusion-compressed-accounts "$inclusion_accounts" \
        --non-inclusion-compressed-accounts "$non_inclusion_accounts" \
        --inclusion-tree-height "$height" \
        --non-inclusion-tree-height "$height" \
        --output "$OUTPUT_DIR/$output_file"

    echo "Generated $output_file"
}
PH2_DIR="$REPO_ROOT/../experiment/gnark-mt-setup/contributions/0016_badcryptobitch/"

# Generate R1CS for inclusion circuits
for accounts in 1 2 3 4 8; do
    generate_r1cs "inclusion" "$accounts" "0" "26" "inclusion_26_${accounts}_contribution_0.r1cs"
    ./../semaphore-mtb-setup/semaphore-mtb-setup p2n "$REPO_ROOT/ceremony/17.ph1" "$OUTPUT_DIR/inclusion_26_${accounts}_contribution_0.r1cs" "$OUTPUT_DIR/inclusion_26_${accounts}_dummy.ph1"
    ./../semaphore-mtb-setup/semaphore-mtb-setup key "${PH2_DIR}inclusion_26_${accounts}_badcryptobitch_contribution_16.ph2"
    ./prover/server/light-prover import-setup --circuit "inclusion" --inclusion-compressed-accounts "$accounts" --inclusion-tree-height 26 --pk ./../pk --vk ./../vk --output ./prover/server/proving-keys/inclusion_26_${accounts}.key
done

# Generate R1CS for non-inclusion circuits
for accounts in 1 2; do
    generate_r1cs "non-inclusion" "0" "$accounts" "26" "non-inclusion_26_${accounts}_contribution_0.r1cs"
    ./../semaphore-mtb-setup/semaphore-mtb-setup p2n "$REPO_ROOT/ceremony/17.ph1" "$OUTPUT_DIR/non-inclusion_26_${accounts}_contribution_0.r1cs" "$OUTPUT_DIR/non_inclusion_26_${accounts}_dummy.ph1"
    ./../semaphore-mtb-setup/semaphore-mtb-setup key "${PH2_DIR}non-inclusion_26_${accounts}_badcryptobitch_contribution_16.ph2"
    ./prover/server/light-prover import-setup --circuit "non-inclusion" --non-inclusion-compressed-accounts "$accounts" --non-inclusion-tree-height 26 --pk ./../pk --vk ./../vk --output ./prover/server/proving-keys/non-inclusion_26_${accounts}.key
done

# Generate R1CS for combined circuits
for inclusion_accounts in  2 3 4; do
    for non_inclusion_accounts in 1 2; do
        generate_r1cs "combined" "$inclusion_accounts" "$non_inclusion_accounts" "26" "combined_26_${inclusion_accounts}_${non_inclusion_accounts}_contribution_0.r1cs"
        ./../semaphore-mtb-setup/semaphore-mtb-setup p2n "$REPO_ROOT/ceremony/17.ph1" "$OUTPUT_DIR/combined_26_${inclusion_accounts}_${non_inclusion_accounts}_contribution_0.r1cs" "$OUTPUT_DIR/combined_26_${inclusion_accounts}_${non_inclusion_accounts}_dummy.ph1"
        ./../semaphore-mtb-setup/semaphore-mtb-setup key "${PH2_DIR}combined_26_${inclusion_accounts}_${non_inclusion_accounts}_badcryptobitch_contribution_16.ph2"
        ./prover/server/light-prover import-setup --circuit "combined" --inclusion-compressed-accounts "$inclusion_accounts" --inclusion-tree-height 26 --non-inclusion-compressed-accounts "$non_inclusion_accounts" --non-inclusion-tree-height 26 --pk ./../pk --vk ./../vk --output ./prover/server/proving-keys/combined_26_${inclusion_accounts}_${non_inclusion_accounts}.key
    done
done
