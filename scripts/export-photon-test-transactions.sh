#!/bin/bash

# Note generated data hasn't been in photon tests yet.
# expected test results, 50 compressed accounts with 1_000_000 each owned by Pubkey::new_unique() (produces pubkeys deterministicly)
# fully forested
cargo test -p forester -- --test test_state_batched
cargo xtask export-photon-test-data
killall solana-test-validator;
