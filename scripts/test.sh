#!/usr/bin/env bash

set -e

$(dirname "${BASH_SOURCE[0]}")/build.sh

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_OLD_PATH="${PATH}"
    export PATH="$(git rev-parse --show-toplevel)/.local/bin:$PATH"
fi

pushd light-system-programs
yarn test
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

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    export PATH="${LIGHT_PROTOCOL_OLD_PATH}"
fi
