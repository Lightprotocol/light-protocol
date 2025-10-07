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
  echo "Running: cargo publish $PACKAGE_ARGS --dry-run"
fi
echo "----------------------------------------"

# Native cargo 1.90.0+ handles dependency ordering for interdependent workspace crates
# Without --execute: dry-run validation using native cargo publish
# With --execute: use cargo-release for actual publish to crates.io
if [ -n "$EXECUTE_FLAG" ]; then
  cargo release publish $PACKAGE_ARGS --execute --no-confirm
else
  # Use native cargo publish --dry-run with all packages at once
  # This allows interdependent workspace crates to validate together
  # Allow dirty state and skip verification due to cargo bug with unpublished dep hashes
  cargo publish $PACKAGE_ARGS --dry-run --allow-dirty --no-verify
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
