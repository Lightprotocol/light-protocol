#!/usr/bin/env sh
set -eux

# copy build-circuits from zkjs into webapp/public
SOURCE="./node_modules/@lightprotocol/zk.js/build-circuits/"
DESTINATION="./public/"

# mkdir -p "$DESTINATION"
cp -LR "$SOURCE" "$DESTINATION"

echo "Copied circuit files to $DESTINATION"

sleep 5

echo "Listing all folders/files in ./public:"
ls -la ./public
ls -la ./public/build-circuits || true # debug purpose