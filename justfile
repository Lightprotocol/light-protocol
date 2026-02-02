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
lint: lint-rust lint-readmes js::lint

lint-rust:
    cargo +nightly fmt --all -- --check
    cargo clippy --workspace --all-features --all-targets -- -D warnings

# Check READMEs are up-to-date with cargo-rdme
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
