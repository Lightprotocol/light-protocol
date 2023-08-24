#!/usr/bin/env sh

keys="merkle_tree_program verifier_program_zero verifier_program_one verifier_program_two verifier_program_storage"

mkdir -p bin/programs
cd ..
for key in $keys
do
    cp light-system-programs/target/deploy/$key.so cli/bin/$key.so
done

cp test-env/programs/spl_noop.so cli/bin/spl_noop.so
cd -