#!/usr/bin/env bash

set -e

# Configuration
ROOT_DIR="$(git rev-parse --show-toplevel)"
KEYS_DIR="${ROOT_DIR}/light-prover/proving-keys"
BASE_URL="https://light.fra1.cdn.digitaloceanspaces.com/proving-keys"
CHECKSUM_URL="${BASE_URL}/CHECKSUM"
MAX_RETRIES=3
RETRY_DELAY=5

# Create keys directory
mkdir -p "$KEYS_DIR"

# Download function with retry mechanism
download_file() {
    local url="$1"
    local output="$2"
    local attempt=1

    while [ $attempt -le $MAX_RETRIES ]; do
        echo "Downloading $url (attempt $attempt/$MAX_RETRIES)"
        if curl -L \
                --fail \
                -H "Accept: */*" \
                -H "Accept-Encoding: identity" \
                -A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36" \
                --connect-timeout 30 \
                --max-time 300 \
                --output "$output" \
                "$url"; then
            return 0
        fi
        
        echo "Download failed. Retrying in $RETRY_DELAY seconds..."
        rm -f "$output"  # Remove failed download
        attempt=$((attempt + 1))
        [ $attempt -le $MAX_RETRIES ] && sleep $RETRY_DELAY
    done
    return 1
}

verify_checksum() {
    local file="$1"
    local checksum_file="$2"
    local expected
    local actual
    
    expected=$(grep "${file##*/}" "$checksum_file" | cut -d' ' -f1)
    actual=$(sha256sum "$file" | cut -d' ' -f1)
    
    echo "Expected checksum: $expected"
    echo "Actual checksum:   $actual"
    
    [ "$expected" = "$actual" ]
}

# Download checksum file
CHECKSUM_FILE="${KEYS_DIR}/CHECKSUM"
if ! download_file "$CHECKSUM_URL" "$CHECKSUM_FILE"; then
    echo "Failed to download checksum file"
    exit 1
fi

echo "Content of CHECKSUM file:"
cat "$CHECKSUM_FILE"

case "$1" in
    "light")
        SUFFIXES=(
            "inclusion_32:1 2 3 4 8"
            "mainnet_inclusion_26:1 2 3 4 8"
            "non-inclusion_26:1 2"
            "non-inclusion_40:1 2 3 4 8"
            "combined_26:1_1 1_2 2_1 2_2 3_1 3_2 4_1 4_2"
            "combined_32_40:1_1 1_2 1_3 1_4 2_1 2_2 2_3 2_4 3_1 3_2 3_3 3_4 4_1 4_2 4_3 4_4"
            "append-with-proofs_32:10"
            "update_32:10"
            "address-append_40:10"
        )
        ;;
    "full")
        SUFFIXES=(
            "inclusion_32:1 2 3 4 8"
            "mainnet_inclusion_26:1 2 3 4 8"
            "non-inclusion_26:1 2"
            "non-inclusion_40:1 2 3 4 8"
            "combined_26:1_1 1_2 2_1 2_2 3_1 3_2 4_1 4_2"
            "combined_32_40:1_1 1_2 1_3 1_4 2_1 2_2 2_3 2_4 3_1 3_2 3_3 3_4 4_1 4_2 4_3 4_4"
            "append-with-proofs_32:10 100 500 1000"
            "update_32:10 100 500 1000"
            "address-append_40:10 100 250 500 1000"
        )
        ;;
    *)
        echo "Usage: $0 [light|full]"
        exit 1
        ;;
esac

# Process each file
for group in "${SUFFIXES[@]}"; do
    base=${group%:*}
    suffixes=${group#*:}
    
    for suffix in $suffixes; do
        for ext in key vkey; do
            file="${base}_${suffix}.${ext}"
            output="${KEYS_DIR}/${file}"
            
            if [ -f "$output" ] && verify_checksum "$output" "$CHECKSUM_FILE"; then
                echo "Skipping $file (already downloaded and verified)"
                continue
            fi
            
            if download_file "${BASE_URL}/${file}" "$output"; then
                echo "Verifying checksum for $file..."
                if ! verify_checksum "$output" "$CHECKSUM_FILE"; then
                    echo "Checksum verification failed for $file"
                    rm -f "$output"
                    exit 1
                fi
                echo "Successfully downloaded and verified $file"
            else
                echo "Failed to download $file"
                exit 1
            fi
        done
    done
done

echo "All files downloaded and verified successfully"