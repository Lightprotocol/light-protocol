#!/bin/bash

cd "$(git rev-parse --show-toplevel)"

PDA_FILE="target/idl/psp_compressed_pda.json"
TOKEN_FILE="target/idl/psp_compressed_token.json"
DEST_DIR="js/compressed-token/src/idl"
TS_FILE="$DEST_DIR/psp_compressed_token.ts" # ts output file path
TYPE_NAME="PspCompressedToken" # ts type name

# Check if jq is installed
if ! command -v jq &> /dev/null
then
    echo "jq could not be found. Please install jq to run this script."
    exit 1
fi

# Extract types
PDA_TYPES=$(jq '.types' "$PDA_FILE")
TOKEN_TYPES=$(jq '.types' "$TOKEN_FILE")

# Merge types
MERGED_TYPES=$(jq -s '.[0] + .[1]' <(echo "$PDA_TYPES") <(echo "$TOKEN_TYPES"))

# Generate TS content
MERGED_CONTENT=$(jq --argjson types "$MERGED_TYPES" '.types = $types' "$TOKEN_FILE")

# Generate TypeScript file with JSON object inline
{
  echo -n "export type ${TYPE_NAME} = "
  echo "$MERGED_CONTENT"
  echo ";"
  echo -n "export const IDL: ${TYPE_NAME} = "
  echo "$MERGED_CONTENT"
  echo ";"
} > "$TS_FILE"

echo "IDL for $TYPE_NAME generated at $TS_FILE"


# fmt 
if ! command -v pnpm prettier &> /dev/null
then
    echo "Prettier could not be found. Please install Prettier to run this script."
    exit 1
fi

pnpm prettier --write "$TS_FILE"
echo "Prettier formatting applied to $TS_FILE"

