#!/usr/bin/env sh

command -v pnpm >/dev/null 2>&1 || { echo >&2 "pnpm is required but it's not installed.  Aborting."; exit 1; }
command -v npx >/dev/null 2>&1 || { echo >&2 "npx is required but it's not installed.  Aborting."; exit 1; }

# source "./scripts/devenv.sh"
. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

pnpm install || { echo >&2 "Failed to install dependencies. Aborting."; exit 1; }
npx nx run-many --target=build --all || { echo >&2 "Build failed. Aborting."; exit 1; }