#!/usr/bin/env sh

set -eux

keys="light_merkle_tree_program light_psp2in2out light_psp10in2out light_psp4in4out_app_storage light_psp2in2out_storage light_user_registry psp_compressed_token psp_account_compression"

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
cargo build --release -p macro-circom
cp ../target/release/macro-circom $out_dir/macro-circom
