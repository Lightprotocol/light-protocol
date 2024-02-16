#!/bin/bash -e
function download_ptau {
  directory="$1"
  ptau_number="$2"

  if [ ! -f "$directory/ptau$ptau_number" ]; then
    echo "Downloading powers of tau file"
    curl -L "https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$ptau_number.ptau" --create-dirs -o "./$directory/ptau$ptau_number" || { echo "Download failed"; exit 1; }
  fi
}

function execute_commands {
  merkle_number="$1"
  utxo_count="$2"
  ptau_number="$3"

  if [[ $# -ne 3 ]]; then
    echo "Invalid number of arguments"
    exit 1;
  fi

  build_directory="test-data/merkle${merkle_number}_$utxo_count"
  src_directory="src/verifying_keys/merkle${merkle_number}_$utxo_count"
  mkdir -p "$build_directory"
  mkdir -p "$src_directory"
  download_ptau "build" "$ptau_number" || { echo "download_ptau failed"; exit 1; }
  circom --r1cs --wasm --sym "circuits/merkle/merkle${merkle_number}_$utxo_count.circom" -o "$build_directory" || { echo "circom failed"; exit 1; }
  npx snarkjs groth16 setup "$build_directory/merkle${merkle_number}_$utxo_count.r1cs" "build/ptau$ptau_number" "$build_directory/tmp_circuit.zkey" || { echo "snarkjs groth16 setup failed"; exit 1; }
  npx snarkjs zkey contribute "$build_directory/tmp_circuit.zkey" "$build_directory/circuit.zkey" -e="321432151325321543215" || { echo "snarkjs zkey contribute failed"; exit 1; }
  rm "$build_directory/tmp_circuit.zkey"
  npx snarkjs zkey verify "$build_directory/merkle${merkle_number}_$utxo_count.r1cs" "build/ptau$ptau_number" "$build_directory/circuit.zkey" || { echo "snarkjs zkey verify failed"; exit 1; }
  npx snarkjs zkey export verificationkey "$build_directory/circuit.zkey" "$build_directory/merkle${merkle_number}_$utxo_count.json"
  npx node parse_vk_to_rust.js "$build_directory/merkle${merkle_number}_$utxo_count.json" "$src_directory"
  echo "mod merkle${merkle_number}_$utxo_count;" >> "src/verifying_keys/mod.rs";
  echo "pub use crate::verifying_keys::merkle${merkle_number}_$utxo_count::VERIFYINGKEY as VK${merkle_number}_$utxo_count;" >> "src/verifying_keys/mod.rs";
}

rm src/verifying_keys/mod.rs
touch src/verifying_keys/mod.rs
echo "mod helpers;" >> "src/verifying_keys/mod.rs";
echo "pub use crate::verifying_keys::helpers::{get_zk_bytes, vk};" >> "src/verifying_keys/mod.rs";

POWERS_OF_TAU=16
MAX_COUNT=3

MERKLE_TREE_HEIGHT=22
for ((i=1; i<=MAX_COUNT; i++)); do
  execute_commands "$MERKLE_TREE_HEIGHT" "$i" "$POWERS_OF_TAU" || exit
done
#
#POWERS_OF_TAU=18
#MERKLE_TREE_HEIGHT=30
#for ((i=1; i<=MAX_COUNT; i++)); do
#  execute_commands "$MERKLE_TREE_HEIGHT" "$i" "$POWERS_OF_TAU" || exit
#done