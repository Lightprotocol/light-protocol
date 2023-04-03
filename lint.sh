#!/usr/bin/env bash

set -e

pushd light-system-programs
yarn install
yarn run lint
cargo fmt --all -- --check
popd

pushd light-sdk-ts
yarn install
yarn run lint
popd
