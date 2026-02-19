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

# Batched feature checks - cargo can check multiple -p flags in one invocation,
# sharing compilation work and drastically reducing wall-clock time.

echo "Checking all library crates with --no-default-features..."
cargo check \
    -p light-account-checks \
    -p light-batched-merkle-tree \
    -p light-bloom-filter \
    -p light-compressed-account \
    -p light-compressible \
    -p light-concurrent-merkle-tree \
    -p light-token-interface \
    -p light-hash-set \
    -p light-hasher \
    -p light-indexed-merkle-tree \
    -p light-macros \
    -p light-merkle-tree-metadata \
    -p light-verifier \
    -p light-zero-copy \
    -p light-heap \
    -p light-array-map \
    -p light-indexed-array \
    -p aligned-sized \
    -p light-sdk-types \
    -p light-sdk-pinocchio \
    -p light-token-pinocchio \
    -p light-sdk-macros \
    -p light-token \
    -p light-token-types \
    -p light-sdk \
    -p light-account \
    -p light-account-pinocchio \
    -p light-client \
    -p light-compressed-token-sdk \
    -p light-instruction-decoder \
    -p light-program-test \
    -p light-token-client \
    -p csdk-anchor-full-derived-test \
    --no-default-features

echo "Checking pinocchio feature..."
cargo check \
    -p light-hasher \
    -p light-indexed-merkle-tree \
    -p light-zero-copy \
    -p light-bloom-filter \
    -p light-compressed-account \
    -p light-merkle-tree-metadata \
    -p light-macros \
    -p light-batched-merkle-tree \
    -p light-concurrent-merkle-tree \
    -p light-verifier \
    -p light-account-checks \
    -p light-compressible \
    --features pinocchio

echo "Checking solana feature..."
cargo check \
    -p light-hasher \
    -p light-indexed-merkle-tree \
    -p light-zero-copy \
    -p light-bloom-filter \
    -p light-compressed-account \
    -p light-hash-set \
    -p light-merkle-tree-metadata \
    -p light-token-interface \
    -p light-macros \
    -p light-batched-merkle-tree \
    -p light-concurrent-merkle-tree \
    -p light-verifier \
    -p light-account-checks \
    -p light-compressible \
    --features solana

echo "Checking anchor feature..."
cargo check \
    -p light-indexed-merkle-tree \
    -p light-compressed-account \
    -p light-merkle-tree-metadata \
    -p light-token-interface \
    -p light-verifier \
    -p light-compressible \
    -p light-sdk-types \
    -p light-sdk \
    -p light-token \
    -p light-token-types \
    --features anchor

echo "Checking all library crates compile tests..."
cargo test \
    -p light-account-checks \
    -p light-batched-merkle-tree \
    -p light-bloom-filter \
    -p light-compressed-account \
    -p light-compressible \
    -p light-concurrent-merkle-tree \
    -p light-token-interface \
    -p light-hash-set \
    -p light-hasher \
    -p light-indexed-merkle-tree \
    -p light-macros \
    -p light-merkle-tree-metadata \
    -p light-verifier \
    -p light-zero-copy \
    -p light-heap \
    -p light-array-map \
    -p light-indexed-array \
    -p aligned-sized \
    -p light-sdk-types \
    -p light-sdk-pinocchio \
    -p light-token-pinocchio \
    -p light-sdk-macros \
    -p light-token \
    -p light-token-types \
    -p light-sdk \
    -p light-account \
    -p light-account-pinocchio \
    -p light-client \
    -p light-compressed-token-sdk \
    -p light-instruction-decoder \
    -p light-program-test \
    -p light-token-client \
    -p csdk-anchor-full-derived-test \
    --no-run
