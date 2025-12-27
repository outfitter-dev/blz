#!/usr/bin/env bash
# Core dependency scanner orchestrator
# Detects and scans all dependency files in a project

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default values
FORMAT="text"
SEARCH_PATH="."
VERBOSE=false

# Usage
usage() {
  cat << EOF
Usage: scan-dependencies.sh [OPTIONS]

Scan project dependencies from Cargo.toml, package.json, and other sources

This script:
  1. Detects available dependency files
  2. Invokes language-specific scanners
  3. Aggregates results
  4. Cross-references with indexed sources

OPTIONS:
  --format <text|json>    Output format (default: text)
  --path <dir>            Path to search (default: current directory)
  --verbose               Show detailed output
  -h, --help              Show this help message

EXAMPLES:
  # Scan current directory
  ./scan-dependencies.sh

  # Scan specific directory with JSON output
  ./scan-dependencies.sh --path /path/to/project --format json

  # Verbose mode
  ./scan-dependencies.sh --verbose

OUTPUT (JSON):
  {
    "found": {
      "cargo": ["serde", "tokio", "axum"],
      "npm": ["react", "next", "prisma"]
    },
    "total": 6,
    "candidates": ["serde", "tokio", "axum", "react", "next", "prisma"]
  }
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --format)
      FORMAT="$2"
      shift 2
      ;;
    --path)
      SEARCH_PATH="$2"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      usage
      exit 1
      ;;
  esac
done

# Validate format
if [[ "$FORMAT" != "text" && "$FORMAT" != "json" ]]; then
  echo "Error: Format must be 'text' or 'json'"
  exit 1
fi

# Verbose logging
log_verbose() {
  if [[ "$VERBOSE" == "true" ]]; then
    echo -e "${BLUE}[scan]${NC} $1" >&2
  fi
}

log_info() {
  if [[ "$FORMAT" == "text" ]]; then
    echo -e "${GREEN}[scan]${NC} $1"
  fi
}

# Check if blz is available
BLZ_AVAILABLE=false
if command -v blz &> /dev/null; then
  BLZ_AVAILABLE=true
  log_verbose "blz CLI detected"
fi

# Detect available dependency files
log_info "Scanning for dependency files in $SEARCH_PATH..."

CARGO_FILES=$(find "$SEARCH_PATH" -name "Cargo.toml" -not -path "*/target/*" 2>/dev/null | wc -l || echo 0)
NPM_FILES=$(find "$SEARCH_PATH" -name "package.json" -not -path "*/node_modules/*" 2>/dev/null | wc -l || echo 0)

CARGO_FILES=$(echo "$CARGO_FILES" | tr -d ' ')
NPM_FILES=$(echo "$NPM_FILES" | tr -d ' ')

log_verbose "Found $CARGO_FILES Cargo.toml file(s)"
log_verbose "Found $NPM_FILES package.json file(s)"

# Collect all dependencies
ALL_DEPS=()
CARGO_DEPS_LIST=()
NPM_DEPS_LIST=()

# Scan Cargo dependencies
if [[ "$CARGO_FILES" -gt 0 ]]; then
  log_info "Scanning Cargo dependencies..."

  if [[ -x "$SCRIPT_DIR/scan-cargo.sh" ]]; then
    cargo_result=$("$SCRIPT_DIR/scan-cargo.sh" --path "$SEARCH_PATH" --format json)

    # Parse JSON and extract dependencies
    if command -v jq &> /dev/null; then
      cargo_deps=$(echo "$cargo_result" | jq -r '.dependencies[]' 2>/dev/null || true)

      for dep in $cargo_deps; do
        CARGO_DEPS_LIST+=("$dep")
        ALL_DEPS+=("$dep")
      done

      cargo_count=$(echo "$cargo_deps" | grep -cve '^[[:space:]]*$' || true)
      log_verbose "Found $cargo_count Cargo dependencies"
    else
      log_verbose "Warning: jq not found, skipping Cargo dependency parsing"
    fi
  else
    log_verbose "Warning: scan-cargo.sh not found or not executable"
  fi
