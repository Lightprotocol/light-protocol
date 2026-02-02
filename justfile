# Light Protocol Monorepo
set dotenv-load

export SBF_OUT_DIR := "target/deploy"
export REDIS_URL := env_var_or_default("REDIS_URL", "redis://localhost:6379")
export CARGO_FEATURES := env_var_or_default("CARGO_FEATURES", "v2_ix")

# Submodules
mod prover 'prover/server'
mod programs 'programs'
mod program-tests 'program-tests'
mod program-libs 'program-libs'
mod sdk-libs 'sdk-libs'
mod sdk-tests 'sdk-tests'
mod js 'js'
mod cli 'cli'
mod forester 'forester'

default:
    @just --list

# === Setup ===
install:
    pnpm install --frozen-lockfile
    mkdir -p target/deploy
    [ -f target/deploy/spl_noop.so ] || cp third-party/solana-program-library/spl_noop.so target/deploy/

# === Build ===
build: programs::build js::build cli::build

# === Test ===
test: program-tests::test sdk-tests::test js::test

# === Lint & Format ===
lint: lint-rust js::lint lint-dependencies lint-readmes lint-features

lint-rust:
    cargo +nightly fmt --all -- --check
    cargo clippy --workspace --all-features --all-targets -- -D warnings

lint-dependencies:
    ./scripts/check-dependency-constraints.sh

lint-readmes:
    #!/usr/bin/env bash
    set -e
    echo "Checking READMEs are up-to-date..."
    if ! command -v cargo-rdme &> /dev/null; then
        cargo install cargo-rdme
    fi
    for toml in $(find program-libs sdk-libs -name '.cargo-rdme.toml' -type f); do
        crate_dir=$(dirname "$toml")
        echo "Checking README in $crate_dir..."
        (cd "$crate_dir" && cargo rdme --check --no-fail-on-warnings)
    done

lint-features:
    #!/usr/bin/env bash
    set -e
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
        "light-sdk-macros"
        "light-token"
        "light-token-types"
        "light-sdk"
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

format:
    cargo +nightly fmt --all
    just js format

# === Clean ===
clean:
    find . -type d -name "test-ledger" -exec rm -rf {} + 2>/dev/null || true
    cargo clean

# === Info ===
info:
    @echo "Solana: $(solana --version)"
    @echo "Rust: $(rustc --version)"
    @echo "Node: $(node --version)"
