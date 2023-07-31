#!/usr/bin/env sh

set -e

POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
if [ ! -f ./ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o ./ptau$POWERS_OF_TAU
fi

circom --r1cs --wasm --sym circuits/transaction$1$2Main.circom -o ../light-zk.js/build-circuits/

yarn snarkjs groth16 setup ../light-zk.js/build-circuits/transaction$1$2Main.r1cs ./ptau$POWERS_OF_TAU ../light-zk.js/build-circuits/tmp_transaction$1$2Main.zkey
yarn snarkjs zkey contribute ../light-zk.js/build-circuits/tmp_transaction$1$2Main.zkey ../light-zk.js/build-circuits/transaction$1$2Main.zkey -e="321432151325321543215"
yarn snarkjs zkey verify ../light-zk.js/build-circuits/transaction$1$2Main.r1cs ptau$POWERS_OF_TAU ../light-zk.js/build-circuits/transaction$1$2Main.zkey
yarn snarkjs zkey export verificationkey ../light-zk.js/build-circuits/transaction$1$2Main.zkey verification_key_mainnet$2.json

yarn ts-node createRustVerifyingKey.ts $2 $1$2Main

rm verification_key_mainnet$2.json
rm ../light-zk.js/build-circuits/transaction$1$2Main.sym
rm ../light-zk.js/build-circuits/transaction$1$2Main.r1cs
rm ../light-zk.js/build-circuits/tmp_transaction$1$2Main.zkey
rm ../light-zk.js/build-circuits/transaction$1$2Main_js/generate_witness.js
rm ../light-zk.js/build-circuits/transaction$1$2Main_js/witness_calculator.js