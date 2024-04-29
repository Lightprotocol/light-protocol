#!/usr/bin/env bash

DEPTH="26"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate() {
    local INCLUSION_UTXOS=$1
    local NON_INCLUSION_UTXOS=$2
    local CIRCUIT_TYPE=$3
    mkdir -p circuits
    if [ "$CIRCUIT_TYPE" == "inclusion" ]; then
        UTXOS=$INCLUSION_UTXOS
        CIRCUIT_TYPE_RS="inclusion"
    elif [ "$CIRCUIT_TYPE" == "non-inclusion" ]; then
        UTXOS=$NON_INCLUSION_UTXOS
        # rust file names cannot include dashes
        CIRCUIT_TYPE_RS="non_inclusion"
    else
        UTXOS="${INCLUSION_UTXOS}_${NON_INCLUSION_UTXOS}"
        CIRCUIT_TYPE_RS="combined"
    fi
    CIRCUIT_FILE="./circuits/${CIRCUIT_TYPE}_${DEPTH}_${UTXOS}.key"
    CIRCUIT_VKEY_FILE="./circuits/${CIRCUIT_TYPE}_${DEPTH}_${UTXOS}.vkey"
    CIRCUIT_VKEY_RS_FILE="../programs/compressed-pda/src/verifying_keys/${CIRCUIT_TYPE_RS}_${DEPTH}_${UTXOS}.rs"

    echo "Generating ${CIRCUIT_TYPE} circuit for ${UTXOS} UTXOs..."
    echo "go run . setup --circuit ${CIRCUIT_TYPE} --inclusion-utxos ${INCLUSION_UTXOS} --non-inclusion-utxos ${NON_INCLUSION_UTXOS} --inclusion-tree-depth ${DEPTH} --non-inclusion-tree-depth ${DEPTH} --output ${CIRCUIT_FILE} --output-vkey ${CIRCUIT_VKEY_FILE}"

    gnark setup \
      --circuit "${CIRCUIT_TYPE}" \
      --inclusion-utxos "$INCLUSION_UTXOS" \
      --non-inclusion-utxos "$NON_INCLUSION_UTXOS" \
      --inclusion-tree-depth "$DEPTH" \
      --non-inclusion-tree-depth "$DEPTH" \
      --output "${CIRCUIT_FILE}" \
      --output-vkey "${CIRCUIT_VKEY_FILE}"
    cargo xtask generate-vkey-rs --input-path "${CIRCUIT_VKEY_FILE}" --output-path "${CIRCUIT_VKEY_RS_FILE}"
}

declare -a inclusion_utxos_arr=("1" "2" "3" "4" "8")

for utxos in "${inclusion_utxos_arr[@]}"
do
    generate "$utxos" "0" "inclusion"
done

declare -a non_inclusion_utxos_arr=("1" "2")

for utxos in "${non_inclusion_utxos_arr[@]}"
do
    generate "0" "$utxos" "non-inclusion"
done

declare -a combined_inclusion_utxos_arr=("1" "2" "3" "4")
declare -a combined_non_inclusion_utxos_arr=("1" "2")

for i_utxos in "${combined_inclusion_utxos_arr[@]}"
do
  for ni_utxos in "${combined_non_inclusion_utxos_arr[@]}"
  do
    generate "$i_utxos" "$ni_utxos" "combined"
  done
done