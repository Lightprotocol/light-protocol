#!/usr/bin/env sh

set -e

`dirname "${0}"`/build.sh


cd light-zk.js
yarn test
sleep 1
cd ..

cd light-system-programs
yarn test
cd ..


cd cli
yarn test
cd ..

cd relayer
yarn test
cd ..

cd light-circuits
yarn run test
cd ..

pushd light-cli
yarn test-cli
popd
# && cd programs/merkle_tree_program && cargo test
