#!/usr/bin/env sh

set -eux

build_prover() {
  GOOS=$1 GOARCH=$2 go build -o "$3"
}

root_dir="$(git rev-parse --show-toplevel)"
gnark_dir="${root_dir}/light-prover"
out_dir="${root_dir}/cli/bin"

if [ ! -e "$out_dir" ]; then
    mkdir -p "$out_dir"
fi

cp -r "${gnark_dir}/proving-keys" "$out_dir"

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
