#!/bin/sh

ROOT_DIR=$(git rev-parse --show-toplevel)
solana-test-validator --account-dir "$ROOT_DIR"/cli/accounts
