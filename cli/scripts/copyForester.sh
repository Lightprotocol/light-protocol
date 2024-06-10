#!/usr/bin/env sh

set -eux

root_dir=$(git rev-parse --show-toplevel)
out_dir="$root_dir/cli/bin"
if [ ! -e "$out_dir" ]; then
    mkdir -p "$out_dir"
fi

cargo build --release --bin forester
cp "$root_dir/target/release/forester" "$out_dir"
cp "$root_dir/forester/forester.toml" "$out_dir"
