#!/usr/bin/env sh

source "./scripts/devenv.sh"
set -eux

npx nx run-many --target=build --all