#!/usr/bin/env bash

# run this to regenerate the following accounts:
# merkle_tree_pubkey
# indexed_array_pubkey
# governance_authority_pda
# group_pda
# 
# to add more accounts to regenerate, add them to setup_test_programs_with_accounts and test script
cd programs/compressed-pda && cargo test-sbf regenerate_accounts -- --ignored && cd -