#!/usr/bin/env bash

killall light-prover

# Get the root directory of the Git repository
root_dir=$(git rev-parse --show-toplevel)

# Change the directory to 'gnark-prover' within the Git root directory
# shellcheck disable=SC2164
cd "$root_dir/gnark-prover"

# If 'gnark-prover' directory does not exist, print error and exit
if [ $? -ne 0 ]; then
    echo "Directory gnark-prover does not exist in the Git root directory. Run \`git submodule update --init\` to fetch the submodule."
    exit 1
fi

go build

./light-prover start &