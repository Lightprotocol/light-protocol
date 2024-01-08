#!/usr/bin/env sh

. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

npx nx run-many --target=test --all --parallel=false

# run rpc docker build script
. $(dirname $0)/testDockerRpc.sh