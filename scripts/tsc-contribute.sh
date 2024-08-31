# Usage:
# ./scripts/contribute-to-setup.sh <contribution_number> "<contributor_name>" \
# "<download_URL1>" "<download_URL2>" ... "<download_URLn>" \
# -- \
# "<upload_URL1>" "<upload_URL2>" ... "<upload_URLn>" "<contribution_file_upload_URL>"

# Ensure we're working from the root directory of the monorepo
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
cd "$REPO_ROOT"

# Names of phase2 files to download
PH2_FILES=(
    "inclusion_26_1.ph2"
    "inclusion_26_2.ph2"
    "inclusion_26_3.ph2"
    "inclusion_26_4.ph2"
    "inclusion_26_8.ph2"
    "non-inclusion_26_1.ph2"
    "non-inclusion_26_2.ph2"
    "combined_26_1_1.ph2"
    "combined_26_1_2.ph2"
    "combined_26_2_1.ph2"
    "combined_26_2_2.ph2"
    "combined_26_3_1.ph2"
    "combined_26_3_2.ph2"
    "combined_26_4_1.ph2"
    "combined_26_4_2.ph2"
)

# Function to validate URL format
validate_url() {
    local url=$1
    if [[ $url =~ ^https://[^/]+\.s3\.amazonaws\.com/\?AWSAccessKeyId=[^&]+&Signature=[^&]+&Expires=[0-9]+$ ]]; then
        return 0
    else
        return 1
    fi
}

# Check if all required arguments are provided
if [ $# -lt $((${#PH2_FILES[@]} * 2 + 4)) ]; then
    echo "Error: Insufficient arguments. Usage: $0 <contribution_number> <contributor_name> <download_URLs...> -- <upload_URLs...> <contribution_file_upload_URL>"
    exit 1
fi

CONTRIBUTION_NUMBER=$1
CONTRIBUTOR_NAME=$2
shift 2

# Validate contribution number
if ! [[ "$CONTRIBUTION_NUMBER" =~ ^[0-9]+$ ]]; then
    echo "Error: Contribution number must be a number."
    exit 1
fi

# Create the output directory
OUTPUT_DIR="$REPO_ROOT/ceremony/contribute/ph2-files"
mkdir -p "$OUTPUT_DIR"

# Find the separator index
separator_index=0
for i in "$@"; do
    if [ "$i" = "--" ]; then
        break
    fi
    ((separator_index++))
done

# Check if the number of download URLs matches the number of PH2 files
if [ $separator_index -ne ${#PH2_FILES[@]} ]; then
    echo "Error: Number of download URLs (${separator_index}) does not match the number of PH2 files (${#PH2_FILES[@]})."
    exit 1
fi

# Check if the number of upload URLs matches the number of PH2 files plus one for the contribution file
if [ $(($# - $separator_index - 1)) -ne $((${#PH2_FILES[@]} + 1)) ]; then
    echo "Error: Number of upload URLs ($((${#PH2_FILES[@]} + 1))) does not match the number of PH2 files plus one for the contribution file."
    exit 1
fi

# Process each download URL
for i in "${!PH2_FILES[@]}"; do
    url="$1"
    shift
    if ! validate_url "$url"; then
        echo "Error: Invalid URL format: $url"
        exit 1
    fi

    base_name="${PH2_FILES[$i]%.ph2}"
    output_file="$OUTPUT_DIR/${base_name}_contribution_${CONTRIBUTION_NUMBER}.ph2"

    echo "Downloading $output_file"
    curl --output "$output_file" "$url"

    if [ $? -ne 0 ]; then
        echo "Error: Failed to download $output_file"
        exit 1
    fi

    echo "Successfully downloaded $output_file"
done

# Check for separator
if [ "$1" != "--" ]; then
    echo "Error: Missing separator '--' between download and upload URLs."
    exit 1
fi
shift

echo "All files have been downloaded successfully to $OUTPUT_DIR."

# Clone and build semaphore-mtb-setup if not exists
cd ..
if [ ! -d "semaphore-mtb-setup" ]; then
    git clone https://github.com/worldcoin/semaphore-mtb-setup
    cd semaphore-mtb-setup
    go build -v
else
    cd semaphore-mtb-setup
fi
cd "$REPO_ROOT"

# Prepare output directory
OUTPUT_DIR="$REPO_ROOT/ceremony/contribute/outputs"
mkdir -p "$OUTPUT_DIR"

# Contribution file
CONTRIB_FILE="$OUTPUT_DIR/${CONTRIBUTOR_NAME}_CONTRIBUTION.txt"
> "$CONTRIB_FILE"

# Contribute to each .ph2 file
for ph2_file in "$REPO_ROOT/ceremony/contribute/ph2-files"/*_contribution_${CONTRIBUTION_NUMBER}.ph2; do
    base_name=$(basename "$ph2_file" "_contribution_${CONTRIBUTION_NUMBER}.ph2")
    new_contribution=$((CONTRIBUTION_NUMBER + 1))
    output_file="${base_name}_contribution_${new_contribution}.ph2"
    
    echo "Contributing to $ph2_file"
    contribution_hash=$(../semaphore-mtb-setup/semaphore-mtb-setup p2c "$ph2_file" "$OUTPUT_DIR/$output_file")
    
    echo "$base_name $contribution_hash" >> "$CONTRIB_FILE"
    echo "Contribution hash for $base_name: $contribution_hash"
done

echo "All contributions completed. Hashes stored in $CONTRIB_FILE"

# Upload new .ph2 files and contribution file
echo "Uploading new .ph2 files and contribution file..."

# Get the last argument (upload URL for contribution file)
contribution_upload_url="${@: -1}"

# Upload new .ph2 files
for ph2_file in "$OUTPUT_DIR"/*_contribution_$((CONTRIBUTION_NUMBER + 1)).ph2; do
    if [ $# -eq 1 ]; then
        echo "Error: Not enough upload URLs for .ph2 files."
        exit 1
    fi
    
    echo "Uploading $(basename "$ph2_file")..."
    curl -v -T "$ph2_file" "$1"
    
    if [ $? -ne 0 ]; then
        echo "Error: Failed to upload $(basename "$ph2_file")"
        exit 1
    fi
    
    shift
done

# Upload contribution file
echo "Uploading contribution file..."
curl -v -T "$CONTRIB_FILE" "$contribution_upload_url"

if [ $? -ne 0 ]; then
    echo "Error: Failed to upload contribution file"
    exit 1
fi

echo "All files uploaded successfully."
