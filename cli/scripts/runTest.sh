#!/bin/bash

set -eux

LIMIT_LEDGER_SIZE=500000000
NOOP_PROGRAM_ID="noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
MERKLE_TREE_PROGRAM_ID="JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
VERIFIER_PROGRAM_ZERO_ID="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
VERIFIER_PROGRAM_STORAGE_ID="DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"
VERIFIER_PROGRAM_ONE_ID="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
VERIFIER_PROGRAM_TWO_ID="2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
MOCK_VERIFIER_PROGRAM_ID="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"


docker rm -f solana-validator

if [ -f /.dockerenv ]; then
    solana-test-validator \
        --reset \
        --limit-ledger-size=$LIMIT_LEDGER_SIZE \
        --quiet \
        --bpf-program $NOOP_PROGRAM_ID ~/.local/light-protocol/lib/solana-program-library/spl_noop.so \
        --bpf-program $MERKLE_TREE_PROGRAM_ID ./target/deploy/merkle_tree_program.so \
        --bpf-program $VERIFIER_PROGRAM_ZERO_ID ./target/deploy/verifier_program_zero.so \
        --bpf-program $VERIFIER_PROGRAM_STORAGE_ID ./target/deploy/verifier_program_storage.so \
        --bpf-program $VERIFIER_PROGRAM_ONE_ID ./target/deploy/verifier_program_one.so \
        --bpf-program $VERIFIER_PROGRAM_TWO_ID ./target/deploy/verifier_program_two.so \
        --account-dir ../../test-env/accounts \
        --bpf-program $3 /usr/local/lib/test_programs/$4\
        &
    PID=$!

    sleep 7
    $5

    kill $PID
else
    docker rm -f solana-validator || true
    docker run -d \
        --name solana-validator \
        --pull=always \
        -p 8899:8899 \
        -p 8900:8900 \
        -p 8901:8901 \
        -p 8902:8902 \
        -p 9900:9900 \
        -p 8000:8000 \
        -p 8001:8001 \
        -p 8002:8002 \
        -p 8003:8003 \
        -p 8004:8004 \
        -p 8005:8005 \
        -p 8006:8006 \
        -p 8007:8007 \
        -p 8008:8008 \
        -p 8009:8009 \
        -v $HOME/.config/solana/id.json:/home/node/.config/solana/id.json \
        -v $2/target/deploy:/usr/local/lib/test_programs \
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
        --bpf-program $3 /usr/local/lib/test_programs/$4\

    sleep 15
    $5

    docker rm -f solana-validator
fi
