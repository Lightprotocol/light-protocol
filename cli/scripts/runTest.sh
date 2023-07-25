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

# process id on mac is solana-test-val
killall solana-test-validator || true
# process id on ubuntu is solana-test-val
killall solana-test-val || true
# sleep for the process to be killed
sleep 1
solana-test-validator \
    --reset \
    --limit-ledger-size=$LIMIT_LEDGER_SIZE \
    --quiet \
    --bpf-program $NOOP_PROGRAM_ID ./bin/programs/spl_noop.so \
    --bpf-program $MERKLE_TREE_PROGRAM_ID ./bin/programs/merkle_tree_program.so \
    --bpf-program $VERIFIER_PROGRAM_ZERO_ID ./bin/programs/verifier_program_zero.so \
    --bpf-program $VERIFIER_PROGRAM_STORAGE_ID ./bin/programs/verifier_program_storage.so \
    --bpf-program $VERIFIER_PROGRAM_ONE_ID ./bin/programs/verifier_program_one.so \
    --bpf-program $VERIFIER_PROGRAM_TWO_ID ./bin/programs/verifier_program_two.so \
    --account-dir ./bin/accounts \
    --bpf-program $3 ./target/deploy/$4\
    &
PID=$!

sleep 7
$5

kill $PID