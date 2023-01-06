#!/bin/bash -e
POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
# mkdir -p build/circuits
if [ ! -f ../light-sdk-ts/build-circuits/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o ../light-sdk-ts/build-circuits/ptau$POWERS_OF_TAU
fi
circom --r1cs --wasm --sym circuits/transactionMasp$1.circom -o ../light-sdk-ts/build-circuits/
#mv "transactionMasp$1.r1cs" ../light-sdk-ts/build-circuits/transactionMasp$1.r1cs;
#mv "transactionMasp$1_js/transactionMasp$1.wasm" ../light-sdk-ts/build-circuits/transactionMasp$1.wasm;
#mv "transactionMasp$1.sym" ../light-sdk-ts/build-circuits/transactionMasp$1.sym;
npx snarkjs groth16 setup ../light-sdk-ts/build-circuits/transactionMasp$1.r1cs ../light-sdk-ts/build-circuits/ptau$POWERS_OF_TAU ../light-sdk-ts/build-circuits/transactionMasp$1.zkey
# echo "qwe" | npx snarkjs zkey contribute build/circuits/tmp_transactionMasp$1.zkey build/circuits/transactionMasp$1.zkey
#npx snarkjs zkey export verificationkey build/circuits/final_transactionMasp2.zkey verification_key_mainnet.json
