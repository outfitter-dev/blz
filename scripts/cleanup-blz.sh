#!/usr/bin/env bash
set -euo pipefail

# Remove stale blz processes spawned from cargo test runs and optionally clear
# the shared cargo target directory.

usage() {
  cat <<'USAGE'
Usage: cleanup-blz.sh [--prune-target]

  --prune-target   Also remove the shared cargo target directory (../.blz-target).
                   Use this if you need a completely fresh build cache.
USAGE
}

PRUNE_TARGET=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --prune-target)
      PRUNE_TARGET=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CACHE_DIR=$(cd "$REPO_ROOT/.." && pwd)/.blz-target

kill_processes() {
  local patterns=("${CACHE_DIR}/" "${REPO_ROOT}/target/debug/" "blz search")
  local found=false

  for pattern in "${patterns[@]}"; do
    if pgrep -f "${pattern}" > /dev/null 2>&1; then
      if [[ "${pattern}" == "blz search" ]]; then
        echo "Killing lingering CLI search processes (pattern: ${pattern})"
      else
        echo "Killing lingering processes launched from ${pattern}"
      fi
      pkill -f "${pattern}" || true
      found=true
    fi
  done

  if [[ "${found}" == "false" ]]; then
    echo "No lingering blz processes found"
  fi
}

prune_cache() {
  if [[ -d "${CACHE_DIR}" ]]; then
    echo "Removing shared cargo target directory: ${CACHE_DIR}"
    rm -rf "${CACHE_DIR}"
  else
    echo "Shared cargo target directory already clean"
  fi
}

kill_processes

if [[ "${PRUNE_TARGET}" == "true" ]]; then
  prune_cache
fi

echo "Cleanup complete."
