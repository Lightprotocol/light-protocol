#!/bin/sh
set -e

# make sure this is consistent with the size specified in the buildAllBecnhCircuits script
POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints

# Compile circuit => $1 will refer to the full circuit name
CIRCUIT_NAME="${1%%_*}"

# Check if wasm and zkey files exist, if not continue
if [ -f "./artifacts/$1/$CIRCUIT_NAME.wasm" ] && [ -f "./artifacts/$1/$CIRCUIT_NAME.zkey" ]; then
    # echo "Files exist. Exiting."
    exit 0
fi

# Create a directory for the ptau file
[ -d ./artifacts/$1 ] || mkdir -p ./artifacts/$1 
circom ./artifacts/bench_circuits/light/$1.circom -o ./artifacts/$1 --r1cs --wasm -l node_modules/circomlib/circuits

# Setup
pnpm snarkjs groth16 setup ./artifacts/$1/$1.r1cs ./artifacts/ptau/pot$POWERS_OF_TAU.ptau ./artifacts/$1/$CIRCUIT_NAME.zkey

# # Generate reference zkey
pnpm snarkjs zkey new ./artifacts/$1/$1.r1cs ./artifacts/ptau/pot$POWERS_OF_TAU.ptau ./artifacts/$1/$1_0000.zkey

# # Ceremony just like before but for zkey this time
pnpm snarkjs zkey contribute ./artifacts/$1/$1_0000.zkey ./artifacts/$1/$1_0001.zkey \
    --name="First $1 contribution" -v -e="$(head -n 4096 /dev/urandom | openssl sha1)"

# # Export verification key
pnpm snarkjs zkey export verificationkey ./artifacts/$1/$CIRCUIT_NAME.zkey ./artifacts/$1/$CIRCUIT_NAME.vkey.json

rm ./artifacts/$1/$1_000*.zkey
mv ./artifacts/$1/$1_js/$1.wasm ./artifacts/$1
rm -rf ./artifacts/$1/$1_js