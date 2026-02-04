#!/usr/bin/env bash

set -e

# JS linting (use subshells to avoid directory issues)
(cd js/stateless.js && pnpm prettier --write . && pnpm lint)
(cd js/compressed-token && pnpm prettier --write . && pnpm lint)

# Rust linting
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Check that program-libs and programs don't depend on sdk-libs
./scripts/check-dependency-constraints.sh

# Check that READMEs are up-to-date with cargo-rdme
echo "Checking READMEs are up-to-date..."
if ! command -v cargo-rdme &> /dev/null; then
    cargo install cargo-rdme
fi
for toml in $(find program-libs sdk-libs -name '.cargo-rdme.toml' -type f); do
    crate_dir=$(dirname "$toml")
    echo "Checking README in $crate_dir..."
    (cd "$crate_dir" && cargo rdme --check --no-fail-on-warnings)
done

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
    "light-token-interface"
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
    "light-token-pinocchio"
    "light-sdk-macros"
    "light-token"
    "light-token-types"
    "light-sdk"
    "light-account"
    "light-account-pinocchio"
    "light-client"
    "light-compressed-token-sdk"
    "light-instruction-decoder"
    "light-program-test"
    "light-token-client"
    "csdk-anchor-full-derived-test"
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
    "light-token-interface"
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
    "light-token-interface"
    "light-verifier"
    "light-compressible"
    "light-sdk-types"
    "light-sdk"
    "light-token"
    "light-token-types"
)

for crate in "${ANCHOR_CRATES[@]}"; do
    echo "Checking $crate with anchor feature..."
    cargo check -p "$crate" --features anchor
done

for crate in "${NO_DEFAULT_CRATES[@]}"; do
    echo "Checking $crate with --no-default-features..."
    cargo test -p "$crate" --no-run
done
