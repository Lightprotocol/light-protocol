#!/usr/bin/env sh
set -e

npx nx run-many --target=test --all --parallel=false

# run relayer docker build script
. $(dirname $0)/testDockerRelayer.sh