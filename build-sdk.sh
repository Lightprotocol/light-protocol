#!/usr/bin/env bash

set -e

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
