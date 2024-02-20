#!/bin/bash -e

function execute_commands {
  merkle_number="$1"
  utxo_count="$2"

  build_directory="$CIRCUIT_RS_DIR/test-data/merkle${merkle_number}_$utxo_count"
  build_js_directory=$build_directory/merkle${merkle_number}_${utxo_count}_js

  npx node "$build_js_directory/generate_witness.js" \
    "$build_js_directory/merkle${merkle_number}_${utxo_count}.wasm" \
    "$build_js_directory/../inputs${merkle_number}_${utxo_count}.json" \
    "$build_js_directory/../${merkle_number}_${utxo_count}.wtns"

}

REPO_TOP_DIR=$(git rev-parse --show-toplevel)

CIRCUIT_RS_DIR="$REPO_TOP_DIR/circuit-lib/circuitlib-rs"

MAX_COUNT=3

MERKLE_TREE_HEIGHT=22
for ((i=1; i<=MAX_COUNT; i++)); do
  execute_commands "$MERKLE_TREE_HEIGHT" "$i" || exit
done

#
#POWERS_OF_TAU=18
#MERKLE_TREE_HEIGHT=30
#for ((i=1; i<=MAX_COUNT; i++)); do
#  execute_commands "$MERKLE_TREE_HEIGHT" "$i" "$POWERS_OF_TAU" || exit
#done
