#!/usr/bin/env sh
set -eux

echo "ensure that all node_modules are installed before running copy-circuits!"

SOURCE="./node_modules/@lightprotocol/zk.js/build-circuits"
DESTINATION="./public/"
cp -LR "$SOURCE" "$DESTINATION"
sync "$DESTINATION"

echo "Copied circuit files to $DESTINATION"
