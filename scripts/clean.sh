#!/usr/bin/env sh

find . -type d -name "test-ledger" -exec sh -c 'echo "Deleting {}"; rm -rf "{}"' \;

npx nx reset