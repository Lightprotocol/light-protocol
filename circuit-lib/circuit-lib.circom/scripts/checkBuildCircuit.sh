#!/usr/bin/env bash

check_file_modified() {
    local filepath
    local modified=0

    # Check if we're in a Git repository
    if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
        echo "This function must be run inside a Git repository."
        exit 1
    fi

    # Ensure paths are provided as arguments
    if [[ "$#" -eq 0 ]]; then
        echo "Please provide file paths to check."
        exit 1
    fi

    # Iterate over the provided paths
    for filepath in "$@"; do
        # Check if the file exists in the working directory
        if [[ ! -e "$filepath" ]]; then
            echo "Warning: $filepath does not exist in the working directory."
            continue
        fi

        # Use `git diff` to see if the file has changes compared to the HEAD
        if git diff --quiet HEAD -- "$filepath"; then
            echo "$filepath has NOT been modified."
            modified=1
        else
            echo "$filepath has been MODIFIED."
        fi
    done

    if [[ $modified -eq 1 ]]; then
        exit 1
    fi
}

top_dir=`git rev-parse --show-toplevel`

check_file_modified "${top_dir}/programs/psp2in2out/src/verifying_key.rs"
check_file_modified "${top_dir}/programs/psp10in2out/src/verifying_key.rs"
check_file_modified "${top_dir}/programs/psp4in4out/src/verifying_key.rs"
check_file_modified "${top_dir}/programs/psp2in2out-storage/src/verifying_key.rs"

check_file_modified "${top_dir}/zk.js/build-circuits/transactionMasp2Main.zkey"
check_file_modified "${top_dir}/zk.js/build-circuits/transactionMasp10Main.zkey"
check_file_modified "${top_dir}/zk.js/build-circuits/transactionApp4Main.zkey"
