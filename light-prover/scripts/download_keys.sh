#!/usr/bin/env bash

ROOT_DIR="$(git rev-parse --show-toplevel)";
KEYS_DIR="${ROOT_DIR}/light-prover/proving-keys"

if [ ! -d "$KEYS_DIR" ]; then
  mkdir -p "$KEYS_DIR"
fi

# inclusion, non-inclusion and combined keys for 26 elements
BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"

# keys for append circuit 4, 10, 26 elements
APPEND_BUCKET="bafybeiatsnqj4ksuddtzixv5si5cfoy7bah723zigvuuut55vkbvkivig4"

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
  "non-inclusion_26_1.key"
  "non-inclusion_26_1.vkey"
  "non-inclusion_26_2.key"
  "non-inclusion_26_2.vkey"
  "combined_26_1_1.key"
  "combined_26_1_1.vkey"
  "combined_26_1_2.key"
  "combined_26_1_2.vkey"
  "combined_26_2_1.key"
  "combined_26_2_1.vkey"
  "combined_26_2_2.key"
  "combined_26_2_2.vkey"
  "combined_26_3_1.key"
  "combined_26_3_1.vkey"
  "combined_26_3_2.key"
  "combined_26_3_2.vkey"
  "combined_26_4_1.key"
  "combined_26_4_1.vkey"
  "combined_26_4_2.key"
  "combined_26_4_2.vkey"
)

APPEND_FILES=(
  "append_26_1.key"
  "append_26_1.vkey"
  "append_26_10.key"
  "append_26_10.vkey"
  "append_26_100.key"
  "append_26_100.vkey"
  "append_26_500.key"
  "append_26_500.vkey"
  "append_26_1000.key"
  "append_26_1000.vkey"
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


for FILE in "${APPEND_FILES[@]}"
do
  URL="https://${APPEND_BUCKET}.ipfs.w3s.link/${FILE}"
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
