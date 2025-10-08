#!/usr/bin/env bash
# Discover full documentation sources from index files
# Usage: ./discover-links.sh <source-id>
# Example: ./discover-links.sh supabase

set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <source-id>"
  echo "Example: $0 supabase"
  exit 1
fi

SOURCE_ID="$1"
REGISTRY_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TOML_FILE="${REGISTRY_DIR}/sources/${SOURCE_ID}.toml"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

if [[ ! -f "$TOML_FILE" ]]; then
  echo -e "${RED}Error: Source '$SOURCE_ID' not found${NC}"
  echo "Expected: $TOML_FILE"
  exit 1
fi

# Extract URL from TOML
URL=$(grep '^url = ' "$TOML_FILE" | cut -d'"' -f2)

if [[ -z "$URL" ]]; then
  echo -e "${RED}Error: Could not extract URL from $TOML_FILE${NC}"
  exit 1
fi

echo -e "${BLUE}Analyzing: $SOURCE_ID${NC}"
echo "URL: $URL"
echo ""

# Fetch content with error handling
if ! CONTENT=$(curl -sL "$URL" 2>/dev/null); then
  echo -e "${RED}Error: Failed to fetch URL${NC}"
  exit 1
fi

# Check if content is empty
if [[ -z "$CONTENT" ]]; then
  echo -e "${RED}Error: URL returned empty content${NC}"
  exit 1
fi

LINE_COUNT=$(echo "$CONTENT" | wc -l | tr -d ' ')

echo -e "Lines: ${YELLOW}$LINE_COUNT${NC}"

# Determine content type
if [[ $LINE_COUNT -lt 100 ]]; then
  CONTENT_TYPE="index"
elif [[ $LINE_COUNT -lt 1000 ]]; then
  CONTENT_TYPE="mixed"
else
  CONTENT_TYPE="full"
fi

echo -e "Type: ${YELLOW}$CONTENT_TYPE${NC}"
echo ""

# If not an index, no need to discover
if [[ "$CONTENT_TYPE" != "index" ]]; then
  echo -e "${GREEN}✓ Already a full/mixed source (${LINE_COUNT} lines)${NC}"
  echo "No discovery needed."
  exit 0
fi

# Extract .txt links
echo "Extracting .txt links..."

# Find all .txt references (various formats)
LINKS=$(echo "$CONTENT" | grep -oE '(https?://[^)[:space:]]+\.txt|[./][a-zA-Z0-9/_.-]+\.txt|[a-zA-Z0-9/_-]+\.txt)' | sort -u || true)

if [[ -z "$LINKS" ]]; then
  echo -e "${YELLOW}⚠ No .txt links found in index${NC}"
  echo ""
  echo "Manual inspection required. Content preview:"
  echo "$CONTENT" | head -n 20
  exit 0
fi

echo -e "${GREEN}Found $(echo "$LINKS" | wc -l | tr -d ' ') .txt references${NC}"
echo ""

# Process each link
echo "$LINKS" | while IFS= read -r link; do
  # Skip if it's the same as the source URL
  if [[ "$link" == "$URL" ]]; then
    continue
  fi

  # Resolve relative URLs
  if [[ "$link" =~ ^https?:// ]]; then
    FULL_URL="$link"
  elif [[ "$link" =~ ^\./ ]]; then
    # Relative path: ./file.txt
    BASE_URL="${URL%/*}"
    FULL_URL="${BASE_URL}/${link#./}"
  elif [[ "$link" =~ ^/ ]]; then
    # Absolute path: /path/file.txt
    DOMAIN=$(echo "$URL" | grep -oE 'https?://[^/]+')
    FULL_URL="${DOMAIN}${link}"
  else
    # Relative path without ./: file.txt or path/file.txt
    BASE_URL="${URL%/*}"
    FULL_URL="${BASE_URL}/${link}"
  fi

  echo -e "${BLUE}Testing:${NC} $FULL_URL"

  # Generate temporary name from filename
  TEMP_NAME="discover-$(basename "$link" .txt)"

  # Check if blz binary exists
  if ! command -v blz &> /dev/null; then
    echo -e "${YELLOW}  ⚠ blz command not found, skipping dry-run analysis${NC}"
    echo -e "  ${YELLOW}URL: $FULL_URL${NC}"
    continue
  fi

  # Run dry-run analysis
  if RESULT=$(blz add "$TEMP_NAME" "$FULL_URL" --dry-run --quiet 2>/dev/null); then
    # Parse JSON result
    RESULT_TYPE=$(echo "$RESULT" | jq -r '.analysis.contentType // "unknown"')
    RESULT_LINES=$(echo "$RESULT" | jq -r '.analysis.lineCount // 0')
    RESULT_SIZE=$(echo "$RESULT" | jq -r '.analysis.fileSize // "unknown"')

    # Color code by type
    case "$RESULT_TYPE" in
      full)
        TYPE_COLOR="$GREEN"
        VERDICT="✓ GOOD CANDIDATE"
        ;;
      mixed)
        TYPE_COLOR="$YELLOW"
        VERDICT="⚠ MIXED CONTENT"
        ;;
      index)
        TYPE_COLOR="$YELLOW"
        VERDICT="⚠ ANOTHER INDEX"
        ;;
      *)
        TYPE_COLOR="$RED"
        VERDICT="✗ UNKNOWN TYPE"
        ;;
    esac

    echo -e "  Type: ${TYPE_COLOR}${RESULT_TYPE}${NC}, Lines: ${RESULT_LINES}, Size: ${RESULT_SIZE}"
    echo -e "  ${TYPE_COLOR}${VERDICT}${NC}"
  else
    echo -e "  ${RED}✗ Failed to fetch (404 or network error)${NC}"
  fi

  echo ""
done

echo -e "${GREEN}Discovery complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Review good candidates above"
echo "  2. Use 'blz registry create-source' to add them"
echo "  3. Update original '$SOURCE_ID' entry with '(Index)' suffix and 'index' tag"
