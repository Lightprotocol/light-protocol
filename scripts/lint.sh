#!/usr/bin/env bash

set -e

pushd light-system-programs
yarn install
yarn run lint
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
popd

pushd light-zk.js
yarn install
yarn run lint
popd
