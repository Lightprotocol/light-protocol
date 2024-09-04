#!/usr/bin/env python3

import sys
import boto3
from botocore.exceptions import ClientError

def create_presigned_url(bucket_name, object_key, expiration, http_method='GET'):
    s3_client = boto3.client('s3', region_name='eu-central-1')
    try:
        return s3_client.generate_presigned_url(
            'get_object' if http_method == 'GET' else 'put_object',
            Params={'Bucket': bucket_name, 'Key': object_key},
            ExpiresIn=expiration,
            HttpMethod=http_method
        )
    except ClientError as e:
        print(f"Error creating presigned URL: {e}")
        return None

if len(sys.argv) < 4:
    print("Usage: python3 tsc-create-urls.py <bucket_name> <current_user> <current_number> [next_user]")
    sys.exit(1)

bucket_name = sys.argv[1]
current_user = sys.argv[2]
current_number = int(sys.argv[3])
next_user = sys.argv[4] if len(sys.argv) > 4 else None
next_number = current_number + 1
expiration = 3600

PH2_FILES = [
    "inclusion_26_1",
    # "inclusion_26_2",
    # "inclusion_26_3",
    # "inclusion_26_4",
    # "inclusion_26_8",
    # "non-inclusion_26_1",
    # "non-inclusion_26_2",
    # "combined_26_1_1",
    # "combined_26_1_2",
    # "combined_26_2_1",
    # "combined_26_2_2",
    # "combined_26_3_1",
    # "combined_26_3_2",
    # "combined_26_4_1",
    # "combined_26_4_2",
]

print(f"./scripts/tsc-contribute.sh {current_number} \"{current_user}\"", end=" ")

# Download URLs for current number
for file in PH2_FILES:
    download_file = f"{file}_{current_user}_contribution_{current_number}.ph2"
    url = create_presigned_url(bucket_name, download_file, expiration)
    print(f"\"{url}\"", end=" ")

# Upload URLs for next number
for file in PH2_FILES:
    upload_file = f"{file}_{current_user}_contribution_{next_number}.ph2"
    url = create_presigned_url(bucket_name, upload_file, expiration, http_method='PUT')
    print(f"\"{url}\"", end=" ")

# Add URL for contribution file (assuming this is an upload)
contrib_file = f"{current_user}_CONTRIBUTION_{current_number}.txt"
url = create_presigned_url(bucket_name, contrib_file, expiration, http_method='PUT')
print(f"\"{url}\"")