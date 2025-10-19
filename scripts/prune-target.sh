#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: prune-target.sh [--check] [--prune | --prune-debug | --prune-all] [--threshold <GB>] [--yes] [--sweep]

Options:
  --check            Summarize target directory sizes and recommend pruning (default).
  --prune            Remove heavy subdirectories (tests, llvm-cov-target, nextest, tmp).
  --prune-debug      Remove debug build caches (target/debug/deps and target/debug/incremental).
  --prune-all        Remove the entire target directory (full clean).
  --threshold <GB>   Warn when total target size exceeds this gigabyte threshold (default: 60, whole numbers only).
  --sweep            Run cargo-sweep (if installed) after pruning to clear stale incremental artefacts.
  --yes              Do not prompt before pruning.
  --help             Show this message.

Examples:
  scripts/prune-target.sh --check --threshold 80
  scripts/prune-target.sh --prune --yes
  scripts/prune-target.sh --prune-all
USAGE
}

mode="check"
threshold_gb=60
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
TARGET_DIR="${REPO_ROOT}/target"
run_sweep=${run_sweep:-false}

if [[ ! -d "${TARGET_DIR}" ]]; then
  echo "Target directory not found at ${TARGET_DIR}"
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
    local size="${size_kb}"

    while (( size >= 1024 && unit_index < ${#units[@]} - 1 )); do
      size=$(( size / 1024 ))
      ((unit_index++))
    done

    printf "%d %s" "${size}" "${units[unit_index]}"
  }
fi

dir_size_kb() {
  local path=$1
  if [[ -d "${path}" ]]; then
    du -sk "${path}" 2>/dev/null | awk '{print $1}'
  else
    echo 0
  fi
}

summarize() {
  local total_kb debug_kb deps_kb incremental_kb tests_kb cov_kb nextest_kb tmp_kb
  total_kb=$(dir_size_kb "${TARGET_DIR}")
  debug_kb=$(dir_size_kb "${TARGET_DIR}/debug")
  deps_kb=$(dir_size_kb "${TARGET_DIR}/debug/deps")
  incremental_kb=$(dir_size_kb "${TARGET_DIR}/debug/incremental")
  tests_kb=$(dir_size_kb "${TARGET_DIR}/tests")
  cov_kb=$(dir_size_kb "${TARGET_DIR}/llvm-cov-target")
  nextest_kb=$(dir_size_kb "${TARGET_DIR}/nextest")
  tmp_kb=$(dir_size_kb "${TARGET_DIR}/tmp")

  printf "Target directory summary (%s):\n" "${TARGET_DIR}"
  printf "  total:    %s\n" "$(human_size "${total_kb}")"
  printf "  debug:    %s\n" "$(human_size "${debug_kb}")"
  printf "    deps:   %s\n" "$(human_size "${deps_kb}")"
  printf "    incr:   %s\n" "$(human_size "${incremental_kb}")"
  printf "  tests:    %s\n" "$(human_size "${tests_kb}")"
  printf "  llvm-cov: %s\n" "$(human_size "${cov_kb}")"
  printf "  nextest:  %s\n" "$(human_size "${nextest_kb}")"
  printf "  tmp:      %s\n" "$(human_size "${tmp_kb}")"

  local threshold_kb=$(( threshold_gb * 1024 * 1024 ))
  if (( total_kb > threshold_kb )); then
    echo "⚠️  Target directory exceeds ${threshold_gb} GB. Consider:"
    echo "    scripts/prune-target.sh --prune-debug     # Drop incremental + deps caches"
    echo "    scripts/prune-target.sh --prune           # Drop coverage/test artefacts"
    echo "    scripts/prune-target.sh --prune-all       # Full reset"
  fi
}

confirm_prune() {
  local prompt=$1
  if "${auto_confirm}"; then
    return 0
  fi

  read -rp "${prompt} [y/N] " reply
  case "${reply}" in
    [yY][eE][sS]|[yY]) return 0 ;;
    *) echo "Aborted."; return 1 ;;
  esac
}

prune_heavy_subdirs() {
  local -a paths=(
    "${TARGET_DIR}/llvm-cov-target"
    "${TARGET_DIR}/tests"
    "${TARGET_DIR}/nextest"
    "${TARGET_DIR}/tmp"
  )

  if confirm_prune "Remove heavy subdirectories (llvm-cov-target, tests, nextest, tmp)?"; then
    for path in "${paths[@]}"; do
      if [[ -d "${path}" ]]; then
        echo "Removing ${path}"
        rm -rf "${path}"
      fi
    done
    echo "Prune complete."
  fi
}

prune_debug() {
  local -a paths=(
    "${TARGET_DIR}/debug/deps"
    "${TARGET_DIR}/debug/incremental"
  )

  if confirm_prune "Remove debug caches (debug/deps, debug/incremental)?"; then
    for path in "${paths[@]}"; do
      if [[ -d "${path}" ]]; then
        echo "Removing ${path}"
        rm -rf "${path}"
      fi
    done
    echo "Debug caches removed."
  fi
}

prune_all() {
  if confirm_prune "Remove the entire target directory?"; then
    echo "Removing ${TARGET_DIR}"
    rm -rf "${TARGET_DIR}"
    echo "Target directory removed."
  fi
}

case "${mode}" in
  check)
    summarize
    ;;
  prune)
    prune_heavy_subdirs
    ;;
  prune_debug)
    prune_debug
    ;;
  prune_all)
    prune_all
    ;;
  *)
    echo "Internal error: unknown mode '${mode}'" >&2
    exit 1
    ;;
esac

if [[ "${mode}" != "check" ]] && "${run_sweep}"; then
  if command -v cargo-sweep >/dev/null 2>&1; then
    echo "Running cargo sweep..."
    (cd "${REPO_ROOT}" && cargo sweep -s >/dev/null 2>&1 || true)
    (cd "${REPO_ROOT}" && cargo sweep -f -t 0 >/dev/null 2>&1 || true)
    echo "cargo sweep completed."
  else
    echo "cargo-sweep not installed; skipping sweep."
  fi
fi
