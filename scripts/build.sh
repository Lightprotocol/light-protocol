#!/usr/bin/env bash

command -v pnpm >/dev/null 2>&1 || { echo >&2 "pnpm is not installed.  Aborting."; exit 1; }
command -v npx >/dev/null 2>&1 || { echo >&2 "npx is not installed.  Aborting."; exit 1; }

set -eux

pnpm install || { echo >&2 "Failed to install dependencies. Aborting."; exit 1; }

if [ ! -f target/deploy/spl_noop.so ]; then
    mkdir -p target/deploy && cp third-party/solana-program-library/spl_noop.so target/deploy
fi

npx nx run-many --target=build --all

echo "Build process completed successfully."
