#!/usr/bin/env sh

set -eux

keys="account_compression light_system_program light_compressed_token light_registry"

out_dir="`git rev-parse --show-toplevel`/cli/bin"
if [ ! -e $out_dir ]; then
    mkdir -p $out_dir
fi
cd ..
for key in $keys
do
    cp "`git rev-parse --show-toplevel`/target/deploy/$key.so" $out_dir/$key.so
done

cp third-party/solana-program-library/spl_noop.so $out_dir/spl_noop.so
cd -
