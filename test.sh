#!/usr/bin/env bash

set -e

./build-sdk.sh

pushd light-system-programs
anchor build
yarn test
yarn run test-merkle-tree
yarn run test-user
yarn run test-provider
yarn run test-user-merge-sol
yarn run test-user-merge-spl
yarn run test-user-merge-sol-specific
yarn run test-user-merge-spl-specific
yarn run test-verifiers
popd

pushd light-sdk-ts
yarn test
sleep 1
popd

pushd mock-app-verifier
anchor build
yarn test
yarn run test-verifiers
popd

pushd relayer
yarn test
popd

pushd light-circuits
yarn run test
popd

# && cd programs/merkle_tree_program && cargo test
