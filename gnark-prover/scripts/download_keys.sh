#!/usr/bin/env bash

ROOT_DIR="$(git rev-parse --show-toplevel)";
KEYS_DIR="${ROOT_DIR}/gnark-prover/circuits"

# if KEY_DIR does not exist, create it
if [ ! -d "$KEYS_DIR" ]; then
  mkdir -p "$KEYS_DIR"
fi

BUCKET="bafybeidml266k4d62vu5gpvvv3qejwokuok5oveabjtzomdrm7oxu5z7su"
FILES=(
  "inclusion_26_1.key"
  "inclusion_26_1.vkey"
  "inclusion_26_2.key"
  "inclusion_26_2.vkey"
  "inclusion_26_3.key"
  "inclusion_26_3.vkey"
  "inclusion_26_4.key"
  "inclusion_26_4.vkey"
  "inclusion_26_8.key"
  "inclusion_26_8.vkey"
)

for FILE in "${FILES[@]}"
do
  URL="https://${BUCKET}.ipfs.w3s.link/${FILE}"
  echo "Downloading" "$URL"
  MAX_RETRIES=5
  attempt=0
  while ! curl -s -o "$KEYS_DIR/$FILE" "$URL" && (( attempt < MAX_RETRIES )); do
    echo "Download failed for $FILE (attempt $((attempt + 1))). Retrying..."
    sleep 2
    ((attempt++))
  done
  if (( attempt == MAX_RETRIES )); then
    echo "Failed to download $FILE after multiple retries."
    exit 1
  else
    echo "$FILE downloaded successfully"
  fi
done