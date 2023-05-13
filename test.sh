#!/usr/bin/env bash

set -e

./build-sdk.sh

pushd light-system-programs
anchor build
yarn test
popd

pushd light-sdk-ts
yarn test
sleep 1
popd

pushd mock-app-verifier
anchor build
yarn test
popd

pushd relayer
yarn test
popd

pushd light-circuits
yarn run test
popd

# && cd programs/merkle_tree_program && cargo test
