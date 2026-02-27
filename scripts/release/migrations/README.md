# Migration rules

Each Light Protocol release that introduces breaking API changes should include
a migration rule file. These are applied automatically by `bump-downstream.sh`
when updating downstream repos.

## File naming

Files are named after the version range they cover:

```
<from-version>-to-<to-version>.sed   # sed substitution rules
<from-version>-to-<to-version>.sh    # shell script for complex migrations
```

Use the `light-sdk` version as the reference version. For example, a migration
from `light-sdk` 0.22.x to 0.23.x is named `0.22-to-0.23.sed`.

## sed rule files

Applied to all `.rs` and `.ts` files in the target repo. Use standard GNU sed
syntax (`-E` extended regex).

```sed
# Comment explaining the change
s/OldTypeName/NewTypeName/g

# Import path change
s/light_sdk::v1::/light_sdk::/g
```

Keep rules idempotent — running them twice should produce the same result.

## Shell script files

For changes that can't be expressed as line-level find-replace (multi-line
edits, conditional logic, structural changes). Receives the repo directory as
`$1`.

```bash
#!/bin/bash
set -euo pipefail
REPO_DIR="$1"
# Your migration logic here
```

Shell scripts run after sed rules for the same version range.

## Applying migrations

`bump-downstream.sh` detects the current `light-sdk` version in the target
repo, determines the version gap, and applies all migration files in order from
the detected version to the target version.

## When to add a migration rule

Add a rule when any of these change in a release:

- Type or struct renames
- Function or method renames
- Module/import path changes
- Function signature changes (parameter additions, reordering, type changes)
- Removed public API items that have a replacement

If a change has no mechanical migration (e.g., entirely new API with no
predecessor), document it in the sed file as a comment only.
