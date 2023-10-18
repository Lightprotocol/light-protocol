#!/usr/bin/env sh
set -e

npx nx affected --target=test --parallel=false

# run relayer docker build script
. $(dirname $0)/testDockerRelayer.sh