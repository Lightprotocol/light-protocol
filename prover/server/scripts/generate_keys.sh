#!/usr/bin/env bash

# Light Protocol Proving Key Generation Script
# ===========================================
# This script generates proving and verifying keys for both V1 and V2 circuits.
#
# V1 keys: Use height 26 for both state and address trees (legacy)
# V2 keys: Use height 32 for state tree and height 40 for address tree (current)
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

generate_circuit() {
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
        circuit_type_rs="append"
    elif [ "$circuit_type" == "update" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="update"
    elif [ "$circuit_type" == "address-append" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="address_append"
    elif [ "$circuit_type" == "inclusion" ]; then
        compressed_accounts=$inclusion_compressed_accounts
        circuit_type_rs="inclusion"
    elif [ "$circuit_type" == "non-inclusion" ]; then
        compressed_accounts=$non_inclusion_compressed_accounts
        circuit_type_rs="non_inclusion"
    else
        compressed_accounts="${inclusion_compressed_accounts}_${non_inclusion_compressed_accounts}"
        circuit_type_rs="combined"
    fi
    if [ "$circuit_type" == "combined" ]; then
        circuit_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${address_tree_height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${address_tree_height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${height}_${address_tree_height}_${compressed_accounts}.rs"
    elif [ "$circuit_type" == "non-inclusion" ]; then
        circuit_file="${PROVING_KEYS_DIR}/${circuit_type}_${address_tree_height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/${circuit_type}_${address_tree_height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${address_tree_height}_${compressed_accounts}.rs"
    else
        circuit_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${compressed_accounts}.key"
        circuit_vkey_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${compressed_accounts}.vkey"
        circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${height}_${compressed_accounts}.rs"
    fi

    echo "Generating ${circuit_type} circuit for ${compressed_accounts} COMPRESSED_ACCOUNTS with height ${height}..."
    echo "non_inclusion_compressed_accounts: ${non_inclusion_compressed_accounts}"
    # Fixed variable references for batch sizes
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
        --address-append-tree-height "$height" \
        --output "${circuit_file}" \
        --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

    cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
}

generate_v1_keys() {
    echo "========================================"
    echo "Generating V1 keys (height 26)"
    echo "========================================"

    # V1 inclusion keys (mainnet_inclusion_26_*)
    declare -a v1_inclusion_compressed_accounts=("1" "2" "3" "4" "8")
    for compressed_accounts in "${v1_inclusion_compressed_accounts[@]}"; do
        echo "Generating V1 inclusion circuit for ${compressed_accounts} compressed accounts..."
        # Note: V1 inclusion keys are named with 'mainnet_inclusion' prefix
        local circuit_file="${PROVING_KEYS_DIR}/mainnet_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.key"
        local circuit_vkey_file="${PROVING_KEYS_DIR}/mainnet_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.vkey"
        local circuit_vkey_rs_file="${VERIFIER_DIR}/mainnet_inclusion_${V1_STATE_HEIGHT}_${compressed_accounts}.rs"

        gnark setup \
            --circuit "inclusion" \
            --legacy \
            --inclusion-compressed-accounts "$compressed_accounts" \
            --inclusion-tree-height "$V1_STATE_HEIGHT" \
            --output "${circuit_file}" \
            --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

        cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
    done

    # V1 non-inclusion keys (non-inclusion_26_*)
    declare -a v1_non_inclusion_compressed_accounts=("1" "2")
    for compressed_accounts in "${v1_non_inclusion_compressed_accounts[@]}"; do
        echo "Generating V1 non-inclusion circuit for ${compressed_accounts} compressed accounts..."
        local circuit_file="${PROVING_KEYS_DIR}/non-inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.key"
        local circuit_vkey_file="${PROVING_KEYS_DIR}/non-inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.vkey"
        local circuit_vkey_rs_file="${VERIFIER_DIR}/non_inclusion_${V1_ADDRESS_HEIGHT}_${compressed_accounts}.rs"

        gnark setup \
            --circuit "non-inclusion" \
            --legacy \
            --non-inclusion-compressed-accounts "$compressed_accounts" \
            --non-inclusion-tree-height "$V1_ADDRESS_HEIGHT" \
            --output "${circuit_file}" \
            --output-vkey "${circuit_vkey_file}" || { echo "Error: gnark setup failed"; exit 1; }

        cargo xtask generate-vkey-rs --input-path "${circuit_vkey_file}" --output-path "${circuit_vkey_rs_file}" || { echo "Error: cargo xtask generate-vkey-rs failed"; exit 1; }
    done

    # V1 combined keys (combined_26_*_*)
    declare -a v1_combined_inclusion=("1" "2" "3" "4")
    declare -a v1_combined_non_inclusion=("1" "2")
    for i_compressed_accounts in "${v1_combined_inclusion[@]}"; do
        for ni_compressed_accounts in "${v1_combined_non_inclusion[@]}"; do
            echo "Generating V1 combined circuit for ${i_compressed_accounts} inclusion and ${ni_compressed_accounts} non-inclusion accounts..."
            local circuit_file="${PROVING_KEYS_DIR}/combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.key"
            local circuit_vkey_file="${PROVING_KEYS_DIR}/combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.vkey"
            local circuit_vkey_rs_file="${VERIFIER_DIR}/combined_${V1_STATE_HEIGHT}_${V1_ADDRESS_HEIGHT}_${i_compressed_accounts}_${ni_compressed_accounts}.rs"

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
        generate_circuit "address-append" "$V2_ADDRESS_HEIGHT" "0" "$batch_size" "0" "0"
    done

    # V2 append keys
    declare -a append_batch_sizes=("10" "500")
    for batch_size in "${append_batch_sizes[@]}"; do
        generate_circuit "append" "$V2_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    done

    # V2 update keys
    declare -a update_batch_sizes=("10" "500")
    for batch_size in "${update_batch_sizes[@]}"; do
        generate_circuit "update" "$V2_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    done

    # V2 inclusion keys
    declare -a inclusion_compressed_accounts=("1" "2" "3" "4" "8")
    for compressed_accounts in "${inclusion_compressed_accounts[@]}"; do
        generate_circuit "inclusion" "$V2_STATE_HEIGHT" "0" "0" "$compressed_accounts" "0"
    done

    # V2 non-inclusion keys
    declare -a non_inclusion_compressed_accounts=("1" "2" "3" "4" "8")
    for compressed_accounts in "${non_inclusion_compressed_accounts[@]}"; do
        generate_circuit "non-inclusion" "0" "$V2_ADDRESS_HEIGHT" "0" "0" "$compressed_accounts"
    done

    # V2 combined keys
    declare -a combined_inclusion_compressed_accounts=("1" "2" "3" "4")
    declare -a combined_non_inclusion_compressed_accounts=("1" "2" "3" "4")
    for i_compressed_accounts in "${combined_inclusion_compressed_accounts[@]}"; do
        for ni_compressed_accounts in "${combined_non_inclusion_compressed_accounts[@]}"; do
            generate_circuit "combined" "$V2_STATE_HEIGHT" "$V2_ADDRESS_HEIGHT" "0" "$i_compressed_accounts" "$ni_compressed_accounts"
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
        echo "Checksums generated successfully in CHECKSUM file"
    else
        echo "Warning: Python3 not found. Please run 'python3 scripts/generate_checksums.py' manually to update checksums"
    fi
}

main "$@"
