#!/bin/bash -e

function download_ptau {
  directory="$1"
  ptau_number="$2"

  if [ ! -f "$directory/ptau$ptau_number" ]; then
    echo "Downloading powers of tau file"
    curl -L "https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$ptau_number.ptau" --create-dirs -o "$directory/ptau$ptau_number" || { echo "Download failed"; exit 1; }
  fi
}

function execute_commands {
  echo "Building non-inclusion merkle tree with height $1 and $2 UTXOs..."

  merkle_number="$1"
  utxo_count="$2"
  ptau_number="$3"

  if [[ $# -ne 3 ]]; then
    echo "Invalid number of arguments"
    exit 1;
  fi

  temp_directory="/tmp"
  name="ni_${merkle_number}_$utxo_count"
  build_directory="$CIRCUIT_RS_DIR/test-data/$name"
  src_directory="$CIRCUIT_RS_VERIFYINGKEY_DIR/$name"
  circuits_circom_directory="$REPO_TOP_DIR/circuit-lib/circuit-lib.circom"
  mkdir -p "$build_directory"
  mkdir -p "$src_directory"

  download_ptau "$temp_directory" "$ptau_number" || { echo "download_ptau failed"; exit 1; }
  echo "Compiling circuits..."
  circom --r1cs --wasm --sym "$circuits_circom_directory/src/non-inclusion-merkle-tree/$name.circom" \
    -o "$temp_directory" -l "$circuits_circom_directory/node_modules/circomlib/circuits" || { echo "circom failed"; exit 1; }
  
  echo "Generating keys..."
  npx snarkjs groth16 setup "$temp_directory/$name.r1cs" "$temp_directory/ptau$ptau_number" "$temp_directory/tmp_$name.zkey" \
    || { echo "snarkjs groth16 setup failed"; exit 1; }
  
  echo "Contributing to powers of tau..."
  npx snarkjs zkey contribute "$temp_directory/tmp_$name.zkey" "$temp_directory/$name.zkey" -e="321432151325321543215" \
    || { echo "snarkjs zkey contribute failed"; exit 1; }
  rm "$temp_directory/tmp_$name.zkey"
  
  echo "Verifying proof..."
  npx snarkjs zkey verify "$temp_directory/$name.r1cs" "$temp_directory/ptau$ptau_number" "$temp_directory/$name.zkey" || { echo "snarkjs zkey verify failed"; exit 1; }

  echo "Exporting verification key..."
  npx snarkjs zkey export verificationkey "$temp_directory/$name.zkey" "$temp_directory/$name.json"

  cp "$temp_directory/$name.zkey" "$build_directory/$name.zkey"
  cp "$temp_directory/${name}_js/$name.wasm" "$build_directory/circuit.wasm"

  echo "Parsing verification key to Rust..."
  npx ts-node "$circuits_circom_directory/scripts/parseVerifyingKeyToRust.js" "$temp_directory/$name.json" "$src_directory"
  echo "mod $name;" >> "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs";
  echo "pub use crate::verifying_keys::$name::VERIFYINGKEY as VK_$name;" >> "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs";

  echo "Done"
}

REPO_TOP_DIR=$(git rev-parse --show-toplevel)

CIRCUIT_RS_DIR="$REPO_TOP_DIR/circuit-lib/circuitlib-rs"
CIRCUIT_RS_VERIFYINGKEY_DIR="$CIRCUIT_RS_DIR/src/verifying_keys"

rm "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs"
touch "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs"
echo "mod helpers;" >> "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs";
echo "pub use crate::verifying_keys::helpers::vk;" >> "$CIRCUIT_RS_VERIFYINGKEY_DIR/mod.rs";

POWERS_OF_TAU=16
MAX_COUNT=4

MERKLE_TREE_HEIGHT=26
for ((i=1; i<=MAX_COUNT; i++)); do
  execute_commands "$MERKLE_TREE_HEIGHT" "$i" "$POWERS_OF_TAU" || exit
done

execute_commands "$MERKLE_TREE_HEIGHT" 8 "$POWERS_OF_TAU" || exit
