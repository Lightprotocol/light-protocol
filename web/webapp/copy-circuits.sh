#!/usr/bin/env sh
set -eux

echo "ensure that all node_modules are installed before running copy-circuits!"

# copy build-circuits from zkjs into webapp/public
SOURCE="./node_modules/@lightprotocol/zk.js/build-circuits/"
DESTINATION="./public/"

cp -LR "$SOURCE" "$DESTINATION"

echo "Copied circuit files to $DESTINATION"

sleep 5
