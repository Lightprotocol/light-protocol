#!/usr/bin/env sh

set -e

CIRCUITS_DIR="tests/circuits/build-circuits"
mkdir -p $CIRCUITS_DIR

CIRCUIT="poseidon"

POWERS_OF_TAU=10 # circuit will support max 2^POWERS_OF_TAU constraints
if [ ! -f $CIRCUITS_DIR/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o $CIRCUITS_DIR/ptau$POWERS_OF_TAU
fi

circom --r1cs --wasm --sym ./tests/circuits/$CIRCUIT.circom -o $CIRCUITS_DIR

yarn snarkjs groth16 setup $CIRCUITS_DIR/$CIRCUIT.r1cs $CIRCUITS_DIR/ptau$POWERS_OF_TAU $CIRCUITS_DIR/tmp_$CIRCUIT.zkey
yarn snarkjs zkey contribute $CIRCUITS_DIR/tmp_$CIRCUIT.zkey $CIRCUITS_DIR/$CIRCUIT.zkey -e="12345"
yarn snarkjs zkey verify $CIRCUITS_DIR/$CIRCUIT.r1cs $CIRCUITS_DIR/ptau$POWERS_OF_TAU $CIRCUITS_DIR/$CIRCUIT.zkey
yarn snarkjs zkey export verificationkey $CIRCUITS_DIR/$CIRCUIT.zkey $CIRCUITS_DIR/verifying_key.json