#!/usr/bin/env bash

cd "$(git rev-parse --show-toplevel)"

PDA_FILE="target/idl/light_system_program.json"
TOKEN_FILE="target/idl/light_compressed_token.json"
DEST_DIR="js/compressed-token/src/idl"
TS_FILE="$DEST_DIR/light_compressed_token.ts" # ts output file path

DEST_DIR_STATELESS="js/stateless.js/src/idls"
TS_FILE_STATELESS="$DEST_DIR_STATELESS/light_compressed_token.ts" # ts output file path

TYPE_NAME="LightCompressedToken" # ts type name

# Check if jq is installed
if ! command -v jq &> /dev/null
then
    echo "jq could not be found. Please install jq to run this script."
    exit 1
fi

# Extract types
PDA_TYPES=$(jq '.types' "$PDA_FILE")
TOKEN_TYPES=$(jq '.types' "$TOKEN_FILE")
# Merge types and deduplicate
MERGED_TYPES=$(jq -s 'add | unique_by(.name)' <(echo "$PDA_TYPES") <(echo "$TOKEN_TYPES"))
# Generate TS content
MERGED_CONTENT=$(jq --argjson types "$MERGED_TYPES" '.types = $types' "$TOKEN_FILE")

# Generate TypeScript file with JSON object inline and place it in both destinations
{
  echo -n "export type ${TYPE_NAME} = "
  echo "$MERGED_CONTENT"
  echo ";"
  echo -n "export const IDL: ${TYPE_NAME} = "
  echo "$MERGED_CONTENT"
  echo ";"
} | tee "$TS_FILE" > "$TS_FILE_STATELESS"

echo "IDL for $TYPE_NAME generated at $TS_FILE and $TS_FILE_STATELESS"

export COREPACK_ENABLE_STRICT=0

# fmt 
if ! command -v pnpm prettier &> /dev/null
then
    echo "Prettier could not be found. Please install Prettier to run this script."
    exit 1
fi
{
  echo "Current directory: $(pwd)"
  pnpm prettier --write "$TS_FILE" "$TS_FILE_STATELESS" && \
  echo "Prettier formatting applied to $TS_FILE and $TS_FILE_STATELESS"
} || {
  echo "Failed to apply Prettier formatting to $TS_FILE and $TS_FILE_STATELESS"
}
