#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF' >&2
Usage: scripts/release/semver-bump.sh [--check] <patch|minor|major|canary|set> [value]

Commands:
  patch   Increment patch and clear prerelease/build metadata
  minor   Increment minor, reset patch to 0, clear prerelease/build metadata
  major   Increment major, reset minor/patch to 0, clear prerelease/build metadata
  canary  Produce pre-release version with incrementing canary suffix
  set     Set version explicitly (value required)

Options:
  --check Verify that package metadata matches the workspace version
EOF
  exit 1
}

read_current_version() {
  awk -F '"' '/^[[:space:]]*version[[:space:]]*=/ { print $2; exit }' Cargo.toml
}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
NODE_SCRIPT="$REPO_ROOT/scripts/release/semver-bump.mjs"
META_PATH="$REPO_ROOT/.semver-meta.json"

if [[ ! -f "$NODE_SCRIPT" ]]; then
  echo "Missing helper script: $NODE_SCRIPT" >&2
  exit 1
fi

CHECK=0
if [[ ${1-} == "--check" ]]; then
  CHECK=1
  shift
fi

MODE=${1-}
cd "$REPO_ROOT"

if [[ $CHECK -eq 1 && -z "$MODE" ]]; then
  CURRENT_VERSION=$(read_current_version)
  if [[ -z "$CURRENT_VERSION" ]]; then
    echo "Unable to determine current version from Cargo.toml" >&2
    exit 1
  fi
  if ! command -v node >/dev/null 2>&1; then
    echo "node is required to run version helper scripts" >&2
    exit 1
  fi
  node "$NODE_SCRIPT" check --expect "$CURRENT_VERSION" --repo-root "$REPO_ROOT"
  exit 0
fi

if [[ -z "$MODE" ]]; then
  usage
fi
shift

VALUE=""
if [[ "$MODE" == "set" ]]; then
  VALUE=${1-}
  if [[ -z "$VALUE" ]]; then
    usage
  fi
  shift
fi

if [[ $# -gt 0 ]]; then
  usage
fi

if [[ $CHECK -eq 0 ]]; then
  if [[ -n $(git status --porcelain) ]]; then
    echo "Working tree has uncommitted changes. Commit or stash before bumping." >&2
    exit 1
  fi
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to run version helper scripts" >&2
  exit 1
fi

CURRENT_VERSION=$(read_current_version)
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "Unable to determine current version from Cargo.toml" >&2
  exit 1
fi

if ! command -v cargo-set-version >/dev/null 2>&1; then
  echo "cargo-set-version (cargo-edit) is required. Install with: cargo install cargo-edit" >&2
  exit 1
fi

NEXT_ARGS=(next --mode "$MODE" --current "$CURRENT_VERSION" --meta "$META_PATH" --write-meta)
if [[ "$MODE" == "set" ]]; then
  NEXT_ARGS+=(--value "$VALUE")
fi

NEW_VERSION=$(node "$NODE_SCRIPT" "${NEXT_ARGS[@]}")

if [[ -z "$NEW_VERSION" ]]; then
  echo "Failed to compute new version" >&2
  exit 1
fi

cargo set-version --workspace "$NEW_VERSION"

node "$NODE_SCRIPT" sync --version "$NEW_VERSION" --repo-root "$REPO_ROOT"

python3 - "$NEW_VERSION" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
lock_path = Path("Cargo.lock")
if not lock_path.exists():
    raise SystemExit("Cargo.lock not found; refusing to proceed. Generate it first.")

raw = lock_path.read_text(encoding="utf-8")
separator = "[[package]]"
parts = raw.split(separator)
if len(parts) <= 1:
    raise SystemExit("Unexpected Cargo.lock format: no [[package]] blocks found.")

def update_block(block: str, package: str) -> tuple[str, bool]:
    if f'name = "{package}"' not in block:
        return block, False
    new_block, count = re.subn(
        r'(\bversion\s*=\s*")([^"\n]+)(")',
        rf"\\1{version}\\3",
        block,
        count=1,
    )
    if count != 1:
        raise SystemExit(f"Failed to update version for {package} in Cargo.lock (updated {count}).")
    return new_block, True

updated = {"blz-cli": False, "blz-core": False}
for index, block in enumerate(parts):
    for package in list(updated):
        if updated[package]:
            continue
        new_block, did_update = update_block(block, package)
        if did_update:
            parts[index] = new_block
            updated[package] = True

missing = [pkg for pkg, done in updated.items() if not done]
if missing:
    raise SystemExit(f"Failed to locate package block(s) in Cargo.lock: {', '.join(missing)}")

lock_path.write_text(separator.join(parts), encoding="utf-8")
PY

echo "$NEW_VERSION"
