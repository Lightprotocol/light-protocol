#!/usr/bin/env bash

declare -a HEIGHTS=("26")
DEFAULT_HEIGHT="26"
PROVING_KEYS_DIR="./proving-keys"
VERIFIER_DIR="../circuit-lib/verifier/src/verifying_keys"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate_circuit() {
    local circuit_type=$1
    local height=$2
    local batch_size=$3
    local inclusion_compressed_accounts=$4
    local non_inclusion_compressed_accounts=$5

    local compressed_accounts
    local circuit_type_rs
    if [ "$circuit_type" == "append-with-subtrees" ]; then
        compressed_accounts=$batch_size
        circuit_type_rs="append_with_subtrees"
    elif [ "$circuit_type" == "append-with-proofs" ]; then
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

    local circuit_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${compressed_accounts}.key"
    local circuit_vkey_file="${PROVING_KEYS_DIR}/${circuit_type}_${height}_${compressed_accounts}.vkey"
    local circuit_vkey_rs_file="${VERIFIER_DIR}/${circuit_type_rs}_${height}_${compressed_accounts}.rs"

    echo "Generating ${circuit_type} circuit for ${compressed_accounts} COMPRESSED_ACCOUNTS with height ${height}..."

    # Fixed variable references for batch sizes
    gnark setup \
        --circuit "${circuit_type}" \
        --inclusion-compressed-accounts "$inclusion_compressed_accounts" \
        --non-inclusion-compressed-accounts "$non_inclusion_compressed_accounts" \
        --inclusion-tree-height "$height" \
        --non-inclusion-tree-height "$height" \
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
    declare -a append_batch_sizes_arr=("1" "10" "100" "500" "1000")
    
    echo "Generating proving keys..."
    for height in "${HEIGHTS[@]}"; do
        for batch_size in "${append_batch_sizes_arr[@]}"; do
            echo "Generating address-append circuit for ${batch_size} COMPRESSED_ACCOUNTS with height ${height}..."
            generate_circuit "address-append" "$height" "$batch_size" "0" "0" 
        done
    done

    for height in "${HEIGHTS[@]}"; do
        for batch_size in "${append_batch_sizes_arr[@]}"; do
            generate_circuit "append-with-proofs" "$height" "$batch_size" "0" "0"
        done
    done

    declare -a append_batch_sizes_arr=("1" "10" "100" "500" "1000")
    for height in "${HEIGHTS[@]}"; do
        for batch_size in "${append_batch_sizes_arr[@]}"; do
            generate_circuit "append-with-subtrees" "$height" "$batch_size" "0" "0"
        done
    done

    declare -a update_batch_sizes_arr=("1" "10" "100" "500" "1000")
    for height in "${HEIGHTS[@]}"; do
        for batch_size in "${update_batch_sizes_arr[@]}"; do
            generate_circuit "update" "$height" "$batch_size" "0" "0"
        done
    done

    declare -a inclusion_compressed_accounts_arr=("1" "2" "3" "4" "8")
    for compressed_accounts in "${inclusion_compressed_accounts_arr[@]}"; do
        generate_circuit "inclusion" "$DEFAULT_HEIGHT" "0" "$compressed_accounts" "0"
    done

    declare -a non_inclusion_compressed_accounts_arr=("1" "2")
    for compressed_accounts in "${non_inclusion_compressed_accounts_arr[@]}"; do
        generate_circuit "non-inclusion" "$DEFAULT_HEIGHT" "0" "$compressed_accounts"
    done

    declare -a combined_inclusion_compressed_accounts_arr=("1" "2" "3" "4")
    declare -a combined_non_inclusion_compressed_accounts_arr=("1" "2")
    for i_compressed_accounts in "${combined_inclusion_compressed_accounts_arr[@]}"; do
        for ni_compressed_accounts in "${combined_non_inclusion_compressed_accounts_arr[@]}"; do
            generate_circuit "combined" "$DEFAULT_HEIGHT" "0" "$i_compressed_accounts" "$ni_compressed_accounts"
        done
    done

    echo "Done."
}

main "$@"
