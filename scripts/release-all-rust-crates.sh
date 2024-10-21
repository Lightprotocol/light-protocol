#!/usr/bin/env bash

# Configuration
CRATES_IO_TOKEN=${CRATES_IO_TOKEN}

# Ensure cargo, git, and gh are installed
command -v cargo >/dev/null 2>&1 || { echo >&2 "Cargo is not installed. Aborting."; exit 1; }
command -v git >/dev/null 2>&1 || { echo >&2 "Git is not installed. Aborting."; exit 1; }
command -v gh >/dev/null 2>&1 || { echo >&2 "GitHub CLI is not installed. Aborting."; exit 1; }

# Parse command line arguments
RELEASE_PROGRAMS=false
RELEASE_SDKS=false

while [[ "$#" -gt 0 ]]; do
    case $1 in
        --programs) RELEASE_PROGRAMS=true ;;
        --sdks) RELEASE_SDKS=true ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

if [ "$RELEASE_PROGRAMS" = false ] && [ "$RELEASE_SDKS" = false ]; then
    echo "Please specify --programs or --sdks (or both)"
    exit 1
fi

echo "Logging in to crates.io..."
cargo login "${CRATES_IO_TOKEN}"

PROGRAMS=("aligned-sized" "light-heap" "light-bounded-vec" "light-utils" "light-hasher" "light-macros" "light-hash-set" "light-merkle-tree-reference" "light-concurrent-merkle-tree" "light-indexed-merkle-tree" "light-prover-client" "light-verifier" "account-compression" "light-system-program" "light-registry" "light-compressed-token")
SDKS=("photon-api" "forester-utils" "light-test-utils" "light-sdk-macros" "light-sdk")

release_packages() {
    local packages=("$@")
    for PACKAGE in "${packages[@]}"; do
        PKG_VERSION=$(cargo pkgid -p "$PACKAGE" | cut -d "#" -f2)
        VERSION=${PKG_VERSION#*@}
        echo "Creating tag for Rust package: $PACKAGE v$VERSION"
        git tag "${PACKAGE}-v${VERSION}"
        git push origin "${PACKAGE}-v${VERSION}"
        for attempt in {1..2}; do
            echo "Attempt $attempt: Publishing $PACKAGE..."
            cargo release publish --package "$PACKAGE" --execute --no-confirm && break || echo "Attempt $attempt failed, retrying in 10..."
            sleep 10
        done
    done
}

if [ "$RELEASE_PROGRAMS" = true ]; then
    echo "Releasing programs..."
    release_packages "${PROGRAMS[@]}"
fi

if [ "$RELEASE_SDKS" = true ]; then
    echo "Releasing SDKs..."
    release_packages "${SDKS[@]}"
fi