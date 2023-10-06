#!/usr/bin/env bash

keys="merkle_tree_program verifier_program_zero verifier_program_one verifier_program_two verifier_program_storage user_registry"

out_dir="cli/bin"
if [[ ! -e $out_dir ]]; then
    mkdir -p $out_dir
fi
mkdir -p bin/programs
cd ..
for key in $keys
do
    cp system-programs/target/deploy/$key.so $out_dir/$key.so
done

cp third-party/solana-program-library/spl_noop.so $out_dir/spl_noop.so
cd -
