#!/bin/bash -e
POWERS_OF_TAU=18 # circuit will support max 2^POWERS_OF_TAU constraints
mkdir -p build/circuits
if [ ! -f build/circuits/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o build/circuits/ptau$POWERS_OF_TAU
fi
npx circom -v -r build/circuits/transaction$1.r1cs -w build/circuits/transaction$1.wasm -s ../artifacts/new_circuits/transaction$1.sym Light_circuits/circuits/transaction$1.circom
npx snarkjs groth16 setup build/circuits/transaction$1.r1cs build/circuits/ptau$POWERS_OF_TAU ../artifacts/new_circuits/tmp_transaction$1.zkey
# echo "qwe" | npx snarkjs zkey contribute build/circuits/tmp_transaction$1.zkey build/circuits/transaction$1.zkey
#npx snarkjs zkey export verificationkey build/circuits/final_transaction2.zkey verification_key_mainnet.json
