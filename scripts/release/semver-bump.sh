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
META_PATH="$REPO_ROOT/.semver-meta.json"
RELEASE_TOOL=(cargo run --quiet -p blz-release --)

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
  "${RELEASE_TOOL[@]}" check --expect "$CURRENT_VERSION" --repo-root "$REPO_ROOT"
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

"${RELEASE_TOOL[@]}" sync --version "$NEW_VERSION" --repo-root "$REPO_ROOT"
"${RELEASE_TOOL[@]}" update-lock --version "$NEW_VERSION" --lock-path "$REPO_ROOT/Cargo.lock"

echo "$NEW_VERSION"
