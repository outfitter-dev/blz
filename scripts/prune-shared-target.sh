#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: prune-shared-target.sh [--check] [--prune | --prune-debug | --prune-all] [--threshold <GB>] [--yes] [--sweep]

Manages the shared target directory (target-shared/) used by git worktrees.

Options:
  --check            Summarize shared target directory sizes and recommend pruning (default).
  --prune            Remove heavy subdirectories (tests, llvm-cov-target, nextest, tmp).
  --prune-debug      Remove debug build caches (target-shared/debug/deps and target-shared/debug/incremental).
  --prune-all        Remove the entire shared target directory (full clean).
  --threshold <GB>   Warn when total shared target size exceeds this gigabyte threshold (default: 8, whole numbers only).
  --sweep            Run cargo-sweep (if installed) after pruning to clear stale incremental artefacts.
  --yes              Do not prompt before pruning.
  --help             Show this message.

Examples:
  scripts/prune-shared-target.sh --check --threshold 10
  scripts/prune-shared-target.sh --prune --yes
  scripts/prune-shared-target.sh --prune-all

Note: This script operates on the shared target directory used by git worktrees.
      For local per-worktree targets, use scripts/prune-target.sh instead.
USAGE
}

mode="check"
threshold_gb=8
auto_confirm=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --prune)
      mode="prune"
      shift
      ;;
    --prune-all)
      mode="prune_all"
      shift
      ;;
    --prune-debug)
      mode="prune_debug"
      shift
      ;;
    --threshold)
      shift
      if [[ $# -eq 0 ]]; then
        echo "Missing value for --threshold" >&2
        usage
        exit 1
      fi
      threshold_gb="$1"
      if ! [[ "${threshold_gb}" =~ ^[0-9]+$ ]]; then
        echo "Invalid threshold '${threshold_gb}' (expected whole number of GB)" >&2
        exit 1
      fi
      shift
      ;;
    --yes|-y)
      auto_confirm=true
      shift
      ;;
    --sweep)
      run_sweep=true
      shift
      ;;
    --help|-h)
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
TARGET_DIR="${REPO_ROOT}/target-shared"
run_sweep=${run_sweep:-false}

if [[ ! -d "${TARGET_DIR}" ]]; then
  echo "Shared target directory not found at ${TARGET_DIR}"
  echo ""
  echo "This is normal if:"
  echo "  - You're not using git worktrees"
  echo "  - CARGO_TARGET_DIR is not configured"
  echo ""
  echo "To enable shared target for worktrees, run: scripts/setup-agent-conductor.sh"
  exit 0
fi

if command -v numfmt >/dev/null 2>&1; then
  human_size() {
    local size_kb=$1
    local bytes=$(( size_kb * 1024 ))
    numfmt --to=iec --suffix=B --format="%.2f" "${bytes}"
  }
else
  human_size() {
    local size_kb=$1
    local units=("KB" "MB" "GB" "TB" "PB")
    local unit_index=0
    local size=$size_kb

    while (( $(echo "$size >= 1024" | bc -l) )) && (( unit_index < 4 )); do
      size=$(echo "scale=2; $size / 1024" | bc)
      unit_index=$((unit_index + 1))
    done

    printf "%.2f %s" "$size" "${units[$unit_index]}"
  }
fi

get_dir_size_kb() {
  local dir=$1
  if [[ -d "${dir}" ]]; then
    du -sk "${dir}" 2>/dev/null | cut -f1
  else
    echo "0"
  fi
}

summarize_target() {
  echo "=== Shared Target Directory Analysis ==="
  echo "Location: ${TARGET_DIR}"
  echo ""

  local total_kb
  total_kb=$(get_dir_size_kb "${TARGET_DIR}")
  local total_human
  total_human=$(human_size "$total_kb")

  echo "Total size: ${total_human}"
  echo ""

  # Analyze subdirectories
  local heavy_paths=(
    "debug"
    "debug/deps"
    "debug/incremental"
    "llvm-cov-target"
    "tests"
    "nextest"
    "tmp"
  )

  echo "Breakdown by subdirectory:"
  for subdir in "${heavy_paths[@]}"; do
    local path="${TARGET_DIR}/${subdir}"
    if [[ -d "${path}" ]]; then
      local size_kb
      size_kb=$(get_dir_size_kb "${path}")
      local size_human
      size_human=$(human_size "$size_kb")
      printf "  %-30s %s\n" "${subdir}/" "${size_human}"
    fi
  done

  echo ""

  # Compare with local target directories from worktrees
  if command -v git >/dev/null 2>&1; then
    local worktree_count
    worktree_count=$(git worktree list 2>/dev/null | wc -l)
    if [[ $worktree_count -gt 1 ]]; then
      echo "Worktree local target directories:"
      git worktree list --porcelain 2>/dev/null | grep '^worktree ' | cut -d' ' -f2 | while read -r worktree_path; do
        local local_target="${worktree_path}/target"
        if [[ -d "${local_target}" ]]; then
          local size_kb
          size_kb=$(get_dir_size_kb "${local_target}")
          local size_human
          size_human=$(human_size "$size_kb")
          printf "  %-50s %s\n" "$(basename "${worktree_path}")/target/" "${size_human}"
        fi
      done
      echo ""
      echo "💡 Tip: Old per-worktree target/ directories can be removed with:"
      echo "   cd <worktree> && scripts/prune-target.sh --prune-all"
      echo ""
    fi
  fi

  # Threshold warning
  local threshold_kb=$((threshold_gb * 1024 * 1024))
  if (( total_kb > threshold_kb )); then
    echo "⚠️  WARNING: Shared target exceeds ${threshold_gb} GB threshold!"
    echo "   Consider running: scripts/prune-shared-target.sh --prune-debug"
    echo "   Or for full clean: scripts/prune-shared-target.sh --prune-all"
  else
    echo "✓ Shared target size is within ${threshold_gb} GB threshold"
  fi
}

