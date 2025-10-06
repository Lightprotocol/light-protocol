#!/usr/bin/env bash
set -euo pipefail

# Validate or publish packages using cargo-release
# Usage:
#   ./scripts/validate-packages.sh [base-ref] [head-ref]           # Dry-run validation
#   ./scripts/validate-packages.sh --execute [base-ref] [head-ref] # Actual publish
# Arguments:
#   --execute: Actually publish to crates.io (default: dry-run only)
#   base-ref: Base reference to compare against (default: origin/main)
#   head-ref: Head reference to compare (default: HEAD)
# Exits with 0 on success, 1 on failure

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse --execute flag
EXECUTE_FLAG=""
if [ "${1:-}" = "--execute" ]; then
  EXECUTE_FLAG="--execute"
  shift
fi

BASE_REF="${1:-origin/main}"
HEAD_REF="${2:-HEAD}"

echo "Detecting packages with version changes..."
echo "Comparing: $BASE_REF...$HEAD_REF"
echo ""

# Detect packages using the detection script
PACKAGES_STRING=$("$SCRIPT_DIR/detect-version-changes.sh" "$BASE_REF" "$HEAD_REF")

# Convert to array
read -ra PACKAGES <<< "$PACKAGES_STRING"

if [ -n "$EXECUTE_FLAG" ]; then
  echo "Publishing packages to crates.io..."
else
  echo "Running dry-run validation for packages..."
fi
echo "Packages: ${PACKAGES[*]}"

# Build package args for cargo-release
PACKAGE_ARGS=""
for pkg in "${PACKAGES[@]}"; do
  PACKAGE_ARGS="$PACKAGE_ARGS -p $pkg"
done

echo ""
if [ -n "$EXECUTE_FLAG" ]; then
  echo "Running: cargo release publish $PACKAGE_ARGS --execute --no-confirm"
else
  echo "Running: cargo release publish $PACKAGE_ARGS --allow-branch '*' --no-verify"
fi
echo "----------------------------------------"

# cargo-release handles dependency ordering
# Without --execute: dry-run validation (allow any branch, skip git checks for uncommitted changes)
# With --execute: actual publish to crates.io (requires main or release/* branch and clean working tree)
if [ -n "$EXECUTE_FLAG" ]; then
  cargo release publish $PACKAGE_ARGS --execute --no-confirm
else
  cargo release publish $PACKAGE_ARGS --allow-branch '*' --no-verify
fi

if [ $? -eq 0 ]; then
  echo ""
  if [ -n "$EXECUTE_FLAG" ]; then
    echo "All crates published successfully"
  else
    echo "All crates validated successfully"
  fi
  exit 0
else
  echo ""
  if [ -n "$EXECUTE_FLAG" ]; then
    echo "Publishing failed"
  else
    echo "Validation failed"
  fi
  exit 1
fi
