#!/usr/bin/env sh

set -eux

root_dir=$(git rev-parse --show-toplevel)
out_dir="$root_dir/cli/bin"
if [ ! -e "$out_dir" ]; then
    mkdir -p "$out_dir"
fi

keys="account_compression light_system_program_pinocchio light_compressed_token light_registry"
for key in $keys
do
    # cli build process deletes target/deploy contents, so fall back to
    # sbf-solana-solana
    src_deploy="$root_dir/target/deploy/$key.so"
    src_sbf_release="$root_dir/target/sbf-solana-solana/release/$key.so"

    if [ -f "$src_deploy" ]; then
        cp "$src_deploy" "$out_dir/$key.so"
    elif [ -f "$src_sbf_release" ]; then
        cp "$src_sbf_release" "$out_dir/$key.so"
    else
        echo "Error: $key.so not found in $src_deploy or $src_sbf_release" >&2
        exit 1
    fi
done
cp "$root_dir"/third-party/solana-program-library/spl_noop.so "$out_dir"/spl_noop.so
