#!/usr/bin/env sh
set -e

# run relayer docker build script
source ./scripts/testDocker.sh

EXIT 1

# npx nx run-many --target=test --all --parallel=false