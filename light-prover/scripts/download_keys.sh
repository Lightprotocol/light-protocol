#!/usr/bin/env bash

set -e

ROOT_DIR="$(git rev-parse --show-toplevel)"
KEYS_DIR="${ROOT_DIR}/light-prover/proving-keys"

mkdir -p "$KEYS_DIR"

# Circuits with multiple inputs (we are going to cease supporting address trees of
# height 26 hence no point in doing a new trusted setup for these circuits)
MAINNET_INCLUSION_26_BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"
NON_INCLUSION_26_BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"
COMBINED_26_26_BUCKET="bafybeiacecbc3hnlmgifpe6v3h3r3ord7ifedjj6zvdv7nxgkab4npts54"

# Circuits with unified inputs4"
INCLUSION_32_BUCKET="bafybeihhka7qkdiq3hhur6hycmaqzgov4vpzw5jmjsvomjbcybvqc4exgy"
NON_INCLUSION_40_BUCKET="bafybeigp64bqx2k2ogwur4efzcxczm22jkxye57p5mnmvgzvlpb75b66m4"

COMBINED_32_40_BUCKET="bafybeihhka7qkdiq3hhur6hycmaqzgov4vpzw5jmjsvomjbcybvqc4exgy"
APPEND_WITH_PROOFS_32_BUCKET="bafybeihhka7qkdiq3hhur6hycmaqzgov4vpzw5jmjsvomjbcybvqc4exgy"
UPDATE_32_BUCKET="bafybeihhka7qkdiq3hhur6hycmaqzgov4vpzw5jmjsvomjbcybvqc4exgy"

APPEND_ADDRESS_40_BUCKET="bafybeib2rajatndlpslpqhf4vrbekpyyehjt5byivfzxl36c5p67ypddvu"

get_bucket_url() {
    local FILE="$1"

    if [[ $FILE == inclusion_32_* ]]; then
        echo "https://${INCLUSION_32_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == mainnet_inclusion_26_* ]]; then
        echo "https://${MAINNET_INCLUSION_26_BUCKET}.ipfs.w3s.link/${FILE#mainnet_}"
    elif [[ $FILE == non-inclusion_26_* ]]; then
        echo "https://${NON_INCLUSION_26_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == non-inclusion_40_* ]]; then
        echo "https://${NON_INCLUSION_40_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == combined_32_40_* ]]; then 
        echo "https://${COMBINED_32_40_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == combined_26_* ]]; then
        echo "https://${COMBINED_26_26_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == append-with-proofs_32_* ]]; then
        echo "https://${APPEND_WITH_PROOFS_32_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == address-append_40_* ]]; then
        echo "https://${APPEND_ADDRESS_40_BUCKET}.ipfs.w3s.link/${FILE}"
    elif [[ $FILE == update_32_* ]]; then
        echo "https://${UPDATE_32_BUCKET}.ipfs.w3s.link/${FILE}"
    fi
}

case "$1" in
    "light")
        SUFFIXES=(
            "inclusion_32:1 2 3 4 8"
            "mainnet_inclusion_26:1 2 3 4 8"
            "non-inclusion_26:1 2 3 4 8"
            "non-inclusion_40:1 2 3 4 8"
            "combined_26:1_1 1_2 2_1 2_2 3_1 3_2 4_1 4_2"
            "combined_32_40:1_1 1_2 1_3 1_4 2_1 2_2 2_3 2_4 3_1 3_2 3_3 3_4 4_1 4_2 4_3 4_4"
            "append-with-proofs_32:1 10"
            "update_32:1 10"
            "address-append_40:1 10"
        )
        ;;
    "full")
        SUFFIXES=(
            "inclusion_32:1 2 3 4 8"
            "mainnet_inclusion_26:1 2 3 4 8"
            "non-inclusion_26:1 2 3 4 8"
            "non-inclusion_40:1 2 3 4 8"
            "combined_26_26:1_1 1_2 2_1 2_2 3_1 3_2 4_1 4_2"
            "combined_32_40:1_1 1_2 1_3 1_4 2_1 2_2 2_3 2_4 3_1 3_2 3_3 3_4 4_1 4_2 4_3 4_4"
            "append-with-proofs_32:1 10 100 500 1000"
            "update_32:1 10 100 500 1000"
            "address-append_40:1 10 100 250 500 1000"
        )
        ;;
    *)
        echo "Usage: $0 [light|full]"
        exit 1
        ;;
esac

for group in "${SUFFIXES[@]}"; do
    base=${group%:*}
    suffixes=${group#*:}
    for suffix in $suffixes; do
        for ext in key vkey; do
            file="${base}_${suffix}.${ext}"
            url="$(get_bucket_url "$file")"
            echo "Downloading $file"
            curl -S --retry 3 -o "${KEYS_DIR}/${file}" "$url"
        done
    done
done
