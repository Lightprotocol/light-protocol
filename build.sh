#!/usr/bin/env bash

set -eux

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    LIGHT_PROTOCOL_OLD_PATH="${PATH}"
    export PATH="$(git rev-parse --show-toplevel)/.local/bin:$PATH"
fi

pushd light-zk.js
yarn install --force
yarn run build
popd

pushd light-system-programs
yarn install --force
light-anchor build
popd

pushd mock-app-verifier
yarn install --force
light-anchor build
popd

pushd light-circuits
yarn install --force
popd

pushd relayer
yarn install --force
yarn run build
popd

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    export PATH="${LIGHT_PROTOCOL_OLD_PATH}"
fi