prune_heavy_subdirs() {
  local dirs_to_remove=(
    "${TARGET_DIR}/tests"
    "${TARGET_DIR}/llvm-cov-target"
    "${TARGET_DIR}/nextest"
    "${TARGET_DIR}/tmp"
  )

  echo "=== Pruning Heavy Subdirectories ==="
  echo ""

  local total_freed_kb=0
  for dir in "${dirs_to_remove[@]}"; do
    if [[ -d "${dir}" ]]; then
      local size_kb
      size_kb=$(get_dir_size_kb "${dir}")
      local size_human
      size_human=$(human_size "$size_kb")

      if [[ "${auto_confirm}" == "true" ]]; then
        echo "Removing ${dir} (${size_human})..."
        rm -rf "${dir}"
        total_freed_kb=$((total_freed_kb + size_kb))
      else
        read -p "Remove ${dir} (${size_human})? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
          rm -rf "${dir}"
          total_freed_kb=$((total_freed_kb + size_kb))
        fi
      fi
    fi
  done

  if (( total_freed_kb > 0 )); then
    local freed_human
    freed_human=$(human_size "$total_freed_kb")
    echo ""
    echo "✓ Freed ${freed_human}"
  else
    echo "No directories needed pruning"
  fi
}

prune_debug_cache() {
  local dirs_to_remove=(
    "${TARGET_DIR}/debug/deps"
    "${TARGET_DIR}/debug/incremental"
  )

  echo "=== Pruning Debug Build Cache ==="
  echo ""
  echo "This removes incremental compilation caches."
  echo "Rebuilds will be slower until sccache warms up again."
  echo ""

  local total_freed_kb=0
  for dir in "${dirs_to_remove[@]}"; do
    if [[ -d "${dir}" ]]; then
      local size_kb
      size_kb=$(get_dir_size_kb "${dir}")
      local size_human
      size_human=$(human_size "$size_kb")

      if [[ "${auto_confirm}" == "true" ]]; then
        echo "Removing ${dir} (${size_human})..."
        rm -rf "${dir}"
        total_freed_kb=$((total_freed_kb + size_kb))
      else
        read -p "Remove ${dir} (${size_human})? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
          rm -rf "${dir}"
          total_freed_kb=$((total_freed_kb + size_kb))
        fi
      fi
    fi
  done

  if (( total_freed_kb > 0 )); then
    local freed_human
    freed_human=$(human_size "$total_freed_kb")
    echo ""
    echo "✓ Freed ${freed_human}"
  else
    echo "No debug cache needed pruning"
  fi
}

prune_all() {
  echo "=== Full Shared Target Clean ==="
  echo ""

  local size_kb
  size_kb=$(get_dir_size_kb "${TARGET_DIR}")
  local size_human
  size_human=$(human_size "$size_kb")

  echo "This will remove the entire shared target directory:"
  echo "  ${TARGET_DIR} (${size_human})"
  echo ""
  echo "Next build will be slow (~8 minutes cold compile)."
  echo "sccache will help reduce rebuilds once cache warms."
  echo ""

  if [[ "${auto_confirm}" == "true" ]]; then
    echo "Removing ${TARGET_DIR}..."
    rm -rf "${TARGET_DIR}"
    echo "✓ Removed ${size_human}"
  else
    read -p "Remove entire shared target directory? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
      rm -rf "${TARGET_DIR}"
      echo "✓ Removed ${size_human}"
    else
      echo "Cancelled"
    fi
  fi
}

run_cargo_sweep() {
  if ! command -v cargo-sweep >/dev/null 2>&1; then
    echo ""
    echo "💡 cargo-sweep not installed. Install with:"
    echo "   cargo install cargo-sweep"
    return
  fi

  echo ""
  echo "=== Running cargo-sweep ==="

  # Set CARGO_TARGET_DIR for sweep
  export CARGO_TARGET_DIR="${TARGET_DIR}"

  cd "${REPO_ROOT}"
  cargo sweep -s >/dev/null 2>&1 || true
  cargo sweep -f -t 30

  echo "✓ Swept stale artifacts older than 30 days"
}

# Main execution
case "${mode}" in
  check)
    summarize_target
    ;;
  prune)
    prune_heavy_subdirs
    if [[ "${run_sweep}" == "true" ]]; then
      run_cargo_sweep
    fi
    ;;
  prune_debug)
    prune_debug_cache
    if [[ "${run_sweep}" == "true" ]]; then
      run_cargo_sweep
    fi
    ;;
  prune_all)
    prune_all
    # No sweep after prune-all since we just deleted everything
    ;;
esac
