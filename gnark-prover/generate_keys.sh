#!/usr/bin/env bash

DEPTH="26"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate() {
    local UTXOS=$1
    local CIRCUIT_TYPE=$2
    mkdir -p circuits
    CIRCUIT_FILE="./circuits/${CIRCUIT_TYPE}_${DEPTH}_${UTXOS}.key"

    echo "Generating ${CIRCUIT_TYPE} circuit for ${UTXOS} UTXOs..."
    gnark setup --utxos "$UTXOS" --tree-depth "$DEPTH" --output "${CIRCUIT_FILE}"
    
}

declare -a utxos_arr=("1" "2" "3" "4" "8")

for utxos in "${utxos_arr[@]}"
do
    generate $utxos "inclusion"
done