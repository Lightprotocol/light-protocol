#!/bin/bash
# Syncs the photon version and commit from the external/photon submodule to a TypeScript constant.
# This script is run as part of the CLI build process.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$CLI_DIR")"

PHOTON_DIR="$REPO_ROOT/external/photon"
OUTPUT_FILE="$CLI_DIR/src/utils/photonVersion.generated.ts"

if [ ! -d "$PHOTON_DIR" ]; then
    echo "Error: photon submodule not found at $PHOTON_DIR"
    echo "       Run: git submodule update --init external/photon"
    exit 1
fi

VERSION=$(grep '^version' "$PHOTON_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
COMMIT=$(git -C "$PHOTON_DIR" rev-parse HEAD 2>/dev/null)

if [ -z "$VERSION" ] || [ -z "$COMMIT" ]; then
    echo "Error: Could not extract version or commit from photon submodule"
    exit 1
fi

REPO="https://github.com/lightprotocol/photon.git"

cat > "$OUTPUT_FILE" << EOF
// Auto-generated from external/photon submodule - do not edit manually
export const PHOTON_VERSION = "$VERSION";
export const PHOTON_GIT_COMMIT = "$COMMIT";
export const PHOTON_GIT_REPO = "$REPO";
EOF

echo "Synced photon version $VERSION (commit $COMMIT) to $OUTPUT_FILE"
