#!/usr/bin/env bash
# ─── Savant Version Bump ──────────────────────────────────────────────
# Single source of truth: VERSION file at repo root.
#
# Usage:
#   ./scripts/bump-version.sh <new-version>   # Write VERSION + sync all targets
#   ./scripts/bump-version.sh                  # Sync from existing VERSION file
#
# Syncs to: Cargo.toml [workspace.package], dashboard/package.json,
#           dashboard/package-lock.json, tauri.conf.json
# ──────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION_FILE="$ROOT/VERSION"

# ─── Resolve version ─────────────────────────────────────────────────
if [ -n "${1:-}" ]; then
    NEW_VERSION="$1"
    printf '%s' "$NEW_VERSION" > "$VERSION_FILE"
    echo "  VERSION file: $NEW_VERSION"
else
    if [ ! -f "$VERSION_FILE" ]; then
        echo "ERROR: No VERSION file and no argument provided."
        exit 1
    fi
    NEW_VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
    echo "  VERSION file: $NEW_VERSION (existing)"
fi

# Detect old version
OLD_VERSION=$(grep -A5 '\[workspace\.package\]' "$ROOT/Cargo.toml" | grep -oP 'version\s*=\s*"\K\d+\.\d+\.\d+')

if [ "$OLD_VERSION" = "$NEW_VERSION" ]; then
    echo "  Already at v$NEW_VERSION. Nothing to sync."
    exit 0
fi

echo ""
echo "  Savant Version Bump: $OLD_VERSION -> $NEW_VERSION"
echo "  Source: VERSION file"
echo "  ============================================="
echo ""

updated=0

# 1. Root Cargo.toml
sed -i "s/version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/g" "$ROOT/Cargo.toml"
echo "  OK  Cargo.toml [workspace.package] -> $NEW_VERSION"
echo "      (28 Rust crates inherit automatically)"
((updated++))

# 2. dashboard/package.json
sed -i "s/\"$OLD_VERSION\"/\"$NEW_VERSION\"/g" "$ROOT/dashboard/package.json"
echo "  OK  dashboard/package.json -> $NEW_VERSION"
((updated++))

# 3. dashboard/package-lock.json
sed -i "s/\"$OLD_VERSION\"/\"$NEW_VERSION\"/g" "$ROOT/dashboard/package-lock.json"
echo "  OK  dashboard/package-lock.json -> $NEW_VERSION"
((updated++))

# 4. Tauri config
sed -i "s/$OLD_VERSION/$NEW_VERSION/g" "$ROOT/crates/desktop/src-tauri/tauri.conf.json"
echo "  OK  crates/desktop/src-tauri/tauri.conf.json -> $NEW_VERSION"
((updated++))

echo ""
echo "  ============================================="
echo "  Synced $updated files from VERSION -> v$NEW_VERSION"
echo "  ============================================="
echo ""

# Verify
if [ "${SKIP_VERIFY:-}" != "1" ]; then
    echo "  Verifying..."
    if cargo check --workspace 2>&1; then
        echo "  cargo check: PASS"
    else
        echo "  cargo check: FAILED"
        exit 1
    fi
fi

echo ""
echo "  Done: $OLD_VERSION -> $NEW_VERSION"
echo ""
