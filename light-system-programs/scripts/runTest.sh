#!/bin/bash

set -eux

LIMIT_LEDGER_SIZE=500000000

NOOP_PROGRAM_ID="noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
MERKLE_TREE_PROGRAM_ID="JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
VERIFIER_PROGRAM_ZERO_ID="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
VERIFIER_PROGRAM_STORAGE_ID="DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"
VERIFIER_PROGRAM_ONE_ID="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"

solana config set --url http://localhost:8899

pkill solana-test-validator || true
solana-test-validator \
    --reset \
    --limit-ledger-size=$LIMIT_LEDGER_SIZE \
    --quiet \
    --bpf-program $NOOP_PROGRAM_ID ../test-env/programs/spl_noop.so \
    --bpf-program $MERKLE_TREE_PROGRAM_ID ./target/deploy/merkle_tree_program.so \
    --bpf-program $VERIFIER_PROGRAM_ZERO_ID ./target/deploy/verifier_program_zero.so \
    --bpf-program $VERIFIER_PROGRAM_STORAGE_ID ./target/deploy/verifier_program_storage.so \
    --bpf-program $VERIFIER_PROGRAM_ONE_ID ./target/deploy/verifier_program_one.so \
    --account-dir ../test-env/accounts \
    &
PID=$!
trap "kill $PID" EXIT
sleep 7
$1
