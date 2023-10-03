#!/bin/bash

# --------------------------------------------------------------------------------
# Phase 1
# ... non-circuit-specific stuff

# if artifacts does not exist, make folder
[ -d ./artifacts ] || mkdir ./artifacts

# Create a directory for the ptau file
[ -d ./artifacts/ptau ] || mkdir -p ./artifacts/ptau 


POWERS_OF_TAU=17 # circuit will support max 2^POWERS_OF_TAU constraints

# Check if the same ptau file exists, if not continue
if [ ! -f ./artifacts/ptau/pot$POWERS_OF_TAU.ptau ]; then
  echo "Downloading powers of tau file"
  curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_$POWERS_OF_TAU.ptau --create-dirs -o ./artifacts/ptau/pot$POWERS_OF_TAU.ptau
fi

# --------------------------------------------------------------------------------
# Phase 2
# ... build circuits

# Check if circom exists in node_modules
if [ -d "./node_modules/circom" ]; then
  # If it exists, delete it and its contents -> deleting it removes the bug that prevents running this script
  rm -r "./node_modules/circom"
fi

# Iterate over files with .circom extension in the current directory
for file in ./artifacts/bench_circuits/light/*.circom; do
  # Extract the base name without the extension
  circuit_name=$(basename "$file" .circom)
  # Run the build-circuit script with the circuit name as an argument
  ./scripts/buildTestCircuit.sh $circuit_name
done