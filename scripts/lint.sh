#!/usr/bin/env sh

set -e

cd system-programs
yarn install
yarn run lint
cargo fmt --all -- --check
cargo clippy --all -- -A clippy::result_large_err -D warnings
cd ..

cd zk.js
yarn install
yarn run lint
cd ..

cd cli
yarn install
yarn run lint
cd ..

cd relayer
yarn install
yarn run lint
cd ..

cd circuit-lib/circuit-lib.js
yarn install
yarn run lint
cd -
