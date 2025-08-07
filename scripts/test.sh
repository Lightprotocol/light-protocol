#!/usr/bin/env bash

if [ -z "${LIGHT_PROTOCOL_DEVENV:-}" ]; then
    . "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }
fi

set -eux

npx nx run-many --target=test --all --parallel=false