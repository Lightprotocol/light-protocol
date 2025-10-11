#!/usr/bin/env bash

# Light Protocol Proving Key Generation Script
# ===========================================
# This script generates proving and verifying keys for both V1 and V2 circuits.
#
# IMPORTANT: Circuit Version Determination
# ----------------------------------------
# V1 circuits (legacy): Used for height 26 trees
#   - Generated with --legacy flag
#   - Required for address tree height 26
#   - File naming: v1_combined_26_26_X_Y where X=inclusion accounts, Y=non-inclusion accounts
#
# V2 circuits: Used for heights 32 (state) and 40 (address)
#   - Generated without --legacy flag
#   - Required for address tree height 40
#   - File naming: v2_combined_32_40_X_Y where X=inclusion accounts, Y=non-inclusion accounts
#
# The prover automatically selects V1 or V2 based on address tree height:
#   - Height 26 -> V1 circuits
#   - Height 40 -> V2 circuits
#
# Usage:
#   ./generate_keys.sh           # Generate both V1 and V2 keys
#   ./generate_keys.sh --v1      # Generate only V1 keys
#   ./generate_keys.sh --v2      # Generate only V2 keys
#   ./generate_keys.sh --help    # Show help

set -e  # Exit on error

# Configuration
V1_STATE_HEIGHT="26"
V1_ADDRESS_HEIGHT="26"
V2_STATE_HEIGHT="32"
V2_ADDRESS_HEIGHT="40"
PROVING_KEYS_DIR="./proving-keys"
VERIFIER_DIR="../../program-libs/verifier/src/verifying_keys"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate_v2_circuit() {
    local circuit_type=$1
    local height=$2
    local address_tree_height=$3
    local batch_size=$4
    local inclusion_compressed_accounts=$5
    local non_inclusion_compressed_accounts=$6

    local compressed_accounts
    local circuit_type_rs
    local circuit_file
    local circuit_vkey_file
    local circuit_vkey_rs_file
    if [ "$circuit_type" == "append" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="v2_append"
    elif [ "$circuit_type" == "update" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="v2_update"
    elif [ "$circuit_type" == "address-append" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="v2_address_append"
    elif [ "$circuit_type" == "inclusion" ]; then
        compressed_accounts=$inclusion_compressed_accounts
        circuit_type_rs="v2_inclusion"
    elif [ "$circuit_type" == "non-inclusion" ]; then
        compressed_accounts=$non_inclusion_compressed_accounts
        circuit_type_rs="v2_non_inclusion"
    else
        compressed_accounts="${inclusion_compressed_accounts}_${non_inclusion_compressed_accounts}"
        circuit_type_rs="v2_combined"
    fi
    if [ "$circuit_type" == "combined" ]; then
        circuit_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${height}_${address_tree_height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${height}_${address_tree_height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${height}_${address_tree_height}_${compressed_accounts}.rs"
    elif [ "$circuit_type" == "non-inclusion" ]; then
        circuit_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${address_tree_height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${address_tree_height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${address_tree_height}_${compressed_accounts}.rs"
    else
        circuit_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/v2_${circuit_type}_${height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${height}_${compressed_accounts}.rs"
    fi

    echo "Generating V2 ${circuit_type} circuit for ${compressed_accounts} compressed accounts with state height ${height}, address height ${address_tree_height}..."
    # Note: V2 circuits do NOT use --legacy flag
    gnark setup \
        --circuit "${circuit_type}" \
        --inclusion-compressed-accounts "$inclusion_compressed_accounts" \
        --non-inclusion-compressed-accounts "$non_inclusion_compressed_accounts" \
        --inclusion-tree-height "$height" \
        --non-inclusion-tree-height "$address_tree_height" \
        --append-batch-size "${batch_size}" \
        --append-tree-height "$height" \
        --update-batch-size "${batch_size}" \
        --update-tree-height "$height" \
        --address-append-batch-size "${batch_size}" \
        --address-append-tree-height "$address_tree_height" \
        --output "${circuit_file}" \
        --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

    cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
}

generate_v1_keys() {
    echo "========================================"
    echo "Generating V1 keys (height 26)"
    echo "========================================"

    # V1 inclusion keys (v1_inclusion_26_*)
    declare -a v1_inclusion_compressed_accounts=("1" "2" "3" "4" "8")
    for compressed_accounts in "${v1_inclusion_compressed_accounts[@]}"; do
        echo "Generating V1 inclusion circuit for ${compressed_accounts} compressed accounts..."
        local circuit_file="${PROVING_KEYS_DIR}/v1_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.key"
        local circuit_vkey_file="${PROVING_KEYS_DIR}/v1_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.vkey"
        local circuit_vkey_rs_file="${VERIFIER_DIR}/v1_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.rs"

        gnark setup \
            --circuit "inclusion" \
            --legacy \
            --inclusion-compressed-accounts "$compressed_accounts" \
            --inclusion-tree-height "$V1_STATE_HEIGHT" \
            --output "${circuit_file}" \
            --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

        cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
    done

    # V1 non-inclusion keys (v1_non-inclusion_26_*)
    declare -a v1_non_inclusion_compressed_accounts=("1" "2")
    for compressed_accounts in "${v1_non_inclusion_compressed_accounts[@]}"; do
        echo "Generating V1 non-inclusion circuit for ${compressed_accounts} compressed accounts..."
        local circuit_file="${PROVING_KEYS_DIR}/v1_non-inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.key"
        local circuit_vkey_file="${PROVING_KEYS_DIR}/v1_non-inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.vkey"
        local circuit_vkey_rs_file="${VERIFIER_DIR}/v1_non_inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.rs"

        gnark setup \
            --circuit "non-inclusion" \
            --legacy \
            --non-inclusion-compressed-accounts "$compressed_accounts" \
            --non-inclusion-tree-height "$V1_ADDRESS_HEIGHT" \
            --output "${circuit_file}" \
            --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

        cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
    done

    # V1 combined keys (v1_combined_26_26_*_*)
    declare -a v1_combined_inclusion=("1" "2" "3" "4")
    declare -a v1_combined_non_inclusion=("1" "2")
    for i_compressed_accounts in "${v1_combined_inclusion[@]}"; do
        for ni_compressed_accounts in "${v1_combined_non_inclusion[@]}"; do
            echo "Generating V1 combined circuit for ${i_compressed_accounts} inclusion and ${ni_compressed_accounts} non-inclusion accounts..."
            local circuit_file="${PROVING_KEYS_DIR}/v1_combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.key"
            local circuit_vkey_file="${PROVING_KEYS_DIR}/v1_combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.vkey"
            local circuit_vkey_rs_file="${VERIFIER_DIR}/v1_combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.rs"

            gnark setup \
                --circuit "combined" \
                --legacy \
                --inclusion-compressed-accounts "$i_compressed_accounts" \
                --inclusion-tree-height "$V1_STATE_HEIGHT" \
                --non-inclusion-compressed-accounts "$ni_compressed_accounts" \
                --non-inclusion-tree-height "$V1_ADDRESS_HEIGHT" \
                --output "${circuit_file}" \
                --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

            cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
        done
    done
}

generate_v2_keys() {
    echo "========================================"
    echo "Generating V2 keys (heights 32/40)"
    echo "========================================"

    # V2 address-append keys
    declare -a address_append_batch_sizes=("10" "250")
    for batch_size in "${address_append_batch_sizes[@]}"; do
        echo "Generating V2 address-append circuit for ${batch_size} compressed accounts..."
        generate_v2_circuit "address-append" "$V2_ADDRESS_HEIGHT" "$V2_ADDRESS_HEIGHT" "$batch_size" "0" "0"
    done

    # V2 append keys
    declare -a append_batch_sizes=("10" "500")
    for batch_size in "${append_batch_sizes[@]}"; do
        generate_v2_circuit "append" "$V2_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    done

    # V2 update keys
    declare -a update_batch_sizes=("10" "500")
    for batch_size in "${update_batch_sizes[@]}"; do
        generate_v2_circuit "update" "$V2_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    done

    # V2 inclusion keys
    for compressed_accounts in $(seq 1 20); do
        generate_v2_circuit "inclusion" "$V2_STATE_HEIGHT" "0" "0" "$compressed_accounts" "0"
    done

    # V2 non-inclusion keys
    for compressed_accounts in $(seq 1 32); do
        generate_v2_circuit "non-inclusion" "0" "$V2_ADDRESS_HEIGHT" "0" "0" "$compressed_accounts"
    done

    # V2 combined keys
    declare -a combined_inclusion_compressed_accounts=("1" "2" "3" "4")
    declare -a combined_non_inclusion_compressed_accounts=("1" "2" "3" "4")
    for i_compressed_accounts in "${combined_inclusion_compressed_accounts[@]}"; do
        for ni_compressed_accounts in "${combined_non_inclusion_compressed_accounts[@]}"; do
            generate_v2_circuit "combined" "$V2_STATE_HEIGHT" "$V2_ADDRESS_HEIGHT" "0" "$i_compressed_accounts" "$ni_compressed_accounts"
        done
    done
}

main() {
    # Ensure directories exist
    mkdir -p "$PROVING_KEYS_DIR"
    mkdir -p "$VERIFIER_DIR"

    # Parse command line arguments
    local generate_v1=false
    local generate_v2=false

    if [[ $# -eq 0 ]]; then
        # No arguments, generate both
        generate_v1=true
        generate_v2=true
    else
        while [[ $# -gt 0 ]]; do
            case $1 in
                --v1)
                    generate_v1=true
                    shift
                    ;;
                --v2)
                    generate_v2=true
                    shift
                    ;;
                --all)
                    generate_v1=true
                    generate_v2=true
                    shift
                    ;;
                --help)
                    echo "Usage: $0 [OPTIONS]"
                    echo "Options:"
                    echo "  --v1     Generate V1 keys (height 26)"
                    echo "  --v2     Generate V2 keys (heights 32/40)"
                    echo "  --all    Generate both V1 and V2 keys (default)"
                    echo "  --help   Show this help message"
                    exit 0
                    ;;
                *)
                    echo "Unknown option: $1"
                    echo "Use --help for usage information"
                    exit 1
                    ;;
            esac
        done
    fi

    # Generate requested keys
    if [[ "$generate_v1" == true ]]; then
        generate_v1_keys
    fi

    if [[ "$generate_v2" == true ]]; then
        generate_v2_keys
    fi

    echo "========================================"
    echo "Key generation complete!"
    echo "========================================"

    # Generate checksums for the new keys
    echo ""
    echo "Generating checksums..."
    if command -v python3 &> /dev/null; then
        python3 ./scripts/generate_checksums.py
        echo "Checksums generated successfully in ${PROVING_KEYS_DIR}/CHECKSUM file"
    else
        echo "Warning: Python3 not found. Please run 'python3 scripts/generate_checksums.py' manually to update checksums"
    fi
}

main "$@"
