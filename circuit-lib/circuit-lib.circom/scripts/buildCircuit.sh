#!/usr/bin/env sh

set -e

POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
if [ ! -f ./ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o ./ptau$POWERS_OF_TAU
fi
echo " $1$4Transaction$2In$3OutMain"

circom --r1cs --wasm --sym src/transaction/$1$4Transaction$2In$3OutMain.circom -o ../../zk.js/build-circuits/ -l node_modules/circomlib/circuits

pnpm snarkjs groth16 setup ../../zk.js/build-circuits/$1$4Transaction$2In$3OutMain.r1cs ./ptau$POWERS_OF_TAU ../../zk.js/build-circuits/tmp_$1$4Transaction$2In$3OutMain.zkey
pnpm snarkjs zkey contribute ../../zk.js/build-circuits/tmp_$1$4Transaction$2In$3OutMain.zkey ../../zk.js/build-circuits/$1$4Transaction$2In$3OutMain.zkey -e="321432151325321543215"
pnpm snarkjs zkey export verificationkey ../../zk.js/build-circuits/$1$4Transaction$2In$3OutMain.zkey verification_key_mainnet$2.json
echo "Creating verifying key"
echo " 3 1 4  $2$3  $1 $4 "
pnpm ts-node ./scripts/createRustVerifyingKey.ts $2 $3  $1 $4

rm verification_key_mainnet$2.json
rm ../../zk.js/build-circuits/$1$4Transaction$2In$3OutMain.sym
rm ../../zk.js/build-circuits/$1$4Transaction$2In$3OutMain.r1cs
rm ../../zk.js/build-circuits/tmp_$1$4Transaction$2In$3OutMain.zkey
# rm ../../zk.js/build-circuits/$1$2Transaction$3In$4OutMain_js/generate_witness.js
# rm ../../zk.js/build-circuits/$1$2Transaction$3In$4OutMain_js/witness_calculator.js