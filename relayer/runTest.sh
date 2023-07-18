#!/usr/bin/env sh

set -eux

LIMIT_LEDGER_SIZE=500000000

NOOP_PROGRAM_ID="noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
MERKLE_TREE_PROGRAM_ID="JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
VERIFIER_PROGRAM_ZERO_ID="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
VERIFIER_PROGRAM_STORAGE_ID="DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"
VERIFIER_PROGRAM_ONE_ID="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
VERIFIER_PROGRAM_TWO_ID="2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
MOCK_VERIFIER_PROGRAM_ID="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

solana config set --url http://localhost:8899
# kills existing solana processes
killall solana-test-val &
sleep 1
pkill solana-test-validator || true
solana-test-validator \
    --reset \
    --limit-ledger-size="${LIMIT_LEDGER_SIZE}" \
    --quiet \
    --bpf-program "${NOOP_PROGRAM_ID}" ../test-env/programs/spl_noop.so \
    --bpf-program "${MERKLE_TREE_PROGRAM_ID}" ../light-system-programs/target/deploy/merkle_tree_program.so \
    --bpf-program "${VERIFIER_PROGRAM_ZERO_ID}" ../light-system-programs/target/deploy/verifier_program_zero.so \
    --bpf-program "${VERIFIER_PROGRAM_STORAGE_ID}" ../light-system-programs/target/deploy/verifier_program_storage.so \
    --bpf-program "${VERIFIER_PROGRAM_ONE_ID}" ../light-system-programs/target/deploy/verifier_program_one.so \
    --bpf-program "${VERIFIER_PROGRAM_TWO_ID}" ../light-system-programs/target/deploy/verifier_program_two.so \
    --account-dir ../test-env/accounts \
    &
PID="${!}"
# trap "kill ${PID}" EXIT
sleep 7

sleep 8

node lib/index.js
relayer_pid=$!
# trap "kill ${relayer_pid}" EXIT

# sleep 15

# npx ts-mocha -p ./tsconfig.json -t 1000000 tests/functional_test.ts --exit;

# tests/indexer_test.ts
# trap "kill $PID" EXIT
