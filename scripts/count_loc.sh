#!/bin/bash

# Function to count production LOC for a directory or file
count_prod_loc() {
    local path=$1
    if [ -d "$path" ]; then
        local count=$(find "$path" -type f \( -name "*.rs" -o -name "*.toml" \) ! -name "Cargo.lock" ! -name "LICENSE" ! -name "README.md" -print0 | xargs -0 awk '
            BEGIN { in_test = 0; in_comment = 0; loc = 0 }
            /^#\[cfg\(test\)\]/ || /^mod test {/ { in_test = 1; next }
            /^}/ && in_test { in_test = 0; next }
            /^\/\*/ { in_comment = 1 }
            /\*\/$/ { in_comment = 0; next }
            !in_test && !in_comment && !/^[[:space:]]*$/ && !/^[[:space:]]*\/\// { loc++ }
            END { print loc }
        ')
        echo "$count"
    else
        local count=$(awk '
            BEGIN { in_test = 0; in_comment = 0; loc = 0 }
            /^#\[cfg\(test\)\]/ || /^mod test {/ { in_test = 1; next }
            /^}/ && in_test { in_test = 0; next }
            /^\/\*/ { in_comment = 1 }
            /\*\/$/ { in_comment = 0; next }
            !in_test && !in_comment && !/^[[:space:]]*$/ && !/^[[:space:]]*\/\// { loc++ }
            END { print loc }
        ' "$path")
        echo "$count"
    fi
}

# Function to recursively count LOC for a directory and its subdirectories
count_loc_recursive() {
    local dir=$1
    local indent=$2
    local total_loc=0
    for item in "$dir"/*; do
        if [ -d "$item" ]; then
            local loc=$(count_prod_loc "$item")
            echo "${indent}|_ $(basename "$item") (Total: $loc)"
            total_loc=$((total_loc + loc))
            count_loc_recursive "$item" "$indent  "
        elif [ -f "$item" ] && [[ "$(basename "$item")" != "Cargo.lock" ]] && [[ "$(basename "$item")" != "LICENSE" ]] && [[ "$(basename "$item")" != "README.md" ]]; then
            local loc=$(count_prod_loc "$item")
            echo "${indent}|_ $(basename "$item"): $loc"
            total_loc=$((total_loc + loc))
        fi
    done
    echo "${indent}Total: $total_loc"
}

# Count LOC for @merkle-tree
echo -e "\n@merkle-tree:"
for subdir in merkle-tree/*; do
    if [ -d "$subdir" ]; then
        echo "$(basename "$subdir")"
        count_loc_recursive "$subdir" "  "
    fi
done

# Count LOC for @programs
echo -e "\n@programs:"
for program in programs/*; do
    if [ -d "$program" ]; then
        echo "$(basename "$program")"
        count_loc_recursive "$program" "  "
    fi
done

# # Count LOC for @macros
# echo -e "\n@macros:"
# for macro in macros/*; do
#     if [ -d "$macro" ]; then
#         echo "$(basename "$macro")"
#         count_loc_recursive "$macro" "  "
#     fi
# done


# Count LOC for @macros/aligned-sized
echo -e "\n@macros/aligned-sized:"
count_loc_recursive "macros/aligned-sized" "  "

# Count LOC for @utils
echo -e "\n@utils:"
count_loc_recursive "utils" "  "

# Count LOC for @heap
echo -e "\n@heap:"
count_loc_recursive "heap" "  "