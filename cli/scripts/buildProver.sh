#!/usr/bin/env sh

set -eux

build_prover() {
  GOOS=$1 GOARCH=$2 go build -o "$3"
}

# Parse command line arguments
RELEASE_ONLY=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --release-only)
      RELEASE_ONLY=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--release-only]"
      exit 1
      ;;
  esac
done

root_dir="$(git rev-parse --show-toplevel)"
gnark_dir="${root_dir}/prover/server"
out_dir="${root_dir}/cli/bin"
cli_dir="${root_dir}/cli"

if [ ! -e "$out_dir" ]; then
    mkdir -p "$out_dir"
fi

# Check if proving keys exist before copying
if [ ! -d "${gnark_dir}/proving-keys" ] || [ -z "$(ls -A "${gnark_dir}/proving-keys" 2>/dev/null)" ]; then
    echo "ERROR: Proving keys not found at ${gnark_dir}/proving-keys"
    echo "Please run: ./prover/server/scripts/download_keys.sh light"
    exit 1
fi

# Create proving-keys directory in output
mkdir -p "$out_dir/proving-keys"

if [ "$RELEASE_ONLY" = true ]; then
    echo "Release mode: copying only keys listed in package.json"
    # Dynamically read .key files from package.json files field
    # Extract all lines containing "/bin/proving-keys/" and ".key"
    key_files=$(node -e "
const pkg = require('${cli_dir}/package.json');
const keyFiles = pkg.files
  .filter(f => f.includes('/bin/proving-keys/') && f.endsWith('.key'))
  .map(f => f.split('/').pop());
console.log(keyFiles.join(' '));
")

    # Copy only the specified .key files
    for key_file in $key_files; do
        if [ -f "${gnark_dir}/proving-keys/${key_file}" ]; then
            cp "${gnark_dir}/proving-keys/${key_file}" "$out_dir/proving-keys/${key_file}"
            echo "Copied (release): ${key_file}"
        else
            echo "WARNING: ${key_file} not found in ${gnark_dir}/proving-keys"
        fi
    done
else
    echo "Development mode: copying ALL .key files"
    # Copy ALL .key files from prover directory
    for key_file in "${gnark_dir}/proving-keys"/*.key; do
        if [ -f "$key_file" ]; then
            filename=$(basename "$key_file")
            cp "$key_file" "$out_dir/proving-keys/$filename"
            echo "Copied (all): $filename"
        fi
    done
fi

cd "$gnark_dir"

# Windows
build_prover windows amd64 "$out_dir"/prover-windows-x64.exe
build_prover windows arm64 "$out_dir"/prover-windows-arm64.exe

# MacOS
build_prover darwin amd64 "$out_dir"/prover-darwin-x64
build_prover darwin arm64 "$out_dir"/prover-darwin-arm64

# Linux
build_prover linux amd64 "$out_dir"/prover-linux-x64
build_prover linux arm64 "$out_dir"/prover-linux-arm64
