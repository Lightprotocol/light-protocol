#!/bin/bash -e
set -eux

docker rm -f solana-validator || true
docker run -d \
    --name solana-validator \
    --net=host \
    -v $HOME/.config/solana/id.json:/root/.config/solana/id.json \
    -v $(pwd)/target/deploy:/usr/local/lib/light-protocol-onchain \
    vadorovsky/solana:audit \
    --reset \
    --limit-ledger-size 500000000 \
    --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i /usr/local/lib/light-protocol-onchain/verifier_program_zero.so \
    --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 /usr/local/lib/light-protocol-onchain/merkle_tree_program.so \
    --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL /usr/local/lib/light-protocol-onchain/verifier_program_one.so \
    --bpf-program noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV /usr/local/lib/solana-program-library/spl_noop.so \
    --bpf-program DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj /usr/local/lib/light-protocol-onchain/verifier_program_storage.so \
    --quiet

sleep 15

# airdrops 
solana airdrop 100000 ZBUKxVWviAJBy12edp5H6kvhcatGYW3BV4ijbgxpVSq && solana airdrop 100000 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k && solana airdrop 100000 8Ers2bBEWExdrh7KDFTrRbauPbFeEvsHz3UX4vxcK9xY && solana airdrop 10000 BEKmoiPHRUxUPik2WQuKqkoFLLkieyNPrTDup5h8c9S7

# running a relayer

pushd ../relayer

node lib/index.js &
relayer_pid=$!

sleep 20

popd

yarn test-browser-wallet

kill $relayer_pid

$1
docker rm -f solana-validator

