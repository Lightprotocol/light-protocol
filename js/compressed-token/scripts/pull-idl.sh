#!/bin/bash
cd "$(git rev-parse --show-toplevel)"

SOURCE_DIR="./target/types"
DEST_DIR="./js"

# stateless.js
DEST_DIR_STATELESS="$DEST_DIR/stateless.js/src/idls"

FILES_TO_COPY=("psp_compressed_token.ts")

# copy each type file into the respective location
for FILE in "${FILES_TO_COPY[@]}"; do
  if [ ! -f "$SOURCE_DIR/$FILE" ]; then
    echo "Error: $FILE not found."
    exit 1
  else
    cp "$SOURCE_DIR/$FILE" $DEST_DIR_STATELESS
  fi
done

echo "IDL type files pulled from /target to compressed-token successfully."
