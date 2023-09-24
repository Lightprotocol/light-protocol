#!/usr/bin/env sh

set -e

POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
if [ ! -f ./ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o ./ptau$POWERS_OF_TAU
fi

circom --r1cs --wasm --sym src/light/transaction$1$2Main.circom -o ../../zk.js/build-circuits/ -l node_modules/circomlib/circuits

pnpm snarkjs groth16 setup ../../zk.js/build-circuits/transaction$1$2Main.r1cs ./ptau$POWERS_OF_TAU ../../zk.js/build-circuits/tmp_transaction$1$2Main.zkey
pnpm snarkjs zkey contribute ../../zk.js/build-circuits/tmp_transaction$1$2Main.zkey ../../zk.js/build-circuits/transaction$1$2Main.zkey -e="321432151325321543215"
pnpm snarkjs zkey export verificationkey ../../zk.js/build-circuits/transaction$1$2Main.zkey verification_key_mainnet$2.json

pnpm ts-node ./scripts/createRustVerifyingKey.ts $2 $1$2Main

rm verification_key_mainnet$2.json
rm ../../zk.js/build-circuits/transaction$1$2Main.sym
rm ../../zk.js/build-circuits/transaction$1$2Main.r1cs
rm ../../zk.js/build-circuits/tmp_transaction$1$2Main.zkey
rm ../../zk.js/build-circuits/transaction$1$2Main_js/generate_witness.js
rm ../../zk.js/build-circuits/transaction$1$2Main_js/witness_calculator.js