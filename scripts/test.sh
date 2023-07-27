#!/usr/bin/env sh

set -e

`dirname "${0}"`/build.sh

cd light-system-programs
yarn test
cd ..

cd light-prover.js
yarn test
cd ..

cd light-zk.js
yarn test
sleep 1
cd ..

cd mock-app-verifier
yarn test
cd ..

cd relayer
yarn test
cd ..

cd light-circuits
yarn run test
cd ..

# && cd programs/merkle_tree_program && cargo test
