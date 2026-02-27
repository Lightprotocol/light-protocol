#!/usr/bin/env bash
set -euo pipefail

# Bump Light Protocol dependency versions in a downstream repo and apply migrations.
# Usage:
#   ./scripts/release/bump-downstream.sh <repo-dir> <versions-env> [--migrations-dir <dir>]
#
# Arguments:
#   repo-dir:       Path to the cloned downstream repo
#   versions-env:   Path to versions env file (output of collect-versions.sh)
#   --migrations-dir: Optional path to migrations directory (default: same dir as this script/migrations/)
#
# The script:
#   1. Detects the current light-sdk version in the target repo
#   2. Bumps all light-* crate versions in Cargo.toml files
#   3. Sets @lightprotocol/* npm packages to "beta" tag
#   4. Applies migration rules for the version gap
#   5. Attempts build verification
#   6. Writes a .bump-report.md summary
#
# Exit codes:
#   0: All builds passed (or no build system found)
#   1: Some builds failed (changes still applied, report written)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  echo "Usage: $0 <repo-dir> <versions-env> [--migrations-dir <dir>]" >&2
  exit 1
}

if [ $# -lt 2 ]; then
  usage
fi

REPO_DIR="$(cd "$1" && pwd)"
VERSIONS_ENV="$2"
shift 2

MIGRATIONS_DIR="$SCRIPT_DIR/migrations"
while [ $# -gt 0 ]; do
  case "$1" in
    --migrations-dir)
      MIGRATIONS_DIR="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      ;;
  esac
done

if [ ! -d "$REPO_DIR" ]; then
  echo "Error: repo directory does not exist: $REPO_DIR" >&2
  exit 1
fi

if [ ! -f "$VERSIONS_ENV" ]; then
  echo "Error: versions env file does not exist: $VERSIONS_ENV" >&2
  exit 1
fi

# ── Helpers ──────────────────────────────────────────────────────────────────

log() { echo "[bump] $*"; }
warn() { echo "[bump] WARNING: $*" >&2; }

# Read a version from the env file for a given crate name.
get_target_version() {
  local crate="$1"
  grep "^${crate}=" "$VERSIONS_ENV" | head -1 | cut -d= -f2
}

# Compare semver major.minor (returns 0 if $1 < $2, 1 if equal, 2 if $1 > $2).
compare_minor() {
  local a_major a_minor b_major b_minor
  a_major=$(echo "$1" | cut -d. -f1)
  a_minor=$(echo "$1" | cut -d. -f2)
  b_major=$(echo "$2" | cut -d. -f1)
  b_minor=$(echo "$2" | cut -d. -f2)

  if [ "$a_major" -lt "$b_major" ] || { [ "$a_major" -eq "$b_major" ] && [ "$a_minor" -lt "$b_minor" ]; }; then
    echo "lt"
  elif [ "$a_major" -eq "$b_major" ] && [ "$a_minor" -eq "$b_minor" ]; then
    echo "eq"
  else
    echo "gt"
  fi
}

# ── Step 1: Detect current version ──────────────────────────────────────────

log "Detecting current Light SDK version in $REPO_DIR..."

CURRENT_VERSION=""
for probe_crate in light-sdk light-program-test light-client; do
  # Search all Cargo.toml files for this crate as a dependency key (unquoted TOML key)
  FOUND=$(grep -rh "${probe_crate}" "$REPO_DIR" --include='Cargo.toml' 2>/dev/null || true)
  if [ -n "$FOUND" ]; then
    # Try simple format: light-sdk = "0.22.0"
    VER=$(echo "$FOUND" | grep -oP "^${probe_crate}\s*=\s*\"\K[0-9]+\.[0-9]+\.[0-9]+" | head -1 || true)
    if [ -z "$VER" ]; then
      # Try table format: light-sdk = { version = "0.22.0", ... }
      VER=$(echo "$FOUND" | grep "${probe_crate}" | grep -oP 'version\s*=\s*"\K[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
    fi
    if [ -n "$VER" ]; then
      CURRENT_VERSION="$VER"
      log "Detected current version: $CURRENT_VERSION (from $probe_crate)"
      break
    fi
  fi
done

if [ -z "$CURRENT_VERSION" ]; then
  warn "Could not detect current Light SDK version — will skip migrations"
fi

TARGET_VERSION=$(get_target_version "light-sdk")
if [ -z "$TARGET_VERSION" ]; then
  echo "Error: light-sdk not found in versions env file" >&2
  exit 1
fi

log "Target version: $TARGET_VERSION"

# ── Step 2: Bump Cargo.toml versions ────────────────────────────────────────

log "Bumping Cargo.toml dependency versions..."

CARGO_FILES=$(find "$REPO_DIR" -name 'Cargo.toml' -not -path '*/target/*')
BUMPED_CRATES=()

while IFS='=' read -r crate version; do
  [ -z "$crate" ] && continue
  # Skip non-crate lines (like LIGHT_SDK_VERSION)
  [[ "$crate" == LIGHT_* ]] && continue

  for cargo_file in $CARGO_FILES; do
    # Check if this crate appears in this Cargo.toml (broad match; sed is anchored)
    if grep -q "${crate}" "$cargo_file" 2>/dev/null; then
      # Simple format: light-sdk = "0.18.0" → light-sdk = "0.22.0"
      sed -i -E "s/^(${crate}\s*=\s*)\"[0-9]+\.[0-9]+\.[0-9]+\"/\1\"${version}\"/" "$cargo_file"

      # Table format: light-sdk = { version = "0.18.0", ... }
      # Match the line with the crate name, then replace version on the same line
      sed -i -E "s/^(${crate}\s*=\s*\{[^}]*version\s*=\s*)\"[0-9]+\.[0-9]+\.[0-9]+\"/\1\"${version}\"/" "$cargo_file"

      # Multi-line table format: light-sdk = { \n  version = "0.18.0", ... }
      # Use address range from crate line to next closing brace
      sed -i -E "/^${crate}\s*=/,/\}/ s/(version\s*=\s*)\"[0-9]+\.[0-9]+\.[0-9]+\"/\1\"${version}\"/" "$cargo_file"

      if ! printf '%s\n' "${BUMPED_CRATES[@]}" | grep -qx "$crate" 2>/dev/null; then
        BUMPED_CRATES+=("$crate")
      fi
    fi
  done
done < "$VERSIONS_ENV"

log "Bumped ${#BUMPED_CRATES[@]} crates in Cargo.toml files"

# ── Step 3: Bump package.json versions ──────────────────────────────────────

log "Bumping package.json @lightprotocol/* versions..."

PKG_FILES=$(find "$REPO_DIR" -name 'package.json' -not -path '*/node_modules/*' -not -path '*/target/*')
PKG_BUMPED=0

for pkg_file in $PKG_FILES; do
  if grep -q '@lightprotocol/' "$pkg_file" 2>/dev/null; then
    # Set dependencies and devDependencies to "beta", but leave peerDependencies alone
    # Uses jq if available, falls back to sed
    if command -v jq &>/dev/null; then
      TMP_FILE=$(mktemp)
      jq '
        if .dependencies then
          .dependencies |= with_entries(
            if (.key | startswith("@lightprotocol/")) then .value = "beta" else . end
          )
        else . end |
        if .devDependencies then
          .devDependencies |= with_entries(
            if (.key | startswith("@lightprotocol/")) then .value = "beta" else . end
          )
        else . end
      ' "$pkg_file" > "$TMP_FILE" && mv "$TMP_FILE" "$pkg_file"
    else
      # Fallback: sed-based replacement (less precise but functional)
      sed -i -E 's/("@lightprotocol\/[^"]+"\s*:\s*)"[^"]+"/\1"beta"/g' "$pkg_file"
      warn "jq not available — used sed fallback for $pkg_file (peerDependencies may be affected)"
    fi
    PKG_BUMPED=$((PKG_BUMPED + 1))
  fi
done

log "Updated $PKG_BUMPED package.json files"

# ── Step 4: Apply migration rules ──────────────────────────────────────────

MIGRATIONS_APPLIED=0

if [ -n "$CURRENT_VERSION" ] && [ -d "$MIGRATIONS_DIR" ]; then
  CURRENT_MINOR="${CURRENT_VERSION%.*}"
  CURRENT_MINOR="${CURRENT_MINOR#*.}"
  CURRENT_MAJOR="${CURRENT_VERSION%%.*}"

  TARGET_MINOR="${TARGET_VERSION%.*}"
  TARGET_MINOR="${TARGET_MINOR#*.}"
  TARGET_MAJOR="${TARGET_VERSION%%.*}"

  log "Checking for migration rules (${CURRENT_MAJOR}.${CURRENT_MINOR} → ${TARGET_MAJOR}.${TARGET_MINOR})..."

  # Find all migration files and apply those in the version range
  for migration_file in "$MIGRATIONS_DIR"/*.sed "$MIGRATIONS_DIR"/*.sh; do
    [ -f "$migration_file" ] || continue

    basename=$(basename "$migration_file")
    # Extract from-to versions from filename: e.g., 0.22-to-0.23.sed
    from_ver=$(echo "$basename" | sed -E 's/^([0-9]+\.[0-9]+)-to-.*$/\1/')
    to_ver=$(echo "$basename" | sed -E 's/^.*-to-([0-9]+\.[0-9]+)\..+$/\1/')

    # Check if this migration is in range: from >= current AND to <= target
    if [ "$(compare_minor "$from_ver" "${CURRENT_MAJOR}.${CURRENT_MINOR}")" = "lt" ]; then
      continue
    fi
    # from_ver should be >= current_minor
    cmp_from=$(compare_minor "${CURRENT_MAJOR}.${CURRENT_MINOR}" "$from_ver")
    cmp_to=$(compare_minor "$to_ver" "${TARGET_MAJOR}.${TARGET_MINOR}")

    if [ "$cmp_from" != "gt" ] && [ "$cmp_to" != "gt" ]; then
      log "Applying migration: $basename"

      if [[ "$migration_file" == *.sed ]]; then
        # Apply sed rules to all .rs and .ts files
        find "$REPO_DIR" \( -name '*.rs' -o -name '*.ts' \) \
          -not -path '*/target/*' \
          -not -path '*/node_modules/*' \
          -exec sed -i -E -f "$migration_file" {} +
      elif [[ "$migration_file" == *.sh ]]; then
        bash "$migration_file" "$REPO_DIR"
      fi

      MIGRATIONS_APPLIED=$((MIGRATIONS_APPLIED + 1))
    fi
  done

  log "Applied $MIGRATIONS_APPLIED migration(s)"
else
  log "Skipping migrations (no current version detected or no migrations dir)"
fi

# ── Step 5: Build verification ──────────────────────────────────────────────

log "Attempting build verification..."

BUILD_STATUS="skipped"
BUILD_LOG=""

# Check for Rust workspace
if [ -f "$REPO_DIR/Cargo.toml" ] && grep -q '\[workspace\]' "$REPO_DIR/Cargo.toml" 2>/dev/null; then
  log "Running: cargo check --workspace"
  if BUILD_LOG=$(cd "$REPO_DIR" && cargo check --workspace 2>&1); then
    BUILD_STATUS="rust_pass"
    log "Rust workspace build: PASS"
  else
    BUILD_STATUS="rust_fail"
    warn "Rust workspace build: FAIL"
  fi
elif [ -f "$REPO_DIR/Cargo.toml" ]; then
  log "Running: cargo check"
  if BUILD_LOG=$(cd "$REPO_DIR" && cargo check 2>&1); then
    BUILD_STATUS="rust_pass"
    log "Rust build: PASS"
  else
    BUILD_STATUS="rust_fail"
    warn "Rust build: FAIL"
  fi
else
  # Check individual Cargo.toml files
  INDIVIDUAL_CARGOS=$(find "$REPO_DIR" -name 'Cargo.toml' -not -path '*/target/*' -maxdepth 3)
  if [ -n "$INDIVIDUAL_CARGOS" ]; then
    ALL_PASS=true
    for cargo_file in $INDIVIDUAL_CARGOS; do
      pkg_dir=$(dirname "$cargo_file")
      pkg_name=$(grep -oP '^name\s*=\s*"\K[^"]+' "$cargo_file" | head -1)
      if [ -n "$pkg_name" ]; then
        log "Running: cargo check in $pkg_dir"
        if ! SINGLE_LOG=$(cd "$pkg_dir" && cargo check 2>&1); then
          ALL_PASS=false
          BUILD_LOG+="$SINGLE_LOG"$'\n'
        fi
      fi
    done
    BUILD_STATUS=$( [ "$ALL_PASS" = true ] && echo "rust_pass" || echo "rust_fail" )
  fi
fi

# Check for npm/pnpm build
NPM_BUILD_STATUS="skipped"
NPM_BUILD_LOG=""
ROOT_PKG="$REPO_DIR/package.json"
if [ -f "$ROOT_PKG" ]; then
  if [ -f "$REPO_DIR/pnpm-lock.yaml" ]; then
    PKG_MGR="pnpm"
  elif [ -f "$REPO_DIR/yarn.lock" ]; then
    PKG_MGR="yarn"
  else
    PKG_MGR="npm"
  fi

  log "Running: $PKG_MGR install && $PKG_MGR run build"
  if NPM_BUILD_LOG=$(cd "$REPO_DIR" && $PKG_MGR install 2>&1 && $PKG_MGR run build 2>&1); then
    NPM_BUILD_STATUS="pass"
    log "npm build: PASS"
  else
    NPM_BUILD_STATUS="fail"
    warn "npm build: FAIL"
  fi
fi

# ── Step 6: Write report ───────────────────────────────────────────────────

REPORT="$REPO_DIR/.bump-report.md"

{
  echo "# Light Protocol dependency bump report"
  echo ""
  echo "**Target version:** light-sdk $TARGET_VERSION"
  if [ -n "$CURRENT_VERSION" ]; then
    echo "**Previous version:** light-sdk $CURRENT_VERSION"
  fi
  echo "**Date:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo ""

  echo "## Crate version bumps"
  echo ""
  if [ ${#BUMPED_CRATES[@]} -gt 0 ]; then
    for c in "${BUMPED_CRATES[@]}"; do
      v=$(get_target_version "$c")
      echo "- \`$c\` → $v"
    done
  else
    echo "No crates bumped."
  fi
  echo ""

  echo "## package.json updates"
  echo ""
  echo "$PKG_BUMPED package.json file(s) updated (\`@lightprotocol/*\` → \`beta\`)"
  echo ""

  echo "## Migration rules applied"
  echo ""
  echo "$MIGRATIONS_APPLIED migration file(s) applied."
  echo ""

  echo "## Build verification"
  echo ""
  echo "| Target | Status |"
  echo "|--------|--------|"
  if [ "$BUILD_STATUS" != "skipped" ]; then
    echo "| Rust | $([ "$BUILD_STATUS" = "rust_pass" ] && echo "PASS" || echo "FAIL") |"
  fi
  if [ "$NPM_BUILD_STATUS" != "skipped" ]; then
    echo "| npm ($PKG_MGR) | $([ "$NPM_BUILD_STATUS" = "pass" ] && echo "PASS" || echo "FAIL") |"
  fi
  if [ "$BUILD_STATUS" = "skipped" ] && [ "$NPM_BUILD_STATUS" = "skipped" ]; then
    echo "| (none) | No build system detected |"
  fi
  echo ""

  # Include build failure logs (truncated)
  if [ "$BUILD_STATUS" = "rust_fail" ] && [ -n "$BUILD_LOG" ]; then
    echo "<details><summary>Rust build log (last 50 lines)</summary>"
    echo ""
    echo '```'
    echo "$BUILD_LOG" | tail -50
    echo '```'
    echo "</details>"
    echo ""
  fi
  if [ "$NPM_BUILD_STATUS" = "fail" ] && [ -n "$NPM_BUILD_LOG" ]; then
    echo "<details><summary>npm build log (last 50 lines)</summary>"
    echo ""
    echo '```'
    echo "$NPM_BUILD_LOG" | tail -50
    echo '```'
    echo "</details>"
    echo ""
  fi
} > "$REPORT"

log "Report written to $REPORT"

# Exit with failure if any build failed
if [ "$BUILD_STATUS" = "rust_fail" ] || [ "$NPM_BUILD_STATUS" = "fail" ]; then
  exit 1
fi

exit 0
