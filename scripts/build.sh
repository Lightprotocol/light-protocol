#!/usr/bin/env sh

command -v pnpm >/dev/null 2>&1 || { echo >&2 "pnpm is not installed.  Aborting."; exit 1; }
command -v npx >/dev/null 2>&1 || { echo >&2 "npx is not installed.  Aborting."; exit 1; }

. "./scripts/devenv.sh" || { echo >&2 "Failed to source devenv.sh. Aborting."; exit 1; }

set -eux

pnpm install || { echo >&2 "Failed to install dependencies. Aborting."; exit 1; }

npx nx run-many --target=build --all \
  --exclude web-wallet \
  --exclude @lightprotocol/cli \
  --exclude @lightprotocol/stateless.js \
  --exclude @lightprotocol/compressed-token

curl -L -o \
  ./target/deploy/spl_noop.so \
  https://github.com/Lightprotocol/light-protocol/releases/download/spl-noop-v0.2.0/spl_noop.so

# Distribute IDL files to client libraries
./scripts/push-stateless-js-idls.sh
./scripts/push-compressed-token-idl.sh

# Enforce build order of dependent projects
npx nx run @lightprotocol/stateless.js:build
npx nx run @lightprotocol/compressed-token:build
npx nx run @lightprotocol/cli:build


echo "Build process completed successfully."

