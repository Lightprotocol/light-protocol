#!/usr/bin/env sh

set -e

# `dirname "${0}"`/build.sh


cd zk.js
yarn test
sleep 1
cd ..

cd system-programs
yarn test
cd ..


cd cli
yarn test
cd ..

cd relayer
yarn test
cd ..

cd circuit-lib/circuit-lib.circom
yarn run test
cd ../..

cd circuit-lib/circuit-lib.js
yarn run test
cd ../..

# && cd programs/merkle_tree_program && cargo test
