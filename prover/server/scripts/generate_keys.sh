#!/usr/bin/env bash

declare -a HEIGHTS=("40")
DEFAULT_STATE_HEIGHT="32"
DEFAULT_ADDRESS_HEIGHT="40"
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
    if [ "$circuit_type" == "append-with-proofs" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="append_with_proofs"
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

main() {
    declare -a append_batch_sizes_arr=("500")

    # echo "Generating proving keys..."
    for batch_size in "${append_batch_sizes_arr[@]}"; do
        echo "Generating address-append circuit for ${batch_size} COMPRESSED_ACCOUNTS with height ${height}..."
        # generate_circuit "address-append" "$DEFAULT_ADDRESS_HEIGHT" "0" "$batch_size" "0" "0"
        generate_circuit "update" "$DEFAULT_STATE_HEIGHT" "0" "$batch_size" "0" "0"
        # generate_circuit "append-with-proofs" "$DEFAULT_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    done


    # for batch_size in "${append_batch_sizes_arr[@]}"; do
    #     generate_circuit "append-with-proofs" "$DEFAULT_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    # done

    # declare -a update_batch_sizes_arr=("1" "10" "100" "500" "1000")
    # for batch_size in "${update_batch_sizes_arr[@]}"; do
    #     generate_circuit "update" "$DEFAULT_STATE_HEIGHT" "0" "$batch_size" "0" "0"
    # done

    # declare -a inclusion_compressed_accounts_arr=("1" "2" "3" "4" "8")
    # for compressed_accounts in "${inclusion_compressed_accounts_arr[@]}"; do
    #     generate_circuit "inclusion" "$DEFAULT_STATE_HEIGHT" "0" "0" "$compressed_accounts" "0"
    # done

    # declare -a non_inclusion_compressed_accounts_arr=("1" "2" "3" "4" "8")
    # for compressed_accounts in "${non_inclusion_compressed_accounts_arr[@]}"; do
    #     generate_circuit "non-inclusion" "0" "$DEFAULT_ADDRESS_HEIGHT" "0" "0" "$compressed_accounts"
    # done

    # declare -a combined_inclusion_compressed_accounts_arr=("1" "2" "3" "4")
    # declare -a combined_non_inclusion_compressed_accounts_arr=("1" "2" "3" "4")
    # for i_compressed_accounts in "${combined_inclusion_compressed_accounts_arr[@]}"; do
    #     for ni_compressed_accounts in "${combined_non_inclusion_compressed_accounts_arr[@]}"; do
    #         generate_circuit "combined" "$DEFAULT_STATE_HEIGHT" "$DEFAULT_ADDRESS_HEIGHT" "0" "$i_compressed_accounts" "$ni_compressed_accounts"
    #     done
    # done

    echo "Done."
}

main "$@"
