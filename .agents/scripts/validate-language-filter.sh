#!/usr/bin/env bash
#
# validate-language-filter.sh
#
# Validates language filter effectiveness against the anthropic source by:
# 1. Searching for known multilingual patterns
# 2. Extracting headings from results
# 3. Testing each heading against language filter
# 4. Generating a detailed report
#
# Usage: ./validate-language-filter.sh [--binary path/to/blz] [--output report.json]

set -euo pipefail

# Configuration
BLZ_BINARY="${1:-blz-dev}"
OUTPUT_DIR="$(cd "$(dirname "$0")/../reports" && pwd)"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
OUTPUT_FILE="${OUTPUT_DIR}/language-filter-validation-${TIMESTAMP}.json"
SOURCE="anthropic"

# ANSI colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}=== Language Filter Validation ===${NC}"
echo "Using binary: $BLZ_BINARY"
echo "Source: $SOURCE"
echo "Output: $OUTPUT_FILE"
echo ""

# Test queries designed to find multilingual content
declare -a QUERIES=(
    "npm install"
    "GitHub Actions"
    "plugins marketplace"
    "auto update"
    "configuration settings"
    "quick start guide"
)

# Initialize results
TOTAL_HEADINGS=0
PASSED_HEADINGS=0
FAILED_HEADINGS=0

# Create temporary files for processing
TEMP_RESULTS=$(mktemp)
TEMP_HEADINGS=$(mktemp)
FAILED_EXAMPLES=$(mktemp)

trap 'rm -f "$TEMP_RESULTS" "$TEMP_HEADINGS" "$FAILED_EXAMPLES"' EXIT

echo -e "${BLUE}Searching for multilingual patterns...${NC}"

# Run searches and collect headings
for query in "${QUERIES[@]}"; do
    echo "  Searching: \"$query\""

    $BLZ_BINARY search "$query" --source "$SOURCE" --limit 20 --json 2>/dev/null | \
        jq -r '.results[]? | .headingPath | join(" > ")' >> "$TEMP_HEADINGS" || true
done

# Deduplicate headings
sort -u "$TEMP_HEADINGS" > "${TEMP_HEADINGS}.dedup"
mv "${TEMP_HEADINGS}.dedup" "$TEMP_HEADINGS"

TOTAL_HEADINGS=$(wc -l < "$TEMP_HEADINGS" | tr -d ' ')

echo -e "${BLUE}Found $TOTAL_HEADINGS unique headings${NC}"
echo ""

# Function to detect non-English using simple heuristics
# This mimics what the language filter does
is_non_english() {
    local text="$1"
    local lower=$(echo "$text" | tr '[:upper:]' '[:lower:]')

    # Strong German indicators
    if echo "$lower" | grep -qE '\b(wie|warum|wann|verstehen|implementieren|funktioniert|kontextfenster|umgebung)\b'; then
        echo "German"
        return 0
    fi

    # Strong Spanish indicators
    if echo "$lower" | grep -qE '\b(documentación|introducción|actualizar|deshabilitar|establece)\b'; then
        echo "Spanish"
        return 0
    fi

    # Strong Portuguese indicators
    if echo "$lower" | grep -qE '\b(gerencie|adicione|desenvolva|configurar|fornece|trabalha|mantém)\b'; then
        echo "Portuguese"
        return 0
    fi

    # Strong Italian indicators
    if echo "$lower" | grep -qE '\b(configurare|aggiornare|aggiornamenti|disabilitare|imposta)\b'; then
        echo "Italian"
        return 0
    fi

    # Strong French indicators (with accents)
    if echo "$text" | grep -qE '\b(utilisez|générer|améliorateur|évaluations)\b'; then
        echo "French"
        return 0
    fi

    # Strong Indonesian indicators
    if echo "$lower" | grep -qE '\b(dapat|dilakukan|menyediakan|memungkinkan|menjalankan|alur|kerja)\b'; then
        echo "Indonesian"
        return 0
    fi

    # CJK scripts
    if echo "$text" | grep -qP '[\p{Han}\p{Hiragana}\p{Katakana}\p{Hangul}]'; then
        echo "CJK"
        return 0
    fi

    # Cyrillic script
    if echo "$text" | grep -qP '[\p{Cyrillic}]'; then
        echo "Russian"
        return 0
    fi

    return 1
}

echo -e "${BLUE}Analyzing headings for language...${NC}"

