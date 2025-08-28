#!/usr/bin/env bash

# Script to sync AGENTS.md files to CLAUDE.md
# AGENTS.md is the source of truth - always overwrites CLAUDE.md
# Creates/updates CLAUDE.md as copies of AGENTS.md

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration - go up to scripts directory
source "$(dirname "$SCRIPT_DIR")/scripts/common.sh"

# Counter for statistics
created=0
updated=0
already_synced=0
errors=0

echo "🔍 Searching for AGENTS.md files to sync..."
echo "📋 Mode: AGENTS.md always overwrites CLAUDE.md"
echo ""

# Find all AGENTS.md files and process them
find . -name "AGENTS.md" -type f 2>/dev/null | while read -r agents_file; do
    dir=$(dirname "$agents_file")
    claude_file="$dir/CLAUDE.md"
    
    # Check if CLAUDE.md already exists
    if [ -e "$claude_file" ]; then
        # Check if files are identical
        if cmp -s "$agents_file" "$claude_file"; then
            echo -e "${BLUE}✓ Already in sync: $claude_file${NC}"
            ((already_synced++)) || true
        else
            # Files differ - overwrite CLAUDE.md with AGENTS.md
            if cp "$agents_file" "$claude_file" 2>/dev/null; then
                echo -e "${GREEN}📝 Updated: $claude_file (overwritten from AGENTS.md)${NC}"
                ((updated++)) || true
            else
                echo -e "${RED}❌ Failed to update $claude_file${NC}"
                ((errors++)) || true
            fi
        fi
    else
        # Create new CLAUDE.md as copy of AGENTS.md
        if cp "$agents_file" "$claude_file" 2>/dev/null; then
            echo -e "${GREEN}✅ Created: $claude_file${NC}"
            ((created++)) || true
        else
            echo -e "${RED}❌ Failed to create $claude_file${NC}"
            ((errors++)) || true
        fi
    fi
done

# Summary
echo ""
echo "📊 Summary:"
echo -e "${GREEN}Created: $created new CLAUDE.md files${NC}"
echo -e "${GREEN}Updated: $updated CLAUDE.md files (overwritten from AGENTS.md)${NC}"
echo -e "${BLUE}Already in sync: $already_synced files${NC}"
if [ $errors -gt 0 ]; then
    echo -e "${RED}Errors: $errors${NC}"
fi

# Show all AGENTS.md and CLAUDE.md pairs
echo ""
echo "📄 Current AGENTS.md → CLAUDE.md mappings:"
find . -name "AGENTS.md" -type f 2>/dev/null | while read -r agents_file; do
    dir=$(dirname "$agents_file")
    claude_file="$dir/CLAUDE.md"
    if [ -e "$claude_file" ]; then
        if cmp -s "$agents_file" "$claude_file"; then
            echo -e "  ${GREEN}✓${NC} $agents_file → $claude_file (in sync)"
        else
            echo -e "  ${YELLOW}⚠${NC} $agents_file → $claude_file (different)"
        fi
    else
        echo -e "  ${RED}✗${NC} $agents_file → (no CLAUDE.md)"
    fi
done

exit 0