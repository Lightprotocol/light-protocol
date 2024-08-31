#!/bin/bash

# Usage:
# 1. For the first user (Alice):
#    ./scripts/tsc-create-urls.sh <bucket_name> alice 0
# (now Alice runs tsc-contribute.sh to download the files and upload her contribution)
# 2. For the next user (e.g., Bob):
#    ./scripts/tsc-create-urls.sh <bucket_name> bob 1
# (now Bob runs tsc-contribute.sh to download the files and upload his contribution)
# 3. Continue for subsequent users, incrementing the last number each time.
#
# Note: Before running this script, ensure you have:
# - Configured AWS credentials using 'aws configure'
# - Proper permissions to access the specified S3 bucket
# - For the first user (Alice), manually uploaded initial files to the S3 bucket
#   with names like 'inclusion_26_1_contribution_0.ph2'
# - Set up the S3 bucket:
#   - Place all files in the root bucket
#   - Enable WORM (Write Once Read Many) model
#   - Enable S3 versioning
#   - Set up IAM roles and bucket policies to restrict access
#   - Enable server-side encryption

create_presigned_url() {
    local bucket_name=$1
    local object_name=$2
    local operation=$3
    local expiration=$4

    if [ "$operation" = "put_object" ]; then
        http_method="--method PUT"
    else
        http_method=""
    fi

    url=$(aws s3 presign s3://${bucket_name}/${object_name} --expires-in ${expiration} ${http_method})
    echo $url
}

# Check if all required arguments are provided
if [ $# -ne 3 ]; then
    echo "Usage: $0 <bucket_name> <current_user> <current_number>"
    exit 1
fi

bucket_name=$1
current_user=$2
current_number=$3

# Set expiration time (1 hour)
expiration=3600

# Array of ph2 file names
ph2_files=(
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

next_number=$((current_number + 1))

echo "Download URLs for $current_user (contribution $current_number):"
for file in "${ph2_files[@]}"; do
    object_name="${file%.ph2}_contribution_${current_number}.ph2"
    url=$(create_presigned_url "$bucket_name" "$object_name" "get_object" "$expiration")
    echo "$url"
done

echo -e "\n--\n"

echo "Upload URLs for $current_user (contribution $next_number):"
for file in "${ph2_files[@]}"; do
    object_name="${file%.ph2}_contribution_${next_number}.ph2"
    url=$(create_presigned_url "$bucket_name" "$object_name" "put_object" "$expiration")
    echo "$url"
done

contribution_file="${current_user}_CONTRIBUTION.txt"
url=$(create_presigned_url "$bucket_name" "$contribution_file" "put_object" "$expiration")
echo -e "\nContribution file upload URL for $current_user:"
echo "$url"