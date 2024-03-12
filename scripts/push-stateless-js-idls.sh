#!/bin/bash
cd "$(git rev-parse --show-toplevel)"

SOURCE_DIR="./target/types"
DEST_DIR="./js"
DEST_DIR_STATELESS="$DEST_DIR/stateless.js/src/idls"

FILES_TO_COPY=("account_compression.ts" "light.ts" "light_merkle_tree_program.ts" "psp_compressed_pda.ts" "user_registry.ts")

# copy each type file into the respective location
for FILE in "${FILES_TO_COPY[@]}"; do
  if [ ! -f "$SOURCE_DIR/$FILE" ]; then
    echo "Error: $FILE not found."
    exit 1
  else
    cp "$SOURCE_DIR/$FILE" $DEST_DIR_STATELESS
  fi
done

echo "IDL type files pushed to directories."


# fmt 
if ! command -v pnpm prettier &> /dev/null
then
    echo "Prettier could not be found. Please install Prettier to run this script."
    exit 1
fi
for FILE in "${FILES_TO_COPY[@]}"; do
  pnpm prettier --write "$DEST_DIR_STATELESS/$FILE"
done

echo "IDL type files formatted succesfully."
