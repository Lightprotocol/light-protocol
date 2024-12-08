#!/usr/bin/env bash

set -e

command -v git >/dev/null 2>&1 || { echo >&2 "git is required but it's not installed. Aborting."; exit 1; }
command -v curl >/dev/null 2>&1 || { echo >&2 "curl is required but it's not installed. Aborting."; exit 1; }
command -v wc >/dev/null 2>&1 || { echo >&2 "wc is required but it's not installed. Aborting."; exit 1; }

ROOT_DIR="$(git rev-parse --show-toplevel)"
KEYS_DIR="${ROOT_DIR}/light-prover/proving-keys"
TEMP_DIR="${KEYS_DIR}/temp"

mkdir -p "$KEYS_DIR" "$TEMP_DIR"

download_file() {
  local FILE="$1"
  local BUCKET_URL
  if [[ $FILE == append-with-proofs* ]]; then
    BUCKET_URL="https://${APPEND_WITH_PROOFS_BUCKET}.ipfs.w3s.link/${FILE}"
  elif [[ $FILE == append-with-subtrees* ]]; then
    BUCKET_URL="https://${APPEND_WITH_SUBTREES_BUCKET}.ipfs.w3s.link/${FILE}"
  elif [[ $FILE == update* ]]; then
    BUCKET_URL="https://${UPDATE_BUCKET}.ipfs.w3s.link/${FILE}"
  elif [[ $FILE == address-append* ]]; then
    BUCKET_URL="https://${APPEND_ADDRESS_BUCKET}.ipfs.w3s.link/${FILE}"
  else
    BUCKET_URL="https://${BUCKET}.ipfs.w3s.link/${FILE}"
  fi

  local TEMP_FILE="${TEMP_DIR}/${FILE}.partial"
  local FINAL_FILE="${KEYS_DIR}/${FILE}"

  # Get remote file size with more robust handling
  local REMOTE_SIZE
  REMOTE_SIZE=$(curl -sI "$BUCKET_URL" | grep -i '^content-length:' | awk '{print $2}' | tr -d '\r\n[:space:]')
  if [[ ! "$REMOTE_SIZE" =~ ^[0-9]+$ ]]; then
    echo "Warning: Could not determine remote file size for $FILE"
    REMOTE_SIZE=0
  fi

  # Check if final file exists and has correct size
  if [ -f "$FINAL_FILE" ] && [ "$REMOTE_SIZE" -ne 0 ]; then
    local FINAL_SIZE
    FINAL_SIZE=$(wc -c < "$FINAL_FILE" | tr -d '[:space:]')
    if [ "$FINAL_SIZE" = "$REMOTE_SIZE" ]; then
      echo "$FILE is already downloaded completely. Skipping."
      return 0
    fi
  fi

  # Check if partial download exists
  local RESUME_FLAG=""
  if [ -f "$TEMP_FILE" ]; then
    local PARTIAL_SIZE
    PARTIAL_SIZE=$(wc -c < "$TEMP_FILE" | tr -d '[:space:]')
    if [ "$REMOTE_SIZE" -ne 0 ] && [ "$PARTIAL_SIZE" -lt "$REMOTE_SIZE" ]; then
      RESUME_FLAG="-C -"
      echo "Resuming download of $FILE from byte $PARTIAL_SIZE"
    else
      rm -f "$TEMP_FILE"  # Remove potentially corrupted partial file
    fi
  fi

  echo "Downloading $FILE"
  [ "$REMOTE_SIZE" -ne 0 ] && echo "Expected size: $REMOTE_SIZE bytes"

  local MAX_RETRIES=100
  local attempt=0
  while (( attempt < MAX_RETRIES )); do
    if curl -S -f --retry 3 --retry-delay 2 --connect-timeout 30 \
         --max-time 3600 $RESUME_FLAG \
         -o "$TEMP_FILE" "$BUCKET_URL"; then

      # Verify downloaded file size only if we know the remote size
      if [ "$REMOTE_SIZE" -ne 0 ]; then
        local DOWNLOADED_SIZE
        DOWNLOADED_SIZE=$(wc -c < "$TEMP_FILE" | tr -d '[:space:]')
        if [ "$DOWNLOADED_SIZE" = "$REMOTE_SIZE" ]; then
          mv "$TEMP_FILE" "$FINAL_FILE"
          echo "$FILE downloaded and verified successfully"
          return 0
        else
          echo "Size mismatch for $FILE (expected: $REMOTE_SIZE, got: $DOWNLOADED_SIZE)"
          rm -f "$TEMP_FILE"  # Remove corrupted file
        fi
      else
        # If we don't know the remote size, just move the file if download completed
        mv "$TEMP_FILE" "$FINAL_FILE"
        echo "$FILE downloaded successfully (size: $(wc -c < "$FINAL_FILE" | tr -d '[:space:]') bytes)"
        return 0
      fi
    fi

    echo "Download failed for $FILE (attempt $((attempt + 1))). Retrying..."
    sleep $((2 ** attempt))
    ((attempt++))
  done

  echo "Failed to download $FILE after $MAX_RETRIES attempts"
  return 1
}

