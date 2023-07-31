#!/usr/bin/env sh

set -e

POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
mkdir -p build
if [ ! -f build/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o build/ptau$POWERS_OF_TAU
fi
CIRCUIT_NAME= MockVerifierTransaction
circom --r1cs --wasm --sym ./circuit/CIRCUIT_NAME.circom -o ./sdk/build-circuit/

yarn snarkjs groth16 setup ./sdk/build-circuit/CIRCUIT_NAME.r1cs build/ptau$POWERS_OF_TAU ./sdk/build-circuit/CIRCUIT_NAME.zkey
yarn snarkjs zkey export verificationkey ./sdk/build-circuit/CIRCUIT_NAME.zkey ./sdk/build-circuit/verifyingkey.json

ts-node ./scripts/createRustVerifyingKey.ts app verifier CIRCUIT_NAME # TODO: needs to be assigned at template creation

rm ./sdk/build-circuit/verifyingkey.json
rm ./sdk/build-circuit/CIRCUIT_NAME.r1cs
rm ./sdk/build-circuit/CIRCUIT_NAME.sym
