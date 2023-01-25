#!/bin/bash -e
../../solana/validator/solana-test-validator     --reset     --limit-ledger-size 500000000     --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i ./target/deploy/verifier_program_zero.so     --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 ./target/deploy/merkle_tree_program.so     --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL ./target/deploy/verifier_program_one.so --quiet &
sleep 5
PID=$!
$1;
kill $PID;
