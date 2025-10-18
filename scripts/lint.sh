#!/usr/bin/env bash

set -e
npx nx run-many --target=format:check --all
npx nx run-many --target=lint --all

cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings

echo "Testing feature combinations..."

# Test no-default-features for all library crates
echo "Testing all library crates with --no-default-features..."
NO_DEFAULT_CRATES=(
    "light-account-checks"
    "light-batched-merkle-tree"
    "light-bloom-filter"
    "light-compressed-account"
    "light-compressible"
    "light-concurrent-merkle-tree"
    "light-ctoken-types"
    "light-hash-set"
    "light-hasher"
    "light-indexed-merkle-tree"
    "light-macros"
    "light-merkle-tree-metadata"
    "light-verifier"
    "light-zero-copy"
    "light-heap"
    "light-array-map"
    "light-indexed-array"
    "aligned-sized"
    "light-sdk-types"
    "light-sdk-pinocchio"
    "light-sdk-macros"
    "light-compressed-token-sdk"
    "light-compressed-token-types"
    "light-sdk"
)

for crate in "${NO_DEFAULT_CRATES[@]}"; do
    echo "Checking $crate with --no-default-features..."
    cargo check -p "$crate" --no-default-features
done

# Test pinocchio feature for all crates that have it
PINOCCHIO_CRATES=(
    "light-hasher"
    "light-indexed-merkle-tree"
    "light-zero-copy"
    "light-bloom-filter"
    "light-compressed-account"
    "light-merkle-tree-metadata"
    "light-macros"
    "light-batched-merkle-tree"
    "light-concurrent-merkle-tree"
    "light-verifier"
    "light-account-checks"
    "light-compressible"
)

for crate in "${PINOCCHIO_CRATES[@]}"; do
    echo "Checking $crate with pinocchio feature..."
    cargo check -p "$crate" --features pinocchio
done

# Test solana feature for all crates that have it
SOLANA_CRATES=(
    "light-hasher"
    "light-indexed-merkle-tree"
    "light-zero-copy"
    "light-bloom-filter"
    "light-compressed-account"
    "light-hash-set"
    "light-merkle-tree-metadata"
    "light-ctoken-types"
    "light-macros"
    "light-batched-merkle-tree"
    "light-concurrent-merkle-tree"
    "light-verifier"
    "light-account-checks"
    "light-compressible"
)

for crate in "${SOLANA_CRATES[@]}"; do
    echo "Checking $crate with solana feature..."
    cargo check -p "$crate" --features solana
done

# Test anchor feature for all crates that have it
ANCHOR_CRATES=(
    "light-indexed-merkle-tree"
    "light-compressed-account"
    "light-merkle-tree-metadata"
    "light-ctoken-types"
    "light-verifier"
    "light-compressible"
    "light-sdk-types"
    "light-sdk"
    "light-compressed-token-sdk"
    "light-compressed-token-types"
)

for crate in "${ANCHOR_CRATES[@]}"; do
    echo "Checking $crate with anchor feature..."
    cargo check -p "$crate" --features anchor
done

for crate in "${NO_DEFAULT_CRATES[@]}"; do
    echo "Checking $crate with --no-default-features..."
    cargo test -p "$crate" --no-run
done
