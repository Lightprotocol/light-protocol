#!/usr/bin/env sh
set -e

export LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS=true

npx nx build @lightprotocol/programs --skip-nx-cache
$(dirname $0)/test.sh
