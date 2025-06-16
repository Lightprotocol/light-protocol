#!/bin/bash

# Note generated data hasn't been in photon tests yet.
# expected test results, 50 compressed accounts with 1_000_000 each owned by Pubkey::new_unique() (produces pubkeys deterministicly)
# fully forested
cargo test -p forester -- --test test_state_batched;
cargo xtask export-photon-test-data --test-name batched_tree_transactions;
killall solana-test-validator;

cargo test-sbf -p compressed-token-test -- --test test_transfer_with_photon_and_batched_tree;
cargo xtask export-photon-test-data --test-name batched_tree_token_transactions;
killall solana-test-validator;

cargo test-sbf -p system-cpi-v2-test -- --ignored --test  generate_photon_test_data_multiple_events;
cargo xtask export-photon-test-data --test-name test_multiple_events;
killall solana-test-validator;

#     let num_addresses = 2;
cargo test -p forester -- --test test_create_v2_address;
cargo xtask export-photon-test-data --test-name batched_address_2_transactions;
killall solana-test-validator;
#     let num_addresses = 1;
cargo test -p forester -- --test test_create_v2_address;
cargo xtask export-photon-test-data --test-name batched_address_transactions;
killall solana-test-validator;
