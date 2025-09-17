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
lock_path = Path('Cargo.lock')
if not lock_path.exists():
    raise SystemExit("Cargo.lock not found; refusing to proceed. Generate it first.")

data = lock_path.read_text(encoding="utf-8").replace("\r\n", "\n")

def bump_in_block(pkg: str, text: str) -> str:
    # Constrain replacement to the matching [[package]] block for `pkg`
    block = re.compile(
        r'^\[\[package\]\]\n'                              # block start
        r'(?:(?!^\[\[package\]\]\n).)*?'                   # until next block
        rf'\bname\s*=\s*"{re.escape(pkg)}"\b'              # name match
        r'(?:(?!^\[\[package\]\]\n).)*?'                   # still within block
        r'(\bversion\s*=\s*")([^"\n]+)(")',                # capture version value
        re.M | re.S
    )
    def _sub(m):
        return f'{m.group(1)}{version}{m.group(3)}'
    new_text, n = block.subn(_sub, text, count=1)
    if n != 1:
        raise SystemExit(f"Failed to update exactly one block for {pkg} in Cargo.lock (updated {n}).")
    return new_text

for pkg in ('blz-cli', 'blz-core'):
    data = bump_in_block(pkg, data)

lock_path.write_text(data, encoding="utf-8")
PY

echo "$NEW_VERSION"
