#!/usr/bin/env sh

keys="merkle_tree_program verifier_program_zero verifier_program_one verifier_program_two verifier_program_storage user_registry"

mkdir -p bin/programs
cd ..
for key in $keys
do
    cp system-programs/target/deploy/$key.so cli/bin/$key.so
done

cp third-party/solana-program-library/spl_noop.so cli/bin/spl_noop.so
cd -