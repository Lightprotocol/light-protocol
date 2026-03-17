#!/usr/bin/env bash
# Lists sdk-libs crates that have changed since their last git tag.
# For crates with no tag yet, they are always listed.
#
# Usage: ./scripts/sdk-libs-changed-since-tag.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_LIBS_DIR="$REPO_ROOT/sdk-libs"

echo "Checking sdk-libs crates for changes since last tag..."
echo ""

changed=()
no_tag=()

for dir in "$SDK_LIBS_DIR"/*/; do
    [[ -f "$dir/Cargo.toml" ]] || continue

    crate_dir="sdk-libs/$(basename "$dir")"
    pkg_name=$(grep -m1 '^name' "$dir/Cargo.toml" | sed 's/name = "//;s/"//')
    pkg_version=$(grep -m1 '^version' "$dir/Cargo.toml" | sed 's/version = "//;s/"//')

    # Find the latest tag for this package (pattern: <pkg-name>-v*)
    last_tag=$(git -C "$REPO_ROOT" tag --sort=-version:refname \
        | grep -E "^${pkg_name}-v" \
        | head -1 || true)

    if [[ -z "$last_tag" ]]; then
        no_tag+=("$pkg_name ($pkg_version) — no tag found")
        continue
    fi

    # Count commits that touched this crate's directory since the last tag
    changes=$(git -C "$REPO_ROOT" log --oneline "${last_tag}..HEAD" -- "$crate_dir" | wc -l | tr -d ' ')

    if [[ "$changes" -gt 0 ]]; then
        changed+=("$pkg_name ($pkg_version, last tag: $last_tag, $changes commit(s) since)")
    fi
done

if [[ ${#changed[@]} -gt 0 ]]; then
    echo "Changed since last tag:"
    for entry in "${changed[@]}"; do
        echo "  $entry"
    done
    echo ""
fi

if [[ ${#no_tag[@]} -gt 0 ]]; then
    echo "No tag found (never released):"
    for entry in "${no_tag[@]}"; do
        echo "  $entry"
    done
    echo ""
fi

if [[ ${#changed[@]} -eq 0 && ${#no_tag[@]} -eq 0 ]]; then
    echo "All sdk-libs crates are up to date with their tags."
fi
