#!/usr/bin/env bash
set -euo pipefail

# Check that program-libs and programs do not depend on sdk-libs crates
# This enforces the architectural constraint that program-libs and programs
# are lower-level and should not depend on higher-level sdk-libs.

echo "Checking dependency constraints..."

# SDK-libs crates that program-libs and programs must NOT depend on
SDK_LIBS_CRATES=(
    "light-sdk"
    "light-sdk-types"
    "light-sdk-macros"
    "light-sdk-pinocchio"
    "light-token"
    "light-token-types"
    "light-client"
    "light-program-test"
    "light-event"
    "light-prover-client"
    "photon-api"
)

# Crates in program-libs that should not depend on sdk-libs
PROGRAM_LIBS_CRATES=(
    "light-account-checks"
    "light-batched-merkle-tree"
    "light-bloom-filter"
    "light-compressed-account"
    "light-compressible"
    "light-concurrent-merkle-tree"
    "light-token-interface"
    "light-hash-set"
    "light-hasher"
    "light-indexed-merkle-tree"
    "light-macros"
    "light-merkle-tree-metadata"
    "light-verifier"
    "light-zero-copy"
    "light-zero-copy-derive"
    "light-heap"
    "light-array-map"
    "light-indexed-array"
    "aligned-sized"
)

# Programs that should not depend on sdk-libs
PROGRAM_CRATES=(
    "account-compression"
    "light-compressed-token"
    "light-registry"
    "light-system-program"
)

check_no_sdk_deps() {
    local crate="$1"
    local tree_output
    tree_output=$(cargo tree -p "$crate" --edges normal 2>/dev/null)

    for sdk_crate in "${SDK_LIBS_CRATES[@]}"; do
        if echo "$tree_output" | grep -q " ${sdk_crate} v"; then
            echo "ERROR: $crate depends on sdk-libs crate: $sdk_crate"
            return 1
        fi
    done
    return 0
}

CONSTRAINT_FAILED=0

echo "Checking program-libs crates do not depend on sdk-libs..."
for crate in "${PROGRAM_LIBS_CRATES[@]}"; do
    if ! check_no_sdk_deps "$crate"; then
        CONSTRAINT_FAILED=1
    fi
done

echo "Checking programs do not depend on sdk-libs..."
for crate in "${PROGRAM_CRATES[@]}"; do
    if ! check_no_sdk_deps "$crate"; then
        CONSTRAINT_FAILED=1
    fi
done

if [ "$CONSTRAINT_FAILED" -eq 1 ]; then
    echo ""
    echo "FAILED: Some program-libs or programs depend on sdk-libs crates."
    echo "This is not allowed. program-libs and programs must only depend on other program-libs or external crates."
    exit 1
fi

# Check that no crates have light-test-utils as a regular dependency
# (dev-dependencies are allowed, --edges normal excludes them)
# Excludes: program-tests/*, sdk-tests/*, xtask (these are test/build crates)
echo ""
echo "Checking that no crates depend on light-test-utils (dev-deps allowed)..."

# Use inverse lookup to find what depends on light-test-utils
# Skip the first line (light-test-utils itself) and filter out test crates
dependents=$(cargo tree --workspace --edges normal -i light-test-utils 2>/dev/null | tail -n +2 | grep -v "program-tests/" | grep -v "sdk-tests/" | grep -v "xtask" || true)
if [ -n "$dependents" ]; then
    echo "ERROR: Found crates with light-test-utils as a regular dependency:"
    echo "$dependents"
    echo ""
    echo "FAILED: light-test-utils should only be used as a dev-dependency."
    exit 1
fi

echo "All dependency constraints satisfied."
