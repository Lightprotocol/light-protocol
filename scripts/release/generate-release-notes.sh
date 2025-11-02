#!/usr/bin/env bash
set -euo pipefail

# Generate crate-specific release notes for GitHub releases
# Usage: ./scripts/release/generate-release-notes.sh <package-name> <version>
# Arguments:
#   package-name: Name of the package (e.g., "light-account-checks")
#   version: Current version being released (e.g., "0.5.1")
# Outputs: Release notes in markdown format to stdout

PACKAGE_NAME="${1:-}"
VERSION="${2:-}"

if [ -z "$PACKAGE_NAME" ] || [ -z "$VERSION" ]; then
  echo "Usage: $0 <package-name> <version>" >&2
  exit 1
fi

TAG="${PACKAGE_NAME}-v${VERSION}"

# Get package directory from cargo metadata
MANIFEST_PATH=$(cargo metadata --format-version 1 --no-deps | jq -r ".packages[] | select(.name == \"$PACKAGE_NAME\") | .manifest_path")
if [ -z "$MANIFEST_PATH" ] || [ "$MANIFEST_PATH" = "null" ]; then
  echo "Error: Package '$PACKAGE_NAME' not found in workspace" >&2
  exit 1
fi

PKG_DIR=$(dirname "$MANIFEST_PATH")
PKG_DIR_RELATIVE="${PKG_DIR#$PWD/}"

# Find the previous tag for this specific package
PREVIOUS_TAG=$(git tag --list "${PACKAGE_NAME}-v*" \
  | grep -v "^${TAG}$" \
  | sort -V -r \
  | head -1)

if [ -z "$PREVIOUS_TAG" ]; then
  echo "Error: No previous tag found for $PACKAGE_NAME" >&2
  echo "This script requires at least one previous release tag." >&2
  exit 1
fi

# Get commits that touched this package's directory
COMMITS=$(git log --format="%H" "${PREVIOUS_TAG}..HEAD" -- "$PKG_DIR_RELATIVE")

if [ -z "$COMMITS" ]; then
  echo "No changes detected for this package."
  exit 0
fi

# Build release notes with PRs that touched this package
echo "## What's Changed"
echo ""

SEEN_PRS=()
FOUND_PRS=false

for commit in $COMMITS; do
  # Get PR number from commit message (format: "title (#123)")
  PR_NUM=$(git log --format=%s -n 1 "$commit" | grep -oE '\(#[0-9]+\)' | grep -oE '[0-9]+' | head -1)

  if [ -n "$PR_NUM" ]; then
    # Check if we've already seen this PR
    if [[ ! " ${SEEN_PRS[@]:-} " =~ " ${PR_NUM} " ]]; then
      SEEN_PRS+=("$PR_NUM")
      FOUND_PRS=true

      # Get PR details using gh CLI
      if PR_TITLE=$(gh pr view "$PR_NUM" --json title --jq '.title' 2>/dev/null); then
        PR_AUTHOR=$(gh pr view "$PR_NUM" --json author --jq '.author.login' 2>/dev/null || echo "unknown")
        echo "* ${PR_TITLE} by @${PR_AUTHOR} in #${PR_NUM}"
      else
        # Fallback if gh CLI fails
        COMMIT_TITLE=$(git log --format=%s -n 1 "$commit" | sed 's/ (#[0-9]*)//')
        echo "* ${COMMIT_TITLE} in #${PR_NUM}"
      fi
    fi
  fi
done

if [ "$FOUND_PRS" = false ]; then
  echo "* Changes in commits between $PREVIOUS_TAG and $TAG"
fi

# Get repository URL and construct changelog link
REPO_URL=$(git remote get-url origin | sed 's/git@github.com:/https:\/\/github.com\//' | sed 's/\.git$//')

echo ""
echo "**Full Changelog**: ${REPO_URL}/compare/${PREVIOUS_TAG}...${TAG}"
