#!/usr/bin/env bash
set -euo pipefail

# Validate or publish packages using cargo-release
# Usage:
#   ./scripts/release/validate-packages.sh [base-ref] [head-ref]           # Dry-run validation
#   ./scripts/release/validate-packages.sh --execute [base-ref] [head-ref] # Actual publish
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

# Check that all dependents are included in the release
echo ""
echo "Checking that all dependents are included..."
if ! "$SCRIPT_DIR/check-dependents.sh" --base-ref "$BASE_REF" "${PACKAGES[@]}"; then
  echo "ERROR: Dependent packages are missing from the release" >&2
  exit 1
fi
echo ""

# Function to check if package is new (no previous release tag)
is_new_package() {
  local pkg=$1
  local tag=$(git tag -l "${pkg}-v*" 2>/dev/null | head -1)
  [ -z "$tag" ]
}

# Build package args, excluding new packages for dry-run
PACKAGE_ARGS=""
NEW_PACKAGES=""
EXISTING_PACKAGES=""
for pkg in "${PACKAGES[@]}"; do
  if is_new_package "$pkg"; then
    NEW_PACKAGES="$NEW_PACKAGES $pkg"
  else
    EXISTING_PACKAGES="$EXISTING_PACKAGES $pkg"
    PACKAGE_ARGS="$PACKAGE_ARGS -p $pkg"
  fi
done

if [ -n "$NEW_PACKAGES" ]; then
  echo "New packages (skipped in dry-run):$NEW_PACKAGES"
  echo ""
fi

echo ""
if [ -n "$EXECUTE_FLAG" ]; then
  # For actual publish, include all packages
  PACKAGE_ARGS=""
  for pkg in "${PACKAGES[@]}"; do
    PACKAGE_ARGS="$PACKAGE_ARGS -p $pkg"
  done
  echo "Running: cargo check (all packages) then cargo publish $PACKAGE_ARGS --no-verify"
else
  if [ -z "$PACKAGE_ARGS" ]; then
    echo "All packages are new - skipping dry-run validation"
    echo "Compilation check will still run"
  else
    echo "Running: cargo check (all packages) then cargo publish $PACKAGE_ARGS --dry-run --allow-dirty --no-verify"
  fi
fi
echo "----------------------------------------"

# Native cargo 1.90.0+ handles dependency ordering for interdependent workspace crates

# First: Always run compilation check to catch errors
echo ""
echo "Running compilation check..."
for pkg in "${PACKAGES[@]}"; do
  echo "  Checking $pkg..."
  if ! cargo test -p "$pkg" --all-features --no-run 2>&1 | tail -20; then
    echo "Error: Compilation check failed for $pkg"
    exit 1
  fi
done
echo "All packages compile successfully"
echo ""

# Function to check if packages have interdependencies
has_interdependencies() {
  local packages=("$@")
  for pkg in "${packages[@]}"; do
    # Find Cargo.toml for this package
    local cargo_toml=$(find program-libs sdk-libs prover/client sparse-merkle-tree -name "Cargo.toml" -exec grep -l "^name = \"$pkg\"" {} \; 2>/dev/null | head -1)
    if [ -z "$cargo_toml" ]; then
      continue
    fi

    # Check if this package depends on any other package in the release
    for dep_pkg in "${packages[@]}"; do
      if [ "$pkg" != "$dep_pkg" ]; then
        if grep -q "^$dep_pkg *= *{" "$cargo_toml" 2>/dev/null; then
          echo "Detected interdependency: $pkg depends on $dep_pkg"
          return 0
        fi
      fi
    done
  done
  return 1
}

# Then: Either publish or dry-run
if [ -n "$EXECUTE_FLAG" ]; then
  # Publish with --no-verify to avoid cargo bug with unpublished deps
  cargo publish $PACKAGE_ARGS --no-verify
else
  # Check for interdependencies
  if has_interdependencies "${PACKAGES[@]}"; then
    echo "Skipping cargo publish dry-run (interdependent packages detected)"
    echo "The compilation check above already validated the packages"
  elif [ -z "$(echo $PACKAGE_ARGS | tr -d ' ')" ]; then
    echo "Skipping cargo publish dry-run (all packages are new)"
  else
    # Dry-run validation - allow dirty state and skip verification
    cargo publish $PACKAGE_ARGS --dry-run --allow-dirty --no-verify
  fi
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
