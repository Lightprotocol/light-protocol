#!/usr/bin/env sh

find . -type d \( -name "test-ledger" \) -exec rm -rf {} +

npx nx reset