#!/bin/bash
echo $2
set -eux
docker rm -f solana-validator || true
docker run -d \
    --name solana-validator \
    --net=host \
    -v $HOME/.config/solana/id.json:/home/node/.config/solana/id.json \
    -v $1/../accounts:/usr/local/lib/accounts \
    -v $2/target/deploy:/usr/local/lib/test_programs \
    ghcr.io/lightprotocol/solana-test-validator:pr-3 \
    --reset \
    --limit-ledger-size 500000000 \
    --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i /home/node/.local/light-protocol/lib/light-protocol/verifier_program_zero.so \
    --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 /home/node/.local/light-protocol/lib/light-protocol/merkle_tree_program.so \
    --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL /home/node/.local/light-protocol/lib/light-protocol/verifier_program_one.so \
    --bpf-program noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV /home/node/.local/light-protocol/lib/solana-program-library/spl_noop.so \
    --bpf-program DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj /home/node/.local/light-protocol/lib/light-protocol/verifier_program_storage.so \
    --bpf-program GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8 /home/node/.local/light-protocol/lib/light-protocol/verifier_program_two.so  \
    --account-dir /usr/local/lib/accounts \
    --bpf-program $3 /usr/local/lib/test_programs/$4\
    --quiet

sleep 5
# echo $PWD
$5

# docker rm -f solana-validator