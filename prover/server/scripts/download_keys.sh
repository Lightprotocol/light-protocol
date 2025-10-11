#!/usr/bin/env bash

set -e

# Configuration with environment variable support
ROOT_DIR="$(git rev-parse --show-toplevel)"
KEYS_DIR="${ROOT_DIR}/prover/server/proving-keys"
BASE_URL="https://storage.googleapis.com/light-protocol-proving-keys/proving-keys-06-03-25"
CHECKSUM_URL="${BASE_URL}/CHECKSUM"

# Configurable parameters for poor connections
MAX_RETRIES=${DOWNLOAD_MAX_RETRIES:-10}  # Default 10, can be overridden
INITIAL_RETRY_DELAY=${DOWNLOAD_RETRY_DELAY:-5}  # Initial delay in seconds
MAX_RETRY_DELAY=${DOWNLOAD_MAX_RETRY_DELAY:-300}  # Max delay (5 minutes)
BANDWIDTH_LIMIT=${DOWNLOAD_BANDWIDTH_LIMIT:-}  # Optional bandwidth limit (e.g., "500K")
PARALLEL_DOWNLOADS=${DOWNLOAD_PARALLEL:-1}  # Number of parallel downloads
STATUS_FILE="${KEYS_DIR}/.download_status"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create keys directory
mkdir -p "$KEYS_DIR"

# Function to calculate exponential backoff
calculate_backoff() {
    local attempt=$1
    local delay=$((INITIAL_RETRY_DELAY * (2 ** (attempt - 1))))
    if [ $delay -gt $MAX_RETRY_DELAY ]; then
        delay=$MAX_RETRY_DELAY
    fi
    echo $delay
}

# Function to format bytes for human reading
format_bytes() {
    local bytes=$1
    if [ $bytes -gt 1073741824 ]; then
        echo "$(echo "scale=2; $bytes/1073741824" | bc) GB"
    elif [ $bytes -gt 1048576 ]; then
        echo "$(echo "scale=2; $bytes/1048576" | bc) MB"
    else
        echo "$(echo "scale=2; $bytes/1024" | bc) KB"
    fi
}

# Function to save download status
save_status() {
    local file="$1"
    local status="$2"
    local timestamp=$(date +%s)
    echo "${file}|${status}|${timestamp}" >> "$STATUS_FILE"
}

# Function to check if file was already completed
is_completed() {
    local file="$1"
    [ -f "$STATUS_FILE" ] && grep -q "^${file}|completed" "$STATUS_FILE"
}

# Enhanced download function with progress tracking
download_file() {
    local url="$1"
    local output="$2"
    local attempt=1
    local temp_output="${output}.tmp"
    local progress_file="${output}.progress"
    
    while [ $attempt -le $MAX_RETRIES ]; do
        local retry_delay=$(calculate_backoff $attempt)
        echo -e "${YELLOW}Downloading $url (attempt $attempt/$MAX_RETRIES)${NC}"
        
        # Build curl command with optional bandwidth limit
        local curl_cmd="curl -L --fail -H 'Accept: */*' -H 'Accept-Encoding: identity'"
        curl_cmd+=" -A 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'"
        curl_cmd+=" --connect-timeout 30 --max-time 0"  # No overall timeout
        curl_cmd+=" --progress-bar"
        
        if [ -n "$BANDWIDTH_LIMIT" ]; then
            curl_cmd+=" --limit-rate $BANDWIDTH_LIMIT"
            echo -e "${YELLOW}Bandwidth limited to $BANDWIDTH_LIMIT${NC}"
        fi
        
        # Check if partial download exists
        if [ -f "$temp_output" ]; then
            local resume_pos=$(stat -f%z "$temp_output" 2>/dev/null || stat -c%s "$temp_output" 2>/dev/null || echo "0")
            local formatted_pos=$(format_bytes $resume_pos)
            echo -e "${GREEN}Resuming from $formatted_pos${NC}"
            curl_cmd+=" -H 'Range: bytes=${resume_pos}-' -C -"
        fi
        
        curl_cmd+=" --output '$temp_output' '$url'"
        
        # Execute download and capture progress
        if eval "$curl_cmd" 2>&1 | tee "$progress_file"; then
            mv "$temp_output" "$output"
            rm -f "$progress_file"
            save_status "${output##*/}" "completed"
            return 0
        fi
        
        local exit_code=$?
        echo -e "${RED}Download failed (exit code: $exit_code)${NC}"
        
        # Check if it's a connection error vs other errors
        if [ $exit_code -eq 56 ] || [ $exit_code -eq 18 ] || [ $exit_code -eq 28 ]; then
            echo -e "${YELLOW}Connection issue detected. Will retry with exponential backoff.${NC}"
        fi
        
        if [ $attempt -lt $MAX_RETRIES ]; then
            echo -e "${YELLOW}Retrying in $retry_delay seconds...${NC}"
            echo "Tip: You can also manually resume by running this script again"
            sleep $retry_delay
        fi
        
        attempt=$((attempt + 1))
    done
    
    # Save failed status but keep partial file
    save_status "${output##*/}" "failed"
    return 1
}

