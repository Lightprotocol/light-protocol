import os
import hashlib
import sys

def calculate_sha256(filepath):
    sha256_hash = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            sha256_hash.update(chunk)
    return sha256_hash.hexdigest()

def generate_checksums(directory):
    checksums = {}
    files = [f for f in os.listdir(directory) if os.path.isfile(os.path.join(directory, f))]

    print(f"Calculating checksums for {len(files)} files...")
    for i, filename in enumerate(files, 1):
        filepath = os.path.join(directory, filename)
        checksums[filename] = calculate_sha256(filepath)
        sys.stdout.write(f"\rProgress: {i}/{len(files)} files processed")
        sys.stdout.flush()

    print()

    checksum_file = os.path.join(directory, "CHECKSUM")
    with open(checksum_file, "w") as f:
        for filename, checksum in sorted(checksums.items()):
            f.write(f"{checksum}  {filename}\n")

    print(f"Checksums written to {checksum_file}")

if __name__ == "__main__":
    generate_checksums("./proving-keys")
