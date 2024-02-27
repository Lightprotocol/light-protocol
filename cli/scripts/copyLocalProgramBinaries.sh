#!/usr/bin/env sh

set -eux

keys="account_compression psp_compressed_pda psp_compressed_token light"

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