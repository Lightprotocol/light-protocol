#!/usr/bin/env bash
set -euo pipefail

# Create release PR with current changes
# Usage: ./scripts/create-release-pr.sh <program-libs|sdk-libs> [target-branch]
# Arguments:
#   release-type: Type of release (program-libs or sdk-libs)
#   target-branch: Branch to compare against (default: origin/main)

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "Usage: $0 <program-libs|sdk-libs> [target-branch]"
    exit 1
fi

RELEASE_TYPE=$1
TARGET_BRANCH="${2:-origin/main}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ ! "$RELEASE_TYPE" =~ ^(program-libs|sdk-libs)$ ]]; then
    echo "Error: Release type must be 'program-libs' or 'sdk-libs'"
    exit 1
fi

# Function to get version changes between two git refs
# Output format: One line per package: "package-name old-version new-version"
get_version_changes() {
    local base_ref="$1"
    local head_ref="$2"

    # Fetch if comparing against remote refs
    if [[ "$base_ref" == origin/* ]]; then
        local branch="${base_ref#origin/}"
        git fetch origin "$branch" 2>/dev/null || true
    fi

    # Set up diff arguments based on head_ref
    # If head_ref is "HEAD", compare against working tree (includes uncommitted changes)
    # Otherwise use three-dot diff for commits
    local diff_args=()
    if [ "$head_ref" = "HEAD" ]; then
        diff_args=("$base_ref")
    else
        diff_args=("$base_ref...$head_ref")
    fi

    # Get list of changed Cargo.toml files in program-libs, sdk-libs, program-tests/merkle-tree, sparse-merkle-tree, and prover
    while IFS= read -r file; do
        # Extract old and new version from the diff
        local versions=$(git diff "${diff_args[@]}" -- "$file" | grep -E '^\+version|^-version' | grep -v '+++\|---')
        local old_ver=$(echo "$versions" | grep '^-version' | head -1 | awk -F'"' '{print $2}')
        local new_ver=$(echo "$versions" | grep '^\+version' | head -1 | awk -F'"' '{print $2}')

        # Only process if version actually changed
        if [ -n "$old_ver" ] && [ -n "$new_ver" ] && [ "$old_ver" != "$new_ver" ]; then
            # Extract actual package name from Cargo.toml
            local pkg_name=$(grep '^name = ' "$file" | head -1 | awk -F'"' '{print $2}')

            if [ -n "$pkg_name" ]; then
                echo "$pkg_name $old_ver $new_ver"
            fi
        fi
    done < <(git diff "${diff_args[@]}" --name-only -- '**/Cargo.toml' | grep -E '(program-libs|sdk-libs|program-tests/merkle-tree|sparse-merkle-tree|prover)/')
}

# Check if there are changes
if git diff --quiet; then
    echo "No changes detected. Please bump versions first."
    exit 1
fi

echo "========================================="
echo "Creating $RELEASE_TYPE release PR"
echo "========================================="
echo ""

# Show what changed
echo "Changed files:"
git diff --name-only | grep Cargo.toml || echo "  (no Cargo.toml changes)"
echo ""

# Detect packages with version changes
echo "Detecting packages with version changes..."
echo "Comparing against: $TARGET_BRANCH"
echo ""

# Get version changes using the function
VERSION_CHANGES_RAW=$(get_version_changes "$TARGET_BRANCH" "HEAD")

# Build packages array and formatted version changes
PACKAGES=()
VERSION_CHANGES=""
while IFS= read -r line; do
    if [ -n "$line" ]; then
        read -r pkg old_ver new_ver <<< "$line"
        PACKAGES+=("$pkg")
        VERSION_CHANGES="${VERSION_CHANGES}  ${pkg}: ${old_ver} → ${new_ver}\n"
    fi
done <<< "$VERSION_CHANGES_RAW"

VERSION_CHANGES=$(echo -e "$VERSION_CHANGES")

echo "Version changes:"
echo "$VERSION_CHANGES"

echo ""
echo "========================================="
echo "Running cargo publish dry-run validation..."
echo "========================================="
echo ""

# Validate packages using the validation script (comparing against target branch)
# Note: Changes are in working directory but not yet committed
if "$SCRIPT_DIR/validate-packages.sh" "$TARGET_BRANCH" "HEAD"; then
    echo ""
    echo "All crates validated successfully"
else
    echo ""
    echo "Validation failed"
    echo ""
    echo "The GitHub Actions PR validation will run the same checks."
    echo "Continue anyway and let CI validate? (y/N) "
    read -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 1
    fi
fi
echo ""

# Create release branch
BRANCH_NAME="release/${RELEASE_TYPE}"
PR_TITLE="chore: bump ${RELEASE_TYPE} versions"

echo "Will create:"
echo "  Branch: $BRANCH_NAME"
echo "  PR: $PR_TITLE"
echo ""
read -p "Create release branch and PR with these changes? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 1
fi

echo "Creating release branch..."
git checkout -b "$BRANCH_NAME"

# Commit changes
git add -A
git commit -m "chore(${RELEASE_TYPE}): bump versions"

# Push branch
echo "Pushing branch to origin..."
git push -u origin "$BRANCH_NAME"

# Create PR
echo ""
echo "Creating pull request..."

# Capitalize first letter of release type (bash 3.2 compatible)
RELEASE_TYPE_CAPS="$(echo ${RELEASE_TYPE:0:1} | tr '[:lower:]' '[:upper:]')${RELEASE_TYPE:1}"

# Build PR body with proper escaping
PR_BODY="## ${RELEASE_TYPE_CAPS} Release

This PR bumps versions for ${RELEASE_TYPE} crates.

### Version Bumps

\`\`\`
${VERSION_CHANGES}
\`\`\`

### Release Process
1. Versions bumped in Cargo.toml files
2. PR validation (dry-run) will run automatically
3. After merge, GitHub Action will publish each crate individually to crates.io and create releases

---
*Generated by \`scripts/create-release-pr.sh ${RELEASE_TYPE}\`*"

gh pr create \
  --title "$PR_TITLE" \
  --body "$PR_BODY" \
  --label "release"

echo ""
echo "Pull request created!"
echo ""
echo "Next steps:"
echo "1. Wait for PR checks to pass (dry-run validation)"
echo "2. Review and merge the PR"
echo "3. GitHub Action will automatically publish to crates.io and create releases"
