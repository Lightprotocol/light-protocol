#!/usr/bin/env sh

source "./scripts/devenv.sh"
set -eux

pnpm install
npx nx run-many --target=build --all