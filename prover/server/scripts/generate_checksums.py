import os
import hashlib
import boto3
from tqdm import tqdm

def calculate_sha256(filepath):
    sha256_hash = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            sha256_hash.update(chunk)
    return sha256_hash.hexdigest()

def generate_checksums(directory):
    checksums = {}
    files = [f for f in os.listdir(directory) if os.path.isfile(os.path.join(directory, f))]
    
    print("Calculating checksums...")
    for filename in tqdm(files):
        filepath = os.path.join(directory, filename)
        checksums[filename] = calculate_sha256(filepath)
    
    with open("CHECKSUM", "w") as f:
        for filename, checksum in checksums.items():
            f.write(f"{checksum}  {filename}\n")
    
if __name__ == "__main__":
    generate_checksums("./proving-keys")
