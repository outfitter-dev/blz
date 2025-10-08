#!/usr/bin/env bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Validating registry sources..."

ERRORS=0
WARNINGS=0

# Validate each .toml file
for toml_file in registry/sources/*.toml; do
    [ -f "$toml_file" ] || continue

    filename=$(basename "$toml_file")
    echo -n "Checking $filename... "

    # Check required fields exist (basic grep check)
    if ! grep -q '^id = ' "$toml_file"; then
        echo -e "${RED}✗ Missing required field: id${NC}"
        ((ERRORS++))
        continue
    fi

    if ! grep -q '^url = ' "$toml_file"; then
        echo -e "${RED}✗ Missing required field: url${NC}"
        ((ERRORS++))
        continue
    fi

    # Extract URL and validate it's reachable
    url=$(grep '^url = ' "$toml_file" | sed 's/url = "\(.*\)"/\1/')

    if command -v curl &> /dev/null; then
        if curl --fail --silent --head --max-time 10 "$url" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
        else
            echo -e "${YELLOW}⚠️  URL unreachable: $url${NC}"
            ((WARNINGS++))
        fi
    else
        echo -e "${YELLOW}⚠️  curl not available, skipping URL check${NC}"
    fi
done

echo ""
echo "================================"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ Validation passed${NC}"
    if [ $WARNINGS -gt 0 ]; then
        echo -e "${YELLOW}  ($WARNINGS warnings)${NC}"
    fi
    exit 0
else
    echo -e "${RED}✗ Validation failed with $ERRORS error(s)${NC}"
    exit 1
fi