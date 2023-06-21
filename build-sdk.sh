#!/usr/bin/env bash

set -e

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_OLD_PATH="${PATH}"
    export PATH="$(git rev-parse --show-toplevel)/.local/bin:$PATH"
fi

pushd light-zk.js
yarn run build
popd

pushd light-circuits
rm -rf node_modules
yarn
popd

pushd light-system-programs
rm -rf node_modules
yarn
popd

pushd mock-app-verifier
rm -rf node_modules
yarn
popd

pushd relayer
rm -rf node_modules
yarn
popd

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    export PATH="${LIGHT_PROTOCOL_OLD_PATH}"
fi
