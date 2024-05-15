#!/bin/bash

# Configuration
CRATES_IO_TOKEN=${CRATES_IO_TOKEN}

# Ensure cargo, git, and gh are installed
command -v cargo >/dev/null 2>&1 || { echo >&2 "Cargo is not installed. Aborting."; exit 1; }
command -v git >/dev/null 2>&1 || { echo >&2 "Git is not installed. Aborting."; exit 1; }
command -v gh >/dev/null 2>&1 || { echo >&2 "GitHub CLI is not installed. Aborting."; exit 1; }
echo "Tagging and releasing all Rust projects..."

# Log in to crates.io
echo "Logging in to crates.io..."
cargo login "${CRATES_IO_TOKEN}"
# Combined tag and release process
PACKAGES=("light-bounded-vec" "light-hasher" "light-macros" "light-hash-set" "light-merkle-tree-reference" "light-concurrent-merkle-tree" "light-indexed-merkle-tree" "light-circuitlib-rs" "light-verifier" "account-compression" "light-registry" "light-system-program" "light-compressed-token" "light-test-utils")
for PACKAGE in "${PACKAGES[@]}"; do
    PKG_VERSION=$(cargo pkgid -p "$PACKAGE" | cut -d "#" -f2)
    VERSION=${PKG_VERSION#*@}
    echo "Creating tag for Rust package: $PACKAGE v$VERSION"
    git tag "${PACKAGE}-v${VERSION}"
    git push origin "${PACKAGE}-v${VERSION}"
    for attempt in {1..3}; do
        echo "Attempt $attempt: Publishing $PACKAGE..."
        cargo release publish --package "$PACKAGE" --execute --no-confirm && break || echo "Attempt $attempt failed, retrying in 60..."
        sleep 60
    done
    echo "Sleeping for 60 seconds to handle rate limits..."
    sleep 60
done

# # Create a GitHub release
# echo "Creating GitHub release..."
# TAG_NAME="release-$(date +%Y-%m-%d)"  # Customize your tag name
# RELEASE_TITLE="Release on $(date +%Y-%m-%d)"
# RELEASE_NOTES="Released all Rust packages"

# # Using GitHub CLI to create a release
# gh release create "$TAG_NAME" --title "$RELEASE_TITLE" --notes "$RELEASE_NOTES"

# echo "Release process completed."