fi

# Scan npm dependencies
if [[ "$NPM_FILES" -gt 0 ]]; then
  log_info "Scanning npm dependencies..."

  if [[ -x "$SCRIPT_DIR/scan-npm.sh" ]]; then
    npm_result=$("$SCRIPT_DIR/scan-npm.sh" --path "$SEARCH_PATH" --format json)

    # Parse JSON and extract dependencies
    if command -v jq &> /dev/null; then
      npm_deps=$(echo "$npm_result" | jq -r '.dependencies[]' 2>/dev/null || true)

      for dep in $npm_deps; do
        NPM_DEPS_LIST+=("$dep")
        ALL_DEPS+=("$dep")
      done

      npm_count=$(echo "$npm_deps" | grep -cve '^[[:space:]]*$' || true)
      log_verbose "Found $npm_count npm dependencies"
    else
      log_verbose "Warning: jq not found, skipping npm dependency parsing"
    fi
  else
    log_verbose "Warning: scan-npm.sh not found or not executable"
  fi
fi

# Remove duplicates and sort
UNIQUE_DEPS=($(printf '%s\n' "${ALL_DEPS[@]}" | sort -u))

log_info "Total unique dependencies: ${#UNIQUE_DEPS[@]}"

# Output results
if [[ "$FORMAT" == "json" ]]; then
  json_array_from_stdin() {
    if command -v jq &> /dev/null; then
      jq -R -s -c 'split("\n") | map(select(length > 0))'
      return
    fi

    if command -v python3 &> /dev/null; then
      python3 - <<'PY'
import json
import sys

print(json.dumps([line.rstrip("\n") for line in sys.stdin if line.strip()]))
PY
      return
    fi

    return 1
  }

  if ! command -v jq &> /dev/null && ! command -v python3 &> /dev/null; then
    echo "Error: JSON output requires jq or python3 for safe encoding." >&2
    exit 1
  fi

  cargo_json=$(printf '%s\n' "${CARGO_DEPS_LIST[@]}" | sort -u | json_array_from_stdin)
  npm_json=$(printf '%s\n' "${NPM_DEPS_LIST[@]}" | sort -u | json_array_from_stdin)
  candidates_json=$(printf '%s\n' "${UNIQUE_DEPS[@]}" | json_array_from_stdin)

  printf '{"found":{"cargo":%s,"npm":%s},"total":%s,"candidates":%s}\n' \
    "${cargo_json}" \
    "${npm_json}" \
    "${#UNIQUE_DEPS[@]}" \
    "${candidates_json}"
else
  # Text output
  echo ""
  echo -e "${GREEN}━━━ Dependency Scan Results ━━━${NC}"
  echo ""

  if [[ ${#CARGO_DEPS_LIST[@]} -gt 0 ]]; then
    cargo_unique=$(printf '%s\n' "${CARGO_DEPS_LIST[@]}" | sort -u)
    cargo_count=$(echo "$cargo_unique" | wc -l | tr -d ' ')
    echo -e "${YELLOW}Cargo dependencies ($cargo_count):${NC}"
    echo "$cargo_unique" | while read -r dep; do
      echo "  - $dep"
    done
    echo ""
  fi

  if [[ ${#NPM_DEPS_LIST[@]} -gt 0 ]]; then
    npm_unique=$(printf '%s\n' "${NPM_DEPS_LIST[@]}" | sort -u)
    npm_count=$(echo "$npm_unique" | wc -l | tr -d ' ')
    echo -e "${YELLOW}npm dependencies ($npm_count):${NC}"
    echo "$npm_unique" | while read -r dep; do
      echo "  - $dep"
    done
    echo ""
  fi

  echo -e "${GREEN}Total unique dependencies: ${#UNIQUE_DEPS[@]}${NC}"
  echo ""

  if [[ "$BLZ_AVAILABLE" == "true" ]]; then
    echo -e "${BLUE}Tip: Use 'blz list' to see which are already indexed${NC}"
  fi
fi
