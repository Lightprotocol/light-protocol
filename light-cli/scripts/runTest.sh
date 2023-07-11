#!/usr/bin/env sh

set -eux

LIMIT_LEDGER_SIZE=500000000

NOOP_PROGRAM_ID="noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
MERKLE_TREE_PROGRAM_ID="JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
VERIFIER_PROGRAM_ZERO_ID="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
VERIFIER_PROGRAM_STORAGE_ID="DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"
VERIFIER_PROGRAM_ONE_ID="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"

solana config set --url http://localhost:8899

if [ -f /.dockerenv ]; then
    solana-test-validator \
        --reset \
        --limit-ledger-size=$LIMIT_LEDGER_SIZE \
        --quiet \
        --bpf-program $NOOP_PROGRAM_ID ~/.local/light-protocol/lib/solana-program-library/spl_noop.so \
        --bpf-program $MERKLE_TREE_PROGRAM_ID ../light-system-programs/target/deploy/merkle_tree_program.so \
        --bpf-program $VERIFIER_PROGRAM_ZERO_ID ../light-system-programs/target/deploy/verifier_program_zero.so \
        --bpf-program $VERIFIER_PROGRAM_STORAGE_ID ../light-system-programs/target/deploy/verifier_program_storage.so \
        --bpf-program $VERIFIER_PROGRAM_ONE_ID ../light-system-programs/target/deploy/verifier_program_one.so \
        --account-dir ../../test-env/accounts \
        &
    PID=$!

    sleep 7
else
    docker rm -f solana-validator || true
    docker run -d \
        --name solana-validator \
        --net=host \
        --pull=always \
        -v $HOME/.config/solana/id.json:/home/node/.config/solana/id.json \
        -v $(pwd)/../light-system-programs/target/deploy:/home/node/.local/light-protocol/lib/light-protocol \
        -v $(pwd)/../test-env/accounts:/home/node/.local/light-protocol/lib/accounts \
        ghcr.io/lightprotocol/solana-test-validator:main \
        --reset \
        --limit-ledger-size=$LIMIT_LEDGER_SIZE \
        --quiet \
        --bpf-program $NOOP_PROGRAM_ID /home/node/.local/light-protocol/lib/solana-program-library/spl_noop.so \
        --bpf-program $MERKLE_TREE_PROGRAM_ID /home/node/.local/light-protocol/lib/light-protocol/merkle_tree_program.so \
        --bpf-program $VERIFIER_PROGRAM_ZERO_ID /home/node/.local/light-protocol/lib/light-protocol/verifier_program_zero.so \
        --bpf-program $VERIFIER_PROGRAM_STORAGE_ID /home/node/.local/light-protocol/lib/light-protocol/verifier_program_storage.so \
        --bpf-program $VERIFIER_PROGRAM_ONE_ID /home/node/.local/light-protocol/lib/light-protocol/verifier_program_one.so \
        --bpf-program $VERIFIER_PROGRAM_TWO_ID /home/node/.local/light-protocol/lib/light-protocol/verifier_program_two.so \
        --account-dir /home/node/.local/light-protocol/lib/accounts \
    echo "$1"
    sleep 15

    $1

    docker rm -f solana-validator
fi
