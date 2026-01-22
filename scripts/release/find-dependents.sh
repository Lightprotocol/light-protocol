#!/usr/bin/env bash
set -euo pipefail

# Find all packages that need version bumps when releasing a given package
# Usage: ./scripts/release/find-dependents.sh <package-name> [--all]
# Arguments:
#   package-name: The package to find dependents for
#   --all: Include all dependents, even if unchanged since last release
# Outputs: List of packages that need to be bumped
#
# This script checks BOTH directions:
# 1. Dependencies: packages that the input package depends on (must be released first)
# 2. Dependents: packages that depend on the input package (must be released after)

if [ $# -lt 1 ]; then
    echo "Usage: $0 <package-name> [--all]" >&2
    echo "" >&2
    echo "Find all packages that need version bumps when releasing the given package." >&2
    echo "By default, only shows packages with changes since their last release." >&2
    echo "" >&2
    echo "Checks both directions:" >&2
    echo "  - Dependencies (packages it depends on) - must be released first" >&2
    echo "  - Dependents (packages that depend on it) - must be released after" >&2
    echo "" >&2
    echo "Options:" >&2
    echo "  --all    Include all packages, even if unchanged since last release" >&2
    exit 1
fi

ROOT_PACKAGE=$1
SHOW_ALL=false
if [ "${2:-}" = "--all" ]; then
    SHOW_ALL=true
fi

# Scan all lib directories
SCAN_DIRS="program-libs sdk-libs prover/client sparse-merkle-tree"

# Create temp files
DEPS_FILE=$(mktemp)
ALL_PKGS_FILE=$(mktemp)
VISITED_FILE=$(mktemp)
PKG_PATHS_FILE=$(mktemp)
trap "rm -f $DEPS_FILE $ALL_PKGS_FILE $VISITED_FILE $PKG_PATHS_FILE" EXIT

# Find all Cargo.toml files and build package path map
CARGO_FILES=$(find $SCAN_DIRS -name "Cargo.toml" 2>/dev/null)

# Build dependency map and package paths
for cargo_toml in $CARGO_FILES; do
  # Get the package name from this Cargo.toml
  pkg_name=$(grep '^name = ' "$cargo_toml" 2>/dev/null | head -1 | sed 's/name = "\([^"]*\)".*/\1/')

  if [ -z "$pkg_name" ]; then
    continue
  fi

  # Store package path (directory containing Cargo.toml)
  pkg_dir=$(dirname "$cargo_toml")
  echo "$pkg_name:$pkg_dir" >> "$PKG_PATHS_FILE"

  # Find all light-* dependencies in this Cargo.toml
  deps=$(grep -E '^light-[a-zA-Z0-9_-]+ *= *\{' "$cargo_toml" 2>/dev/null || true)

  if [ -n "$deps" ]; then
    echo "$deps" | while read -r line; do
      # Extract dependency name
      dep_name=$(echo "$line" | sed 's/^\([a-zA-Z0-9_-]*\).*/\1/')

      if [ -n "$dep_name" ] && [ "$dep_name" != "$pkg_name" ]; then
        # Format: dependent:dependency (dependent depends on dependency)
        echo "$pkg_name:$dep_name"
      fi
    done >> "$DEPS_FILE"
  fi
done

# Check if the root package exists
pkg_exists=false
for cargo_toml in $CARGO_FILES; do
  pkg_name=$(grep '^name = ' "$cargo_toml" 2>/dev/null | head -1 | sed 's/name = "\([^"]*\)".*/\1/')
  if [ "$pkg_name" = "$ROOT_PACKAGE" ]; then
    pkg_exists=true
    break
  fi
done

if [ "$pkg_exists" = false ]; then
  echo "Error: Package '$ROOT_PACKAGE' not found in: $SCAN_DIRS" >&2
  exit 1
fi

# Function to get the last release tag for a package
get_last_release_tag() {
  local pkg=$1
  # Tags are in format: package-name-vX.Y.Z
  git tag -l "${pkg}-v*" 2>/dev/null | sort -V | tail -1
}

# Function to get package directory
get_pkg_dir() {
  local pkg=$1
  grep "^${pkg}:" "$PKG_PATHS_FILE" 2>/dev/null | head -1 | cut -d: -f2
}

# Function to check if package has changes since last release
has_changes_since_release() {
  local pkg=$1
  local pkg_dir=$(get_pkg_dir "$pkg")

  if [ -z "$pkg_dir" ]; then
    # Can't find package dir, assume it has changes
    return 0
  fi

  local last_tag=$(get_last_release_tag "$pkg")

  if [ -z "$last_tag" ]; then
    # No previous release, needs to be released
    echo "new"
    return 0
  fi

  # Check if there are any commits affecting this package since the last tag
  local changes=$(git log "${last_tag}..HEAD" --oneline -- "$pkg_dir" 2>/dev/null | head -1)

  if [ -n "$changes" ]; then
    echo "changed"
    return 0
  else
    echo "unchanged"
    return 1
  fi
}

# Recursive function to find all dependents (packages that depend on pkg)
find_dependents() {
  local pkg=$1

  # Skip if already visited
  if grep -q "^dependents:${pkg}$" "$VISITED_FILE" 2>/dev/null; then
    return
  fi
  echo "dependents:$pkg" >> "$VISITED_FILE"

  # Find direct dependents
  if [ -s "$DEPS_FILE" ]; then
    local dependents=$(grep ":${pkg}$" "$DEPS_FILE" 2>/dev/null | cut -d: -f1 | sort -u || true)

    for dependent in $dependents; do
      echo "$dependent" >> "$ALL_PKGS_FILE"
      find_dependents "$dependent"
    done
  fi
}

# Recursive function to find all dependencies (packages that pkg depends on)
find_dependencies() {
  local pkg=$1

  # Skip if already visited
  if grep -q "^dependencies:${pkg}$" "$VISITED_FILE" 2>/dev/null; then
    return
  fi
  echo "dependencies:$pkg" >> "$VISITED_FILE"

  # Find direct dependencies
  if [ -s "$DEPS_FILE" ]; then
    local dependencies=$(grep "^${pkg}:" "$DEPS_FILE" 2>/dev/null | cut -d: -f2 | sort -u || true)

    for dependency in $dependencies; do
      echo "$dependency" >> "$ALL_PKGS_FILE"
      find_dependencies "$dependency"
    done
  fi
}

# Add the root package
echo "$ROOT_PACKAGE" >> "$ALL_PKGS_FILE"

# Find both directions
find_dependencies "$ROOT_PACKAGE"
find_dependents "$ROOT_PACKAGE"

# Get unique list of all packages
ALL_PACKAGES=$(sort -u "$ALL_PKGS_FILE")

# Analyze and categorize packages
echo "Analyzing packages for changes since last release..."
echo ""

CHANGED_DEPS=""
UNCHANGED_DEPS=""
NEW_DEPS=""
CHANGED_DEPENDENTS=""
UNCHANGED_DEPENDENTS=""
NEW_DEPENDENTS=""
ROOT_STATUS=""

# Get dependencies and dependents lists for categorization
DEPENDENCIES=$(grep "^${ROOT_PACKAGE}:" "$DEPS_FILE" 2>/dev/null | cut -d: -f2 | sort -u || true)
# Recursively get all dependencies
ALL_DEPENDENCIES=""
for dep in $DEPENDENCIES; do
  ALL_DEPENDENCIES="$ALL_DEPENDENCIES $dep"
  # Add transitive dependencies
  transitive=$(grep "^${dep}:" "$DEPS_FILE" 2>/dev/null | cut -d: -f2 | sort -u || true)
  ALL_DEPENDENCIES="$ALL_DEPENDENCIES $transitive"
done
ALL_DEPENDENCIES=$(echo $ALL_DEPENDENCIES | tr ' ' '\n' | sort -u | tr '\n' ' ')

for pkg in $ALL_PACKAGES; do
  status=$(has_changes_since_release "$pkg" || true)
  last_tag=$(get_last_release_tag "$pkg")

  # Determine if this is the root, a dependency, or a dependent
  if [ "$pkg" = "$ROOT_PACKAGE" ]; then
    category="root"
    ROOT_STATUS="$status"
    ROOT_TAG="$last_tag"
  elif echo "$ALL_DEPENDENCIES" | grep -qw "$pkg"; then
    category="dependency"
  else
    category="dependent"
  fi

  if [ "$status" = "new" ]; then
    if [ "$category" = "dependency" ]; then
      NEW_DEPS="$NEW_DEPS $pkg"
    elif [ "$category" = "dependent" ]; then
      NEW_DEPENDENTS="$NEW_DEPENDENTS $pkg"
    fi
    if [ "$category" != "root" ]; then
      echo "  [NEW]     $pkg (no previous release) [$category]"
    fi
  elif [ "$status" = "changed" ]; then
    if [ "$category" = "dependency" ]; then
      CHANGED_DEPS="$CHANGED_DEPS $pkg"
    elif [ "$category" = "dependent" ]; then
      CHANGED_DEPENDENTS="$CHANGED_DEPENDENTS $pkg"
    fi
    if [ "$category" != "root" ]; then
      echo "  [CHANGED] $pkg (since $last_tag) [$category]"
    fi
  else
    if [ "$category" = "dependency" ]; then
      UNCHANGED_DEPS="$UNCHANGED_DEPS $pkg"
    elif [ "$category" = "dependent" ]; then
      UNCHANGED_DEPENDENTS="$UNCHANGED_DEPENDENTS $pkg"
    fi
    if [ "$SHOW_ALL" = true ] && [ "$category" != "root" ]; then
      echo "  [--]      $pkg (unchanged since $last_tag) [$category]"
    fi
  fi
done

echo ""
echo "========================================"
echo "Summary for releasing '$ROOT_PACKAGE':"
echo "========================================"
echo ""

# Show root package status
if [ "$ROOT_STATUS" = "new" ]; then
  echo "Target package: $ROOT_PACKAGE [NEW - no previous release]"
elif [ "$ROOT_STATUS" = "changed" ]; then
  echo "Target package: $ROOT_PACKAGE [CHANGED since $ROOT_TAG]"
else
  echo "Target package: $ROOT_PACKAGE [UNCHANGED since $ROOT_TAG]"
fi
echo ""

# Show dependencies that need bumps
DEPS_NEED_BUMP="$NEW_DEPS $CHANGED_DEPS"
DEPS_NEED_BUMP=$(echo $DEPS_NEED_BUMP | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ' || true)

if [ -n "$(echo $DEPS_NEED_BUMP | tr -d ' ')" ]; then
  DEPS_COUNT=$(echo $DEPS_NEED_BUMP | wc -w | tr -d ' ')
  echo "DEPENDENCIES that need release FIRST ($DEPS_COUNT):"
  for pkg in $DEPS_NEED_BUMP; do
    echo "  $pkg"
  done
  echo ""
fi

# Show dependents that need bumps
DEPENDENTS_NEED_BUMP="$NEW_DEPENDENTS $CHANGED_DEPENDENTS"
DEPENDENTS_NEED_BUMP=$(echo $DEPENDENTS_NEED_BUMP | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ' || true)

if [ -n "$(echo $DEPENDENTS_NEED_BUMP | tr -d ' ')" ]; then
  DEPENDENTS_COUNT=$(echo $DEPENDENTS_NEED_BUMP | wc -w | tr -d ' ')
  echo "DEPENDENTS that need release AFTER ($DEPENDENTS_COUNT):"
  for pkg in $DEPENDENTS_NEED_BUMP; do
    echo "  $pkg"
  done
  echo ""
fi

# Combined list for release
ALL_NEED_BUMP="$DEPS_NEED_BUMP $ROOT_PACKAGE $DEPENDENTS_NEED_BUMP"
ALL_NEED_BUMP=$(echo $ALL_NEED_BUMP | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ' || true)

if [ -n "$ALL_NEED_BUMP" ]; then
  TOTAL_COUNT=$(echo $ALL_NEED_BUMP | wc -w | tr -d ' ')
  echo "----------------------------------------"
  echo "ALL packages to bump ($TOTAL_COUNT total):"
  echo "$ALL_NEED_BUMP"
  echo ""
fi

# Show unchanged counts
UNCHANGED_DEPS_COUNT=$(echo "$UNCHANGED_DEPS" | wc -w | tr -d ' ')
UNCHANGED_DEPENDENTS_COUNT=$(echo "$UNCHANGED_DEPENDENTS" | wc -w | tr -d ' ')
TOTAL_UNCHANGED=$((UNCHANGED_DEPS_COUNT + UNCHANGED_DEPENDENTS_COUNT))

if [ "$TOTAL_UNCHANGED" -gt 0 ] && [ "$SHOW_ALL" = false ]; then
  echo "($TOTAL_UNCHANGED packages unchanged since last release - use --all to see)"
fi

exit 0
