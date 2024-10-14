#!/usr/bin/env bash

set -e

# Ensure required commands are available
command -v git >/dev/null 2>&1 || { echo >&2 "git is required but it's not installed. Aborting."; exit 1; }
command -v curl >/dev/null 2>&1 || { echo >&2 "curl is required but it's not installed. Aborting."; exit 1; }
command -v wc >/dev/null 2>&1 || { echo >&2 "wc is required but it's not installed. Aborting."; exit 1; }

ROOT_DIR="$(git rev-parse --show-toplevel)"
KEYS_DIR="${ROOT_DIR}/light-prover/proving-keys"

if [ ! -d "$KEYS_DIR" ]; then
  mkdir -p "$KEYS_DIR"
fi

# inclusion, non-inclusion and combined keys for merkle tree of height 26
BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"

# keys for append circuit for trees of height 10, 26 
APPEND_BUCKET="bafybeicfxtft7yfi3sgrhfd7uck5a4m2tifkulh6kbsydergj77y6zuscm"

# keys for update circuit for tree of height 10, 26
UPDATE_BUCKET="bafybeicfxtft7yfi3sgrhfd7uck5a4m2tifkulh6kbsydergj77y6zuscm"

LIGHTWEIGHT_FILES=(
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
  "append_10_10.key"
  "append_10_10.vkey"
  "update_10_10.key"
  "update_10_10.vkey"
)

FULL_FILES=(
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
  "update_26_1.key"
  "update_26_1.vkey"
  "update_26_10.key"
  "update_26_10.vkey"
  "update_26_100.key"
  "update_26_100.vkey"
  "update_26_500.key"
  "update_26_500.vkey"
  "update_26_1000.key"
  "update_26_1000.vkey"
)

download_file() {
  local FILE="$1"
  local BUCKET_URL
  if [[ $FILE == append* ]]; then
    BUCKET_URL="https://${APPEND_BUCKET}.ipfs.w3s.link/${FILE}"
  elif [[ $FILE == update* ]]; then
    BUCKET_URL="https://${UPDATE_BUCKET}.ipfs.w3s.link/${FILE}"
  else
    BUCKET_URL="https://${BUCKET}.ipfs.w3s.link/${FILE}"
  fi
  
  local REMOTE_SIZE=$(curl -sI "$BUCKET_URL" | grep -i Content-Length | awk '{print $2}' | tr -d '\r')
  local LOCAL_SIZE=0
  if [ -f "$KEYS_DIR/$FILE" ]; then
    LOCAL_SIZE=$(wc -c < "$KEYS_DIR/$FILE")
  fi

  if [ "$LOCAL_SIZE" = "$REMOTE_SIZE" ]; then
    echo "$FILE is already downloaded completely. Skipping."
    return 0
  fi

  echo "Downloading $BUCKET_URL"
  local MAX_RETRIES=5
  local attempt=0
  while ! curl -s -o "$KEYS_DIR/$FILE" "$BUCKET_URL" && (( attempt < MAX_RETRIES )); do
    echo "Download failed for $FILE (attempt $((attempt + 1))). Retrying..."
    sleep 2
    ((attempt++))
  done
  if (( attempt == MAX_RETRIES )); then
    echo "Failed to download $FILE after multiple retries."
    return 1
  else
    echo "$FILE downloaded successfully"
  fi
}

download_files() {
  local files=("$@")
  for FILE in "${files[@]}"
  do
    download_file "$FILE" || exit 1
  done
}

if [ "$1" = "light" ]; then
  download_files "${LIGHTWEIGHT_FILES[@]}"
elif [ "$1" = "full" ]; then
  download_files "${FULL_FILES[@]}"
else
  echo "Usage: $0 [light|full]"
  exit 1
fi