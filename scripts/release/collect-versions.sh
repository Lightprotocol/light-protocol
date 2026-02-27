#!/usr/bin/env bash
set -euo pipefail

# Collect current versions of all Light Protocol crates from workspace metadata.
# Usage:
#   ./scripts/release/collect-versions.sh                # Print to stdout
#   ./scripts/release/collect-versions.sh > versions.env # Save to file
#   source <(./scripts/release/collect-versions.sh)      # Source into current shell
# Output format (one per line):
#   <crate-name>=<version>
# Also outputs LIGHT_SDK_VERSION=<version> as the primary version identifier.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v cargo &>/dev/null; then
  echo "Error: cargo not found in PATH" >&2
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq not found in PATH" >&2
  exit 1
fi

# Collect all light-* crate versions from workspace metadata
VERSIONS=$(cargo metadata --format-version 1 --no-deps --manifest-path "$REPO_ROOT/Cargo.toml" \
  | jq -r '.packages[] | select(.name | startswith("light-")) | "\(.name)=\(.version)"' \
  | sort)

if [ -z "$VERSIONS" ]; then
  echo "Error: no light-* crates found in workspace metadata" >&2
  exit 1
fi

echo "$VERSIONS"

# Extract and output the primary SDK version for branch naming
LIGHT_SDK_VERSION=$(echo "$VERSIONS" | grep '^light-sdk=' | head -1 | cut -d= -f2)
if [ -n "$LIGHT_SDK_VERSION" ]; then
  echo "LIGHT_SDK_VERSION=$LIGHT_SDK_VERSION"
fi
