#!/usr/bin/env bash
set -euo pipefail

# get-date.sh — Output timestamp in YYYYMMDDHHMM
# Defaults to UTC time. With --file, uses creation time → first commit date → now.

usage() {
  cat << 'EOF'
Usage: get-date.sh [--file <path>] [--local]

Outputs a timestamp in the format: YYYYMMDDHHMM

Rules (when --file is provided):
  1. Use filesystem creation time (if available)
  2. Else use first git commit date for the file
  3. Else use current time

Options:
  --file <path>   Derive timestamp from file metadata/history
  --local         Use local timezone (default: UTC)
  -h, --help      Show this help

Examples:
  ./.agents/scripts/get-date.sh
  ./.agents/scripts/get-date.sh --file README.md
  ./.agents/scripts/get-date.sh --local
EOF
}

FMT="%Y%m%d%H%M"
USE_UTC=1
FILE_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --file)
      FILE_PATH=${2:-}
      shift 2 || { echo "--file requires a path" >&2; exit 2; }
      ;;
    --local)
      USE_UTC=0
      shift
      ;;
    -h|--help)
      usage; exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage; exit 2
      ;;
  esac
done

date_cmd() {
  if [[ ${USE_UTC} -eq 1 ]]; then
    date -u "$@"
  else
    date "$@"
  fi
}

supports_date_r() {
  date -u -r 0 +%Y >/dev/null 2>&1
}

format_epoch() {
  local epoch="$1"
  if supports_date_r; then
    date_cmd -r "$epoch" +"$FMT"
  else
    # GNU date fallback
    date_cmd -d "@${epoch}" +"$FMT"
  fi
}

file_birth_epoch() {
  local f="$1"
  # macOS: %B; Linux (GNU stat): %W
  if stat -f %B "$f" >/dev/null 2>&1; then
    stat -f %B "$f"
  elif stat -c %W "$f" >/dev/null 2>&1; then
    stat -c %W "$f"
  else
    echo 0
  fi
}

from_file() {
  local f="$1"
  local ts=""

  # 1) Filesystem birth time (if supported and >0)
  if [[ -e "$f" ]]; then
    local epoch
    epoch=$(file_birth_epoch "$f" || echo 0)
    if [[ "$epoch" =~ ^[0-9]+$ ]] && [[ "$epoch" -gt 0 ]]; then
      ts=$(format_epoch "$epoch")
    fi
  fi

  # 2) First git commit date for file (format directly to FMT)
  if [[ -z "$ts" ]]; then
    ts=$(git log --diff-filter=A --follow --date=format:%Y%m%d%H%M --format=%cd -- "$f" 2>/dev/null | tail -1 || true)
  fi

  # 3) Now
  if [[ -z "$ts" ]]; then
    ts=$(date_cmd +"$FMT")
  fi

  printf "%s\n" "$ts"
}

main() {
  if [[ -n "$FILE_PATH" ]]; then
    from_file "$FILE_PATH"
  else
    date_cmd +"$FMT"
  fi
}

main "$@"
