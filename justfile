# Light Protocol Monorepo
set dotenv-load

export SBF_OUT_DIR := "target/deploy"
export REDIS_URL := env_var_or_default("REDIS_URL", "redis://localhost:6379")
export CARGO_FEATURES := env_var_or_default("CARGO_FEATURES", "v2_ix")

default:
    @just --list

# === Setup ===
install:
    pnpm install --frozen-lockfile
    mkdir -p target/deploy
    [ -f target/deploy/spl_noop.so ] || cp third-party/solana-program-library/spl_noop.so target/deploy/

# === Build ===
build: build-programs build-js build-cli

build-programs:
    cd programs/system && cargo build-sbf
    cd programs/account-compression && cargo build-sbf --features 'test, migrate-state'
    cd programs/registry && cargo build-sbf
    cd programs/compressed-token/program && cargo build-sbf

build-program-tests:
    cd program-tests/create-address-test-program && cargo build-sbf

build-js:
    cd js/stateless.js && pnpm build
    cd js/compressed-token && pnpm build

build-cli: build-js
    cd cli && pnpm build

build-forester:
    cargo build -p forester --release

test-forester:
    TEST_MODE=local \
    TEST_V1_STATE=true \
    TEST_V2_STATE=true \
    TEST_V1_ADDRESS=true \
    TEST_V2_ADDRESS=true \
    cargo test --package forester e2e_test -- --nocapture

build-csdk-anchor-full-derived-test:
    cargo build-sbf --manifest-path sdk-tests/csdk-anchor-full-derived-test/Cargo.toml

test-forester-compressible-pda:
    RUST_LOG=forester=debug,light_client=debug \
    cargo test --package forester --test test_compressible_pda -- --nocapture

test-forester-compressible-mint:
    RUST_LOG=forester=debug,light_client=debug \
    cargo test --package forester --test test_compressible_mint -- --nocapture

test-forester-compressible-ctoken:
    RUST_LOG=forester=debug,light_client=debug \
    cargo test --package forester --test test_compressible_ctoken -- --nocapture

# === Test ===
test: test-programs test-sdk test-js

test-programs:
    RUSTFLAGS="-D warnings" cargo test-sbf -p account-compression-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p registry-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p system-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p system-cpi-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p compressed-token-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p e2e-test

test-sdk:
    RUSTFLAGS="-D warnings" cargo test-sbf -p sdk-native-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p sdk-anchor-test
    RUSTFLAGS="-D warnings" cargo test-sbf -p sdk-token-test

# Program libs tests (matching CI groups in rust.yml)
test-program-libs: test-program-libs-fast test-program-libs-slow

test-program-libs-fast:
    cargo test -p light-macros
    cargo test -p aligned-sized
    cargo test -p light-array-map --all-features
    cargo test -p light-hasher --all-features
    cargo test -p light-compressed-account --all-features
    cargo test -p light-account-checks --all-features
    cargo test -p light-verifier --all-features
    cargo test -p light-merkle-tree-metadata --all-features
    cargo test -p light-zero-copy --features "std, mut, derive"
    cargo test -p light-zero-copy-derive --all-features
    cargo test -p zero-copy-derive-test
    cargo test -p light-hash-set --all-features
    cargo test -p batched-merkle-tree-test -- --skip test_simulate_transactions --skip test_e2e
    cargo test -p light-concurrent-merkle-tree
    cargo test -p light-token-interface --features poseidon
    cargo test -p light-compressible --all-features

test-program-libs-slow:
    cargo test -p light-bloom-filter --all-features
    cargo test -p light-indexed-merkle-tree --all-features
    cargo test -p batched-merkle-tree-test -- --test test_e2e

test-batched-merkle-tree-simulate:
    cargo test -p light-batched-merkle-tree --features test-only
    RUST_LOG=light_prover_client=debug cargo test -p batched-merkle-tree-test -- --test test_simulate_transactions

# SDK libs tests (matching CI in sdk-tests.yml)
test-sdk-libs:
    cargo test -p light-sdk-macros
    cargo test -p light-sdk-macros --all-features
    cargo test -p light-sdk
    cargo test -p light-sdk --all-features
    cargo test -p light-program-test
    cargo test -p light-client
    cargo test -p light-sparse-merkle-tree
    cargo test -p light-token-types
    cargo test -p light-token-sdk --all-features

test-js: test-stateless-js test-compressed-token

test-stateless-js:
    cd js/stateless.js && pnpm test

test-compressed-token:
    cd js/compressed-token && pnpm test

test-compressed-token-unit-v2:
    cd js/compressed-token && pnpm test:unit:all:v2

test-cli:
    cd cli && pnpm test

build-sdk-anchor-test:
    cd sdk-tests/sdk-anchor-test && pnpm build

# === Lint & Format ===
lint: lint-rust lint-js

lint-rust:
    cargo +nightly fmt --all -- --check
    cargo clippy --workspace --all-features -- -D warnings

lint-js:
    cd js/stateless.js && pnpm lint
    cd js/compressed-token && pnpm lint

format:
    cargo +nightly fmt --all
    cd js/stateless.js && pnpm format
    cd js/compressed-token && pnpm format

# === Clean ===
clean:
    find . -type d -name "test-ledger" -exec rm -rf {} + 2>/dev/null || true
    cargo clean

# === Info ===
info:
    @echo "Solana: $(solana --version)"
    @echo "Rust: $(rustc --version)"
    @echo "Node: $(node --version)"
