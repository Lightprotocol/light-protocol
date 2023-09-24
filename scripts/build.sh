#!/usr/bin/env sh

source "./scripts/devenv.sh"
set -eux

# Build one project via pnpm:
# pnpm --filter prover.js build

# Build all projects in workspace via pnpm:
# pnpm -r build

# Build one project via nx:
# npx nx build @lightprotocol/zk.js

# Build several project at once via nx:
# npx nx run-many --target=build --projects=@lightprotocol/zk.js,@lightprotocol/cli

# Build projects which cache invalidated:
# npx nx affected:build

# Build all projects via nx:
npx nx run-many --target=build --all