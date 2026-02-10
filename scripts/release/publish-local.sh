#!/usr/bin/env bash
set -euo pipefail

# Local equivalent of the GitHub Actions release workflow
# Usage: ./scripts/release/publish-local.sh [base-ref]
# Arguments:
#   base-ref: Git ref to compare against (default: origin/main)
#
# Prerequisites:
#   - cargo login (credentials in ~/.cargo/credentials.toml)
#   - gh CLI authenticated for GitHub releases
#
# Phases:
#   1. Validation (dry-run) - cargo publish --dry-run
#   2. Publishing - cargo publish to crates.io
#   3. GitHub releases - create tags and releases

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_REF="${1:-origin/main}"
HEAD_REF="HEAD"

# Check prerequisites
if ! gh auth status &>/dev/null; then
    echo "WARNING: gh CLI not authenticated - GitHub releases will be skipped" >&2
    SKIP_RELEASES=true
else
    SKIP_RELEASES=false
fi

# Fetch latest from remote if comparing against origin
if [[ "$BASE_REF" == origin/* ]]; then
    BRANCH="${BASE_REF#origin/}"
    echo "Fetching origin/$BRANCH..."
    git fetch origin "$BRANCH"
fi

echo ""
echo "========================================="
echo "Phase 1: Validation (dry-run)"
echo "========================================="
echo "Comparing: $BASE_REF...$HEAD_REF"
echo ""

if ! "$SCRIPT_DIR/validate-packages.sh" "$BASE_REF" "$HEAD_REF"; then
    echo ""
    echo "Validation failed. Fix issues before publishing."
    exit 1
fi

echo ""
echo "========================================="
echo "Phase 2: Publishing to crates.io"
echo "========================================="
echo ""

read -p "Proceed with publishing to crates.io? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

if ! "$SCRIPT_DIR/validate-packages.sh" --execute "$BASE_REF" "$HEAD_REF"; then
    echo ""
    echo "Publishing failed."
    exit 1
fi

echo ""
echo "========================================="
echo "Phase 3: Creating GitHub releases"
echo "========================================="
echo ""

if [ "$SKIP_RELEASES" = true ]; then
    echo "Skipping GitHub releases (gh CLI not authenticated)"
    exit 0
fi

read -p "Create GitHub releases? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Skipping GitHub releases."
    exit 0
fi

# Detect packages that were published
PACKAGES_STRING=$("$SCRIPT_DIR/detect-version-changes.sh" "$BASE_REF" "$HEAD_REF")
read -ra PACKAGES <<< "$PACKAGES_STRING"

for pkg in "${PACKAGES[@]}"; do
    echo "----------------------------------------"
    # Get the version from Cargo.toml
    VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r ".packages[] | select(.name == \"$pkg\") | .version")
    TAG="${pkg}-v${VERSION}"

    echo "Creating GitHub release for $TAG..."

    # Generate crate-specific release notes
    if RELEASE_NOTES=$("$SCRIPT_DIR/generate-release-notes.sh" "$pkg" "$VERSION" 2>&1); then
        echo "Generated release notes for $pkg"

        # Create release with custom notes
        if echo "$RELEASE_NOTES" | gh release create "$TAG" --title "$TAG" --notes-file -; then
            echo "Created release for $TAG"
        else
            echo "Warning: Failed to create release for $TAG"
        fi
    else
        # If script fails (e.g., no previous tag), fall back to auto-generated notes
        echo "Warning: Could not generate crate-specific notes: $RELEASE_NOTES"
        echo "Falling back to auto-generated notes"
        if gh release create "$TAG" --generate-notes --title "$TAG"; then
            echo "Created release for $TAG"
        else
            echo "Warning: Failed to create release for $TAG"
        fi
    fi
done

echo ""
echo "========================================="
echo "Release complete!"
echo "========================================="