verify_checksum() {
    local file="$1"
    local checksum_file="$2"
    local expected
    local actual

    if command -v sha256sum >/dev/null 2>&1; then
        CHECKSUM_CMD="sha256sum"
    else
        CHECKSUM_CMD="shasum -a 256"
    fi

    expected=$(grep "${file##*/}" "$checksum_file" | cut -d' ' -f1)
    actual=$($CHECKSUM_CMD "$file" | cut -d' ' -f1)

    echo "Expected checksum: $expected"
    echo "Actual checksum:   $actual"

    [ "$expected" = "$actual" ]
}

# Show current configuration
echo "========================================="
echo "Download Configuration:"
echo "  Max retries: $MAX_RETRIES"
echo "  Initial retry delay: ${INITIAL_RETRY_DELAY}s"
echo "  Max retry delay: ${MAX_RETRY_DELAY}s"
if [ -n "$BANDWIDTH_LIMIT" ]; then
    echo "  Bandwidth limit: $BANDWIDTH_LIMIT"
fi
echo "  Parallel downloads: $PARALLEL_DOWNLOADS"
echo ""
echo "To customize, set environment variables:"
echo "  DOWNLOAD_MAX_RETRIES=20"
echo "  DOWNLOAD_RETRY_DELAY=10"
echo "  DOWNLOAD_BANDWIDTH_LIMIT=500K"
echo "  DOWNLOAD_PARALLEL=2"
echo "========================================="
echo ""

# Download checksum file
CHECKSUM_FILE="${KEYS_DIR}/CHECKSUM"
if ! download_file "$CHECKSUM_URL" "$CHECKSUM_FILE"; then
    echo -e "${RED}Failed to download checksum file${NC}"
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
            "append-with-proofs_32:500"
            "update_32:500"
            "address-append_40:250"
        )
        echo -e "${YELLOW}WARNING: Full keys include files >6GB. Ensure stable connection!${NC}"
        ;;
    *)
        echo "Usage: $0 [light|full]"
        exit 1
        ;;
esac

# Count total files and calculate total size
total_files=0
completed_files=0
skipped_files=0
failed_files=0
total_size=0

# Build file list
declare -a FILE_LIST
for group in "${SUFFIXES[@]}"; do
    base=${group%:*}
    suffixes=${group#*:}
    for suffix in $suffixes; do
        for ext in key vkey; do
            FILE_LIST+=("${base}_${suffix}.${ext}")
            total_files=$((total_files + 1))
        done
    done
done

echo "Total files to process: $total_files"
echo ""

# Process downloads (with simple parallel support if requested)
process_download() {
    local file="$1"
    local index="$2"
    local output="${KEYS_DIR}/${file}"
    local temp_output="${output}.tmp"
    
    # Check if already completed in previous run
    if is_completed "$file"; then
        echo -e "${GREEN}[$index/$total_files] Skipping $file (marked as completed)${NC}"
        return 0
    fi
    
    # Check if file already exists and is valid
    if [ -f "$output" ] && verify_checksum "$output" "$CHECKSUM_FILE" 2>/dev/null; then
        echo -e "${GREEN}[$index/$total_files] Skipping $file (already downloaded and verified)${NC}"
        save_status "$file" "completed"
        return 0
    fi
    
    # Check if partial download exists
    if [ -f "$temp_output" ]; then
        local partial_size=$(stat -f%z "$temp_output" 2>/dev/null || stat -c%s "$temp_output" 2>/dev/null || echo "0")
        local formatted_size=$(format_bytes $partial_size)
        echo -e "${YELLOW}Found partial download for $file ($formatted_size)${NC}"
    fi
    
    echo -e "${YELLOW}[$index/$total_files] Downloading $file...${NC}"
    if download_file "${BASE_URL}/${file}" "$output"; then
        echo "Verifying checksum for $file..."
        if ! verify_checksum "$output" "$CHECKSUM_FILE"; then
            echo -e "${RED}Checksum verification failed for $file${NC}"
            rm -f "$output"
            rm -f "$temp_output"
            save_status "$file" "checksum_failed"
            return 1
        fi
        echo -e "${GREEN}[$index/$total_files] Successfully downloaded and verified $file${NC}"
        return 0
    else
        echo -e "${RED}Failed to download $file after $MAX_RETRIES attempts${NC}"
        echo "You can resume the download by running this script again"
        return 1
    fi
}

# Execute downloads
index=0
for file in "${FILE_LIST[@]}"; do
    index=$((index + 1))
    if process_download "$file" "$index"; then
        completed_files=$((completed_files + 1))
    else
        failed_files=$((failed_files + 1))
        # On mobile connections, offer to continue with remaining files
        if [ $failed_files -gt 0 ]; then
            echo ""
            echo -e "${YELLOW}Download failed. Continue with remaining files? (y/n)${NC}"
            read -r -n 1 response
            echo ""
            if [[ ! "$response" =~ ^[Yy]$ ]]; then
                break
            fi
        fi
    fi
done

# Summary
echo ""
echo "========================================="
if [ $failed_files -eq 0 ]; then
    echo -e "${GREEN}All files downloaded and verified successfully!${NC}"
else
    echo -e "${YELLOW}Download session completed with errors${NC}"
    echo -e "  Successful: ${GREEN}$completed_files${NC}"
    echo -e "  Failed: ${RED}$failed_files${NC}"
    echo ""
    echo "To resume failed downloads, run this script again."
    echo "Partial downloads will be automatically resumed."
fi
echo "========================================="

# Exit with appropriate code
[ $failed_files -eq 0 ] && exit 0 || exit 1