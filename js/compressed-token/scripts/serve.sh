#!/bin/bash

# Start gnark-prover in the background
../../circuit-lib/circuit-lib.js/scripts/prover.sh &
PROVER_PID=$!

# Start test-validator in the background
../../cli/test_bin/run test-validator -b &
VALIDATOR_PID=$!

# Wait a bit for servers to start up (adjust sleep as necessary)
sleep 5

# Run your tests here (replace with your actual test command)
pnpm vitest run tests/e2e/*.test.ts
TEST_STATUS=$?

# Kill the background processes
kill $PROVER_PID
kill $VALIDATOR_PID

# Exit with the test command's exit status
exit $TEST_STATUS