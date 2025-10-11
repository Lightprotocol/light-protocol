#!/usr/bin/env bash
set -euo pipefail

# Detect packages with version changes between two git refs
# Usage: ./scripts/detect-version-changes.sh [base-ref] [head-ref]
# Arguments:
#   base-ref: Base reference to compare against (default: origin/main)
#   head-ref: Head reference to compare (default: HEAD)
# Outputs: Space-separated list of package names to stdout

BASE_REF="${1:-origin/main}"
HEAD_REF="${2:-HEAD}"

# Fetch if comparing against remote refs
if [[ "$BASE_REF" == origin/* ]]; then
  BRANCH="${BASE_REF#origin/}"
  git fetch origin "$BRANCH"
fi

# Extract packages with version changes
PACKAGES=()

# Set up diff arguments based on HEAD_REF
# If HEAD_REF is "HEAD", compare against working tree (includes uncommitted changes)
# Otherwise use three-dot diff for commits
if [ "$HEAD_REF" = "HEAD" ]; then
  DIFF_ARGS=("$BASE_REF")
else
  DIFF_ARGS=("$BASE_REF...$HEAD_REF")
fi

# Get list of changed Cargo.toml files in program-libs, sdk-libs, program-tests/merkle-tree, sparse-merkle-tree, and prover
for file in $(git diff "${DIFF_ARGS[@]}" --name-only -- '**/Cargo.toml' | grep -E '(program-libs|sdk-libs|program-tests/merkle-tree|sparse-merkle-tree|prover)/'); do
  # Extract old and new version from the diff
  versions=$(git diff "${DIFF_ARGS[@]}" -- "$file" | grep -E '^\+version|^-version' | grep -v '+++\|---')
  old_ver=$(echo "$versions" | grep '^-version' | head -1 | awk -F'"' '{print $2}')
  new_ver=$(echo "$versions" | grep '^\+version' | head -1 | awk -F'"' '{print $2}')

  # Only process if version actually changed
  if [ -n "$old_ver" ] && [ -n "$new_ver" ] && [ "$old_ver" != "$new_ver" ]; then
    # Extract actual package name from Cargo.toml
    pkg_name=$(grep '^name = ' "$file" | head -1 | awk -F'"' '{print $2}')

    if [ -n "$pkg_name" ]; then
      PACKAGES+=("$pkg_name")
    fi
  fi
done

if [ ${#PACKAGES[@]} -eq 0 ]; then
  echo "No packages with version changes detected" >&2
  exit 1
fi

# Output space-separated list to stdout
echo "${PACKAGES[*]}"
