#!/usr/bin/env sh
set -e

export LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS=false

npx nx build @lightprotocol/programs --skip-nx-cache
npx nx build @lightprotocol/cli --skip-nx-cache
npx nx run --project @lightprotocol/zk.js test-sp-merkle-tree-legacy
$(dirname $0)/test.sh
