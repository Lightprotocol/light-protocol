#!/bin/bash -e
POWERS_OF_TAU=15 # circuit will support max 2^POWERS_OF_TAU constraints
mkdir -p build/circuits
if [ ! -f build/circuits/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o build/circuits/ptau$POWERS_OF_TAU
fi
circom --r1cs --wasm --sym Light_circuits/circuits/transactionMasp$1.circom -o Light_circuits/build/circuits/
#mv "transactionMasp$1.r1cs" Light_circuits/build/circuits/transactionMasp$1.r1cs;
#mv "transactionMasp$1_js/transactionMasp$1.wasm" Light_circuits/build/circuits/transactionMasp$1.wasm;
#mv "transactionMasp$1.sym" Light_circuits/build/circuits/transactionMasp$1.sym;
npx snarkjs groth16 setup Light_circuits/build/circuits/transactionMasp$1.r1cs Light_circuits/build/circuits/ptau$POWERS_OF_TAU Light_circuits/build/circuits/transactionMasp$1.zkey
# echo "qwe" | npx snarkjs zkey contribute build/circuits/tmp_transactionMasp$1.zkey build/circuits/transactionMasp$1.zkey
#npx snarkjs zkey export verificationkey build/circuits/final_transactionMasp2.zkey verification_key_mainnet.json
