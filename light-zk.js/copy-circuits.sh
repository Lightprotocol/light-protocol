#!/bin/bash

SOURCE="./node_modules/@lightprotocol/zk.js/build-circuits/"
DESTINATION="./public/build-circuits/"

mkdir -p "$DESTINATION"
cp -R "$SOURCE" "$DESTINATION"

echo "Copied circuit files to $DESTINATION"
