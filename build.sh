#!/usr/bin/env bash

set -e

pushd light-sdk-ts
yarn install
yarn run build
popd

pushd light-system-programs
yarn install
anchor build
popd

pushd light-circuits
yarn install
popd

pushd relayer
yarn install
yarn run build
popd