cleanup() {
  echo "Cleaning up temporary files..."
  rm -rf "$TEMP_DIR"
  exit 1
}

# Set up trap for script interruption
trap cleanup INT TERM

download_files() {
  local files=("$@")
  local failed_files=()

  for FILE in "${files[@]}"; do
    if ! download_file "$FILE"; then
      failed_files+=("$FILE")
      echo "Failed to download: $FILE"
    fi
  done

  if [ ${#failed_files[@]} -ne 0 ]; then
    echo "The following files failed to download:"
    printf '%s\n' "${failed_files[@]}"
    exit 1
  fi
}

# inclusion, non-inclusion and combined keys for merkle tree of height 26

BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"

# mt height 26, batch sizes {1, 10, 100, 500, 1000}
APPEND_WITH_PROOFS_BUCKET="bafybeicngrfui5cef2a4g67lxw3u42atyrfks35vx4hu6c4rme3knh6lby"
APPEND_WITH_SUBTREES_BUCKET="bafybeieyujtdrhp52unqkwvzn36o4hh4brsw52juaftceaki4gfypszbxa"

# mt height 40, batch sizes {1, 10, 100, 250, 500, 1000}
APPEND_ADDRESS_BUCKET="bafybeib2rajatndlpslpqhf4vrbekpyyehjt5byivfzxl36c5p67ypddvu"

# keys for update circuit for tree of height 26
UPDATE_BUCKET="bafybeievf2qdaex4cskdfk24uifq4244ne42w3dghwnnfp4ybsve6mw2pa"

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
  "append-with-proofs_26_10.key"
  "append-with-proofs_26_10.vkey"
  "append-with-subtrees_26_10.key"
  "append-with-subtrees_26_10.vkey"
  "update_26_10.key"
  "update_26_10.vkey"
  "address-append_40_1.key"
  "address-append_40_1.vkey"
  "address-append_40_10.key"
  "address-append_40_10.vkey"
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
  "append-with-proofs_26_1.key"
  "append-with-proofs_26_1.vkey"
  "append-with-proofs_26_10.key"
  "append-with-proofs_26_10.vkey"
  "append-with-proofs_26_100.key"
  "append-with-proofs_26_100.vkey"
  "append-with-proofs_26_500.key"
  "append-with-proofs_26_500.vkey"
  "append-with-proofs_26_1000.key"
  "append-with-proofs_26_1000.vkey"
  "append-with-subtrees_26_1.key"
  "append-with-subtrees_26_1.vkey"
  "append-with-subtrees_26_10.key"
  "append-with-subtrees_26_10.vkey"
  "append-with-subtrees_26_100.key"
  "append-with-subtrees_26_100.vkey"
  "append-with-subtrees_26_500.key"
  "append-with-subtrees_26_500.vkey"
  "append-with-subtrees_26_1000.key"
  "append-with-subtrees_26_1000.vkey"
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
  "address-append_40_1.key"
  "address-append_40_1.vkey"
  "address-append_40_10.key"
  "address-append_40_10.vkey"
  "address-append_40_100.key"
  "address-append_40_100.vkey"
  "address-append_40_250.key"
  "address-append_40_250.vkey"
  "address-append_40_500.key"
  "address-append_40_500.vkey"
  "address-append_40_1000.key"
  "address-append_40_1000.vkey"
)

if [ "$1" = "light" ]; then
  download_files "${LIGHTWEIGHT_FILES[@]}"
elif [ "$1" = "full" ]; then
  download_files "${FULL_FILES[@]}"
else
  echo "Usage: $0 [light|full]"
  exit 1
fi


rm -rf "$TEMP_DIR"