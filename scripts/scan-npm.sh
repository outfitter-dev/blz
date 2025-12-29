#!/usr/bin/env bash
# Scan package.json files for npm dependencies
# Returns package names that could have documentation

set -euo pipefail

# Colors for output
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
FORMAT="text"
SEARCH_PATH="."
VERBOSE=false

# Usage
usage() {
  cat << EOF
Usage: scan-npm.sh [OPTIONS]

Scan package.json files for npm dependencies

OPTIONS:
  --format <text|json>    Output format (default: text)
  --path <dir>            Path to search (default: current directory)
  --verbose               Show detailed output
  -h, --help              Show this help message

EXAMPLES:
  # Scan current directory
  ./scan-npm.sh

  # Scan specific directory with JSON output
  ./scan-npm.sh --path /path/to/project --format json

  # Verbose mode
  ./scan-npm.sh --verbose
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
    echo -e "${BLUE}[npm]${NC} $1" >&2
  fi
}

# Find all package.json files
log_verbose "Searching for package.json files in $SEARCH_PATH..."
PACKAGE_FILES_RAW=$(find "$SEARCH_PATH" -name "package.json" -not -path "*/node_modules/*" 2>/dev/null || true)

if [[ -z "$PACKAGE_FILES_RAW" ]]; then
  log_verbose "No package.json files found"
  if [[ "$FORMAT" == "json" ]]; then
    echo '{"type":"npm","dependencies":[]}'
  fi
  exit 0
fi

# Convert to array
PACKAGE_FILES=()
while IFS= read -r line; do
  PACKAGE_FILES+=("$line")
done <<< "$PACKAGE_FILES_RAW"

log_verbose "Found ${#PACKAGE_FILES[@]} package.json file(s)"

# Collect all dependencies (using array instead of associative array for Bash 3 compatibility)
DEPS_LIST=()

for package_file in "${PACKAGE_FILES[@]}"; do
  log_verbose "Processing $package_file..."

  # Extract dependencies (not devDependencies, peerDependencies, etc.)
  if [[ -f "$package_file" ]]; then
    # Use jq to extract dependencies
    if command -v jq &> /dev/null; then
      deps=$(jq -r '.dependencies // {} | keys[]' "$package_file" 2>/dev/null || true)

      for dep in $deps; do
        # Skip scoped packages organizational prefixes (keep actual package name)
        # e.g., @tanstack/react-query → tanstack-react-query
        clean_dep=$(echo "$dep" | sed 's/^@//' | tr '/' '-')
        DEPS_LIST+=("$clean_dep")
      done
    else
      # Fallback: use Python if available, otherwise a minimal awk parser.
      if command -v python3 &> /dev/null; then
        deps=$(python3 - "$package_file" <<'PY'
import json
import sys

path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
except Exception:
    sys.exit(0)

for name in (data.get("dependencies") or {}).keys():
    print(name)
PY
)
      elif command -v python &> /dev/null; then
        deps=$(python - "$package_file" <<'PY'
import json
import sys

path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
except Exception:
    sys.exit(0)

for name in (data.get("dependencies") or {}).keys():
    print(name)
PY
)
      else
        log_verbose "Warning: jq and python not found, using awk fallback"
        deps=$(awk '
          /"dependencies"[[:space:]]*:[[:space:]]*{/ {in=1; depth=1; next}
          in {
            if ($0 ~ /{/) depth++
            if ($0 ~ /}/) {depth--; if (depth==0) {in=0; next}}
            if (match($0, /"([^"]+)"[[:space:]]*:/, m)) print m[1]
          }
        ' "$package_file" || true)
      fi

      for dep in $deps; do
        clean_dep=$(echo "$dep" | sed 's/^@//' | tr '/' '-')
        DEPS_LIST+=("$clean_dep")
      done
    fi
  fi
done

# Convert to sorted and deduplicated array
SORTED_DEPS=($(printf '%s\n' "${DEPS_LIST[@]}" | sort -u))

log_verbose "Found ${#SORTED_DEPS[@]} unique dependencies"

# Output results
if [[ "$FORMAT" == "json" ]]; then
  # JSON output
  echo -n '{"type":"npm","dependencies":['
  first=true
  for dep in "${SORTED_DEPS[@]}"; do
    if [[ "$first" == "true" ]]; then
      first=false
    else
      echo -n ','
    fi
    echo -n "\"$dep\""
  done
  echo ']}'
else
  # Text output
  echo "npm dependencies (${#SORTED_DEPS[@]} found):"
  for dep in "${SORTED_DEPS[@]}"; do
    echo "  - $dep"
  done
fi
