#!/bin/bash
# Syncs the prover version from prover/server/VERSION to a TypeScript constant
# This script is run as part of the CLI build process

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$CLI_DIR")"

VERSION_FILE="$REPO_ROOT/prover/server/VERSION"
OUTPUT_FILE="$CLI_DIR/src/utils/proverVersion.generated.ts"

if [ ! -f "$VERSION_FILE" ]; then
    echo "Error: VERSION file not found at $VERSION_FILE"
    exit 1
fi

VERSION=$(cat "$VERSION_FILE" | tr -d '\n\r')

cat > "$OUTPUT_FILE" << EOF
// Auto-generated from prover/server/VERSION - do not edit manually
export const PROVER_VERSION = "$VERSION";
EOF

echo "Synced prover version $VERSION to $OUTPUT_FILE"
