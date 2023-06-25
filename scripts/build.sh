#!/usr/bin/env bash

set -eux

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
