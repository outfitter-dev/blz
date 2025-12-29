#!/usr/bin/env bash
# Scan Cargo.toml files for Rust crate dependencies
# Returns crate names that could have documentation

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
Usage: scan-cargo.sh [OPTIONS]

Scan Cargo.toml files for Rust crate dependencies

OPTIONS:
  --format <text|json>    Output format (default: text)
  --path <dir>            Path to search (default: current directory)
  --verbose               Show detailed output
  -h, --help              Show this help message

EXAMPLES:
  # Scan current directory
  ./scan-cargo.sh

  # Scan specific directory with JSON output
  ./scan-cargo.sh --path /path/to/project --format json

  # Verbose mode
  ./scan-cargo.sh --verbose
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
    echo -e "${BLUE}[cargo]${NC} $1" >&2
  fi
}

# Find all Cargo.toml files
log_verbose "Searching for Cargo.toml files in $SEARCH_PATH..."
CARGO_FILES_RAW=$(find "$SEARCH_PATH" -name "Cargo.toml" -not -path "*/target/*" 2>/dev/null || true)

if [[ -z "$CARGO_FILES_RAW" ]]; then
  log_verbose "No Cargo.toml files found"
  if [[ "$FORMAT" == "json" ]]; then
    echo '{"type":"cargo","dependencies":[]}'
  fi
  exit 0
fi

# Convert to array
CARGO_FILES=()
while IFS= read -r line; do
  CARGO_FILES+=("$line")
done <<< "$CARGO_FILES_RAW"

log_verbose "Found ${#CARGO_FILES[@]} Cargo.toml file(s)"

# Collect all dependencies (using array instead of associative array for Bash 3 compatibility)
DEPS_LIST=()

for cargo_file in "${CARGO_FILES[@]}"; do
  log_verbose "Processing $cargo_file..."

  if [[ ! -f "$cargo_file" ]]; then
    continue
  fi

  # Extract dependencies from [dependencies] section
  # This is a simple parser that handles basic TOML syntax
  in_deps_section=false

  while IFS= read -r line; do
    # Trim leading/trailing whitespace
    line=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

    # Check if we're entering [dependencies] section
    if [[ "$line" == "[dependencies]" ]]; then
      in_deps_section=true
      continue
    fi

    # Check if we're leaving dependencies section (new section starts)
    if [[ "$line" =~ ^\[.*\] ]]; then
      in_deps_section=false
      continue
    fi

    # If in dependencies section, extract package names
    if [[ "$in_deps_section" == "true" ]]; then
      # Skip empty lines and comments
      if [[ -z "$line" ]] || [[ "$line" =~ ^# ]]; then
        continue
      fi

      # Extract crate name (before = sign)
      # Handles: package = "version"
      # Handles: package = { version = "...", features = [...] }
      if [[ "$line" =~ ^([a-zA-Z0-9_-]+)[[:space:]]*= ]]; then
        crate="${BASH_REMATCH[1]}"

        # Skip workspace dependencies (they're internal)
        if [[ "$crate" != "workspace" ]]; then
          DEPS_LIST+=("$crate")
        fi
      fi
    fi
  done < "$cargo_file"
done

# Also check [workspace.dependencies] in root Cargo.toml
for cargo_file in "${CARGO_FILES[@]}"; do
  # Only check root-level Cargo.toml (has workspace section)
  if grep -q "^\[workspace\]" "$cargo_file" 2>/dev/null; then
    log_verbose "Checking workspace dependencies in $cargo_file..."

    in_workspace_deps=false

    while IFS= read -r line; do
      line=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

      if [[ "$line" == "[workspace.dependencies]" ]]; then
        in_workspace_deps=true
        continue
      fi

      if [[ "$line" =~ ^\[.*\] ]]; then
        in_workspace_deps=false
        continue
      fi

      if [[ "$in_workspace_deps" == "true" ]]; then
        if [[ -z "$line" ]] || [[ "$line" =~ ^# ]]; then
          continue
        fi

        if [[ "$line" =~ ^([a-zA-Z0-9_-]+)[[:space:]]*= ]]; then
          crate="${BASH_REMATCH[1]}"
          DEPS_LIST+=("$crate")
        fi
      fi
    done < "$cargo_file"
  fi
done

# Convert to sorted and deduplicated array
SORTED_DEPS=($(printf '%s\n' "${DEPS_LIST[@]}" | sort -u))

log_verbose "Found ${#SORTED_DEPS[@]} unique dependencies"

# Output results
if [[ "$FORMAT" == "json" ]]; then
  # JSON output
  echo -n '{"type":"cargo","dependencies":['
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
  echo "Cargo dependencies (${#SORTED_DEPS[@]} found):"
  for dep in "${SORTED_DEPS[@]}"; do
    echo "  - $dep"
  done
fi
