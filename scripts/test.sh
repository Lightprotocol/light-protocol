#!/usr/bin/env sh
set -e

#npx nx test @lightprotocol/prover.js
#npx nx test @lightprotocol/zk.js
#npx nx test @lightprotocol/circuit-lib.circom
#npx nx test @lightprotocol/circuit-lib.js
#npx nx test @lightprotocol/system-programs
#npx nx test @lightprotocol/cli
#npx nx test @lightprotocol/relayer

# Test one project via nx:
# npx nx test @lightprotocol/zk.js

# Test projects which cache invalidated:
# npx nx affected:test

# We can't run tests in parallel because of static solana-test-validator port binding.
# Test all projects via nx:
 npx nx run-many --target=test --all --parallel=false