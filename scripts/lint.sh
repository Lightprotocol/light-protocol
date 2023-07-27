#!/usr/bin/env sh

set -e

cd light-system-programs
yarn install
yarn run lint
cargo fmt --all -- --check
cargo clippy --all -- -A clippy::result_large_err -D warnings
cd ..

cd light-zk.js
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
