#!/bin/bash -e
POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints
mkdir -p build
if [ ! -f build/ptau$POWERS_OF_TAU ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o build/ptau$POWERS_OF_TAU
fi
circom --r1cs --wasm ./circuit/appTransaction.circom -o ./build/
# npx snarkjs groth16 setup ./sdk/build-circuit/appTransaction.r1cs build/ptau$POWERS_OF_TAU ./sdk/build-circuit/appTransaction.zkey
# # echo "qwe" | npx snarkjs zkey contribute build/circuits/tmp_appTransaction.zkey build/circuits/appTransaction.zkey -e "e12312fsdfadsadf"
# npx snarkjs zkey export verificationkey ./sdk/build-circuit/appTransaction.zkey verifyingkey.json
# node ./scripts/parse_verifyingkey.js app verifier # TODO: needs to be assigned at template creation
# rm verifyingkey.json
# rm ./sdk/build-circuit/appTransaction.r1cs
# rm ./sdk/build-circuit/appTransaction.zkey
