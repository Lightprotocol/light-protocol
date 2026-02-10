#!/usr/bin/env bash
set -euo pipefail

# Check that all dependents of released packages are also being released
# Usage: ./scripts/release/check-dependents.sh [--base-ref <ref>] <packages...>
# Patch version bumps are semver-compatible and skip the dependents check.

BASE_REF="origin/main"
if [ "${1:-}" = "--base-ref" ]; then
    BASE_REF="$2"
    shift 2
fi

if [ $# -lt 1 ]; then
    echo "Usage: $0 [--base-ref <ref>] <package1> [package2] ..." >&2
    exit 1
fi

PACKAGES=("$@")

# Scan directories
SCAN_DIRS="program-libs sdk-libs prover/client sparse-merkle-tree"

# Check if version bump is patch-only (0.x.y -> 0.x.z or 1.x.y -> 1.x.z)
is_patch_only() {
    local old_ver="$1" new_ver="$2"
    local old_major old_minor new_major new_minor
    IFS='.' read -r old_major old_minor _ <<< "$old_ver"
    IFS='.' read -r new_major new_minor _ <<< "$new_ver"
    [ "$old_major" = "$new_major" ] && { [ "$old_major" != "0" ] || [ "$old_minor" = "$new_minor" ]; }
}

# Filter to packages with breaking changes only
BREAKING_PACKAGES=()
for pkg in "${PACKAGES[@]}"; do
    cargo_toml=$(find $SCAN_DIRS -name "Cargo.toml" -exec grep -l "^name = \"$pkg\"" {} \; 2>/dev/null | head -1)
    if [ -n "$cargo_toml" ]; then
        versions=$(git diff "$BASE_REF" -- "$cargo_toml" 2>/dev/null | grep -E '^\+version|^-version' | grep -v '+++\|---' || true)
        old_ver=$(echo "$versions" | grep '^-version' | head -1 | awk -F'"' '{print $2}')
        new_ver=$(echo "$versions" | grep '^\+version' | head -1 | awk -F'"' '{print $2}')
        if [ -n "$old_ver" ] && [ -n "$new_ver" ] && is_patch_only "$old_ver" "$new_ver"; then
            continue
        fi
    fi
    BREAKING_PACKAGES+=("$pkg")
done

if [ ${#BREAKING_PACKAGES[@]} -eq 0 ]; then
    echo "All changes are patch-only - dependents check skipped"
    exit 0
fi

# Packages to exclude from dependent checks (e.g., not published to crates.io)
EXCLUDED_PACKAGES=""

# Scan all lib directories
SCAN_DIRS="program-libs sdk-libs prover/client sparse-merkle-tree"

# Create temp files for tracking
DEPS_FILE=$(mktemp)
RELEASING_FILE=$(mktemp)
MISSING_FILE=$(mktemp)
trap "rm -f $DEPS_FILE $RELEASING_FILE $MISSING_FILE" EXIT

# Store releasing packages
for pkg in "${PACKAGES[@]}"; do
  echo "$pkg" >> "$RELEASING_FILE"
done

# Find all Cargo.toml files first
CARGO_FILES=$(find $SCAN_DIRS -name "Cargo.toml" 2>/dev/null)

# Build dependency map: dependent_pkg:depends_on_pkg
for cargo_toml in $CARGO_FILES; do
  # Get the package name from this Cargo.toml
  pkg_name=$(grep '^name = ' "$cargo_toml" 2>/dev/null | head -1 | sed 's/name = "\([^"]*\)".*/\1/')

  if [ -z "$pkg_name" ]; then
    continue
  fi

  # Find all light-* dependencies in this Cargo.toml
  deps=$(grep -E '^light-[a-zA-Z0-9_-]+ *= *\{' "$cargo_toml" 2>/dev/null || true)

  if [ -n "$deps" ]; then
    echo "$deps" | while read -r line; do
      # Extract dependency name
      dep_name=$(echo "$line" | sed 's/^\([a-zA-Z0-9_-]*\).*/\1/')

      if [ -n "$dep_name" ] && [ "$dep_name" != "$pkg_name" ]; then
        echo "$pkg_name:$dep_name"
      fi
    done >> "$DEPS_FILE"
  fi
done

# Check each package with breaking changes for missing dependents
for pkg in "${BREAKING_PACKAGES[@]}"; do
  # Find all packages that depend on this package
  if [ -s "$DEPS_FILE" ]; then
    dependents=$(grep ":${pkg}$" "$DEPS_FILE" 2>/dev/null | cut -d: -f1 | sort -u || true)

    for dependent in $dependents; do
      # Skip excluded packages
      if echo "$EXCLUDED_PACKAGES" | grep -qw "$dependent"; then
        continue
      fi
      # Check if dependent is in the release list
      if ! grep -q "^${dependent}$" "$RELEASING_FILE"; then
        # Check if dependent is in the scan dirs (it should be since we found it)
        echo "$dependent (depends on $pkg)" >> "$MISSING_FILE"
      fi
    done
  fi
done

# Deduplicate missing file
if [ -s "$MISSING_FILE" ]; then
  sort -u "$MISSING_FILE" > "${MISSING_FILE}.sorted"
  mv "${MISSING_FILE}.sorted" "$MISSING_FILE"

  echo "ERROR: The following packages depend on released packages but are not being released:" >&2
  echo "" >&2
  while read -r line; do
    echo "  - $line" >&2
  done < "$MISSING_FILE"
  echo "" >&2
  echo "Missing packages: $(cut -d' ' -f1 "$MISSING_FILE" | tr '\n' ' ')" >&2
  echo "" >&2
  echo "To fix: bump versions of these packages and include them in the release." >&2
  exit 1
fi

echo "All dependents check passed - no missing packages"
exit 0