# Initialize language counters
declare -A LANGUAGE_COUNTS
declare -A LANGUAGE_EXAMPLES

# Analyze each heading
while IFS= read -r heading; do
    if [ -z "$heading" ]; then
        continue
    fi

    if detected_lang=$(is_non_english "$heading"); then
        ((FAILED_HEADINGS++))

        # Track by language
        if [ -z "${LANGUAGE_COUNTS[$detected_lang]+x}" ]; then
            LANGUAGE_COUNTS[$detected_lang]=1
            LANGUAGE_EXAMPLES[$detected_lang]="$heading"
        else
            ((LANGUAGE_COUNTS[$detected_lang]++))
            # Keep first 3 examples per language
            example_count=$(echo "${LANGUAGE_EXAMPLES[$detected_lang]}" | grep -c "^" || echo 0)
            if [ "$example_count" -lt 3 ]; then
                LANGUAGE_EXAMPLES[$detected_lang]="${LANGUAGE_EXAMPLES[$detected_lang]}"$'\n'"$heading"
            fi
        fi

        echo "$detected_lang|$heading" >> "$FAILED_EXAMPLES"
    else
        ((PASSED_HEADINGS++))
    fi
done < "$TEMP_HEADINGS"

# Calculate pass rate
PASS_RATE=0
if [ "$TOTAL_HEADINGS" -gt 0 ]; then
    PASS_RATE=$(awk "BEGIN {printf \"%.1f\", ($PASSED_HEADINGS / $TOTAL_HEADINGS) * 100}")
fi

# Print summary
echo ""
echo -e "${BLUE}=== Validation Summary ===${NC}"
echo "Total headings analyzed: $TOTAL_HEADINGS"
echo -e "${GREEN}Passed (English):${NC} $PASSED_HEADINGS"
echo -e "${RED}Failed (Non-English):${NC} $FAILED_HEADINGS"
echo -e "Pass rate: ${PASS_RATE}%"
echo ""

# Print language breakdown
if [ "$FAILED_HEADINGS" -gt 0 ]; then
    echo -e "${YELLOW}=== Non-English Languages Detected ===${NC}"
    for lang in "${!LANGUAGE_COUNTS[@]}"; do
        count=${LANGUAGE_COUNTS[$lang]}
        printf "  %-12s %3d headings\n" "$lang:" "$count"
    done
    echo ""

    # Print examples
    echo -e "${YELLOW}=== Example Non-English Headings ===${NC}"
    for lang in "${!LANGUAGE_EXAMPLES[@]}"; do
        echo -e "${YELLOW}$lang:${NC}"
        echo "${LANGUAGE_EXAMPLES[$lang]}" | head -3 | sed 's/^/  /'
        echo ""
    done
fi

# Generate JSON report
cat > "$OUTPUT_FILE" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "binary": "$BLZ_BINARY",
  "source": "$SOURCE",
  "queries": $(printf '%s\n' "${QUERIES[@]}" | jq -R . | jq -s .),
  "results": {
    "total_headings": $TOTAL_HEADINGS,
    "passed": $PASSED_HEADINGS,
    "failed": $FAILED_HEADINGS,
    "pass_rate": $PASS_RATE
  },
  "languages_detected": {
EOF

# Add language counts to JSON
first=true
for lang in "${!LANGUAGE_COUNTS[@]}"; do
    if [ "$first" = true ]; then
        first=false
    else
        echo "," >> "$OUTPUT_FILE"
    fi
    count=${LANGUAGE_COUNTS[$lang]}
    examples=$(echo "${LANGUAGE_EXAMPLES[$lang]}" | jq -R . | jq -s .)
    cat >> "$OUTPUT_FILE" << EOF
    "$lang": {
      "count": $count,
      "examples": $examples
    }
EOF
done

cat >> "$OUTPUT_FILE" << EOF

  }
}
EOF

echo -e "${GREEN}Report saved to: $OUTPUT_FILE${NC}"

# Exit with failure if pass rate is below threshold
THRESHOLD=90
if (( $(echo "$PASS_RATE < $THRESHOLD" | bc -l) )); then
    echo -e "${RED}❌ Pass rate ($PASS_RATE%) is below threshold ($THRESHOLD%)${NC}"
    echo -e "${YELLOW}Language filter needs improvement!${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Pass rate ($PASS_RATE%) meets threshold ($THRESHOLD%)${NC}"
    exit 0
fi
