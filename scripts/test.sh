#!/usr/bin/env sh

. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

npx nx run-many --target=test --all --parallel=false

# run relayer docker build script
. $(dirname $0)/testDockerRelayer.sh