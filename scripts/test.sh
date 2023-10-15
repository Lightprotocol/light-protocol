#!/usr/bin/env sh
set -e

# run relayer docker build script
source $(dirname $0)/testDockerRelayer.sh

npx nx run-many --target=test --all --parallel=false