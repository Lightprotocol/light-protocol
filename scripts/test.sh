#!/usr/bin/env bash

set -e

$(dirname "${BASH_SOURCE[0]}")/build.sh

pushd light-system-programs
yarn test
popd

pushd light-prover.js
yarn test
sleep 1
popd

pushd light-zk.js
yarn test
sleep 1
popd

pushd mock-app-verifier
yarn test
popd

pushd relayer
yarn test
popd

pushd light-circuits
yarn run test
popd

# && cd programs/merkle_tree_program && cargo test
