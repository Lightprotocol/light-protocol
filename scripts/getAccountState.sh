#!/usr/bin/env bash

# run this to regenerate the following accounts:
# merkle_tree_pubkey
# nullifier_queue_pubkey
# governance_authority_pda
# group_pda
# 
# to add more accounts to regenerate, add them to setup_test_programs_with_accounts and test script
cd program-tests/system-test && cargo test-sbf regenerate_accounts -- --ignored --nocapture && cd -
