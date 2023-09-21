#!/usr/bin/env sh

set -e

cd system-programs
yarn install
yarn format
cargo fmt --all
cargo clippy --all -- -A clippy::result_large_err -D warnings
cd ..

cd zk.js
yarn install
yarn format
cd ..

cd cli
yarn install
yarn format
cd ..

cd relayer
yarn install
yarn format
cd ..


cd circuit-lib/circuit-lib.js
yarn install
yarn format
cd -
