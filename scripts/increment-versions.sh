#!/usr/bin/env sh

set -eux

bump_dependency_version() {
    local target_dir="$1"
    local base_dependency="$2"
    local new_version="$3"
    local full_dependency="@lightprotocol/$base_dependency"

    cd "$target_dir"

    # Check if package.json exists and has the specified package
    if [ -f "package.json" ] && grep -q "\"$full_dependency\"" "package.json"; then
        
        # Use sed to update the version of the specified package
        # Using | as delimiter in sed to avoid issues with / in the package name
        sed -i "s|\(\"$full_dependency\": \"\)\([^ \"]*\)\"|\1$new_version\"|" package.json
        echo "Updated $full_dependency to version $new_version in $target_dir."
    else
        echo "package.json not found or does not contain $full_dependency in $target_dir."
    fi

    cd - # Go back to the original directory
}



increment_version() {
    while [ "${#}" -gt 0 ]; do
    case "${1}" in
      -d|--dir)
        dir="${2}"
        shift 2
        ;;
      *)
        echo "Unknown option: ${1}"
        return 1
        ;;
    esac
    done
    
    cd "${dir}"
    # Check if package.json exists
    if [ -f "package.json" ]; then
        # Extract the version
        old_version=$(grep '"version":' package.json | awk -F'"' '{print $4}')

        major=$(echo $old_version | cut -d. -f1)
        minor=$(echo $old_version | cut -d. -f2)
        patch_with_prerelease=$(echo $old_version | cut -d. -f3-)

        # Split the patch and prerelease info if there's a '-'
        if echo $patch_with_prerelease | grep -q "-"; then
            patch=$(echo $patch_with_prerelease | cut -d- -f1)
            prerelease_info=$(echo $patch_with_prerelease | cut -s -d- -f2)
        else
            patch=$patch_with_prerelease
            prerelease_info=""
        fi

        # Extract prerelease tag and number
        prerelease_tag=$(echo $prerelease_info | cut -d. -f1)
        prerelease_number=$(echo $prerelease_info | cut -d. -f2)

        echo "Old prerelease_number: $prerelease_number"

        # Increment the prerelease_number
        new_prerelease_number=$((prerelease_number + 1))

        # Construct the new version
        new_version="${major}.${minor}.${patch}-${prerelease_tag}.${new_prerelease_number}"

        echo "Old Version: $old_version"
        echo "New Version: $new_version"

        # Uncomment the next line if you want to replace old version with new version in package.json
        sed -i "s/\"version\": \"$old_version\"/\"version\": \"$new_version\"/" package.json
    else
        echo "No package.json found in ${dir}. Skipping version increment."
    fi
    cd ..

    bump_dependency_version "zk.js" ${dir} $new_version
    bump_dependency_version "cli" ${dir} $new_version
    bump_dependency_version "system-programs" ${dir} $new_version
    bump_dependency_version "circuits" ${dir} $new_version
    bump_dependency_version "relayer" ${dir} $new_version
}




increment_version --dir "prover.js"
increment_version --dir "zk.js"
increment_version -d "system-programs"
increment_version -d "cli"
increment_version -d "circuits"
increment_version -d "relayer"
