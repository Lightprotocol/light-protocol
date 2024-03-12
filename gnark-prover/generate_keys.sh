#!/usr/bin/env sh

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

    if [ ! -f "${CIRCUIT_FILE}" ]; then
        echo "Generating ${CIRCUIT_TYPE} circuit for ${UTXOS} UTXOs..."
        gnark setup --circuit "${CIRCUIT_TYPE}" --utxos "$UTXOS" --tree-depth "$DEPTH" --output "${CIRCUIT_FILE}"
    fi
}

declare -a utxos_arr=("1" "2" "3" "4" "8")

for utxos in "${utxos_arr[@]}"
do
    generate $utxos "inclusion"
done