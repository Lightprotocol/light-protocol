#!/usr/bin/env bash
# Examples: 
#    ./scripts/bump-versions-and-publish-npm.sh minor
#    ./scripts/bump-versions-and-publish-npm.sh patch @lightprotocol/stateless.js @lightprotocol/compressed-token
#    ./scripts/bump-versions-and-publish-npm.sh alpha @lightprotocol/stateless.js 

cd "$(git rev-parse --show-toplevel)"

if ! command -v pnpm &> /dev/null; then
    echo "pnpm is not installed. Please install pnpm first."
    exit 1
fi

get_package_dir() {
    case "$1" in
        "@lightprotocol/stateless.js") echo "js/stateless.js" ;;
        "@lightprotocol/compressed-token") echo "js/compressed-token" ;;
        "@lightprotocol/zk-compression-cli") echo "cli" ;;
        *) echo "" ;;
    esac
}

# Bump version and publish
publish_package() {
    local package_name=$1
    local version_type=$2
    local package_dir=$(get_package_dir "$package_name")

    if [ -z "$package_dir" ]; then
        echo "No directory mapping found for package $package_name."
        return 1
    fi

    echo "Publishing ${package_name} in directory ${package_dir} with a ${version_type} version bump..."
    # set exec permissions
    find "cli/bin" -type f -exec chmod +x {} +

    sleep 5
    if [ "$version_type" == "alpha" ]; then
        if ! (cd "${package_dir}" && pnpm version prerelease --preid alpha && pnpm publish --tag alpha --access private --no-git-checks --verbose); then
            echo "Error occurred while publishing ${package_name}."
            return 1
        fi
    else
        if ! (cd "${package_dir}" && pnpm version "${version_type}" && pnpm publish --access public --no-git-checks --verbose); then
            echo "Error occurred while publishing ${package_name}."
            return 1
        fi
    fi
}

# Defaults to 'patch' if no version type is provided
version_type=${1:-patch}  
shift  # Remove first arg (version type)

error_occurred=0

if [ "$#" -eq 0 ]; then
    echo "Bumping ${version_type} version for all packages..."
    if [ "$version_type" == "alpha" ]; then
        if ! pnpm -r exec -- pnpm version prerelease --preid alpha || ! pnpm -r exec -- pnpm publish --tag alpha --access private --verbose; then
            echo "Error occurred during bulk version bump and publish."
            error_occurred=1
        fi
    else
        if ! pnpm -r exec -- pnpm version "${version_type}" || ! pnpm -r exec -- pnpm publish --access public --verbose; then
            echo "Error occurred during bulk version bump and publish."
            error_occurred=1
        fi
    fi
else
    # If specific packages are provided, bump version for those packages
    for package_name in "$@"; do
        if ! publish_package "${package_name}" "${version_type}"; then
            error_occurred=1
        fi
    done
fi

if [ "$error_occurred" -eq 1 ]; then
    echo "NPM release process completed with errors."
else
    echo "NPM release process completed successfully."
fi