#!/usr/bin/env bash

set -e

./build-sdk.sh

# pushd light-system-programs
# anchor build
# yarn test
# yarn run test-merkle-tree
# yarn run test-verifiers
# yarn run test-user
# yarn run test-provider
# popd

pushd light-sdk-ts
yarn test
sleep 1
popd

# pushd mock-app-verifier
# anchor build
# yarn test
# yarn run test-verifiers
# popd

pushd light-circuits
yarn run test
popd

# && cd programs/merkle_tree_program && cargo test
