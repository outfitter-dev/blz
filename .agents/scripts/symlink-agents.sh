#!/usr/bin/env bash

# Script to ensure all CLAUDE.md files are symlinks to AGENTS.md
# This replaces the old sync-agents-md.sh script that created copies

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration - go up to scripts directory
source "$(dirname "$SCRIPT_DIR")/scripts/common.sh"

# Counter for statistics
converted=0
already_symlinks=0
no_agents_md=0
errors=0

echo "🔍 Checking CLAUDE.md files to ensure they're symlinks to AGENTS.md..."
echo ""

# Find all CLAUDE.md files (excluding .conductor directories)
find . -name "CLAUDE.md" -not -path "*/.conductor/*" 2>/dev/null | while read -r claude_file; do
    dir=$(dirname "$claude_file")
    agents_file="$dir/AGENTS.md"
    
    # Check if AGENTS.md exists
    if [ ! -f "$agents_file" ]; then
        echo -e "${YELLOW}⚠ No AGENTS.md found for: $claude_file${NC}"
        ((no_agents_md++)) || true
        continue
    fi
    
    # Check if CLAUDE.md is already a symlink
    if [ -L "$claude_file" ]; then
        # Verify it points to AGENTS.md
        target=$(readlink "$claude_file")
        if [ "$target" = "AGENTS.md" ]; then
            echo -e "${BLUE}✓ Already a symlink: $claude_file -> AGENTS.md${NC}"
            ((already_symlinks++)) || true
        else
            echo -e "${YELLOW}⚠ Symlink points to wrong target: $claude_file -> $target${NC}"
            # Fix the symlink
            rm "$claude_file"
            ln -s "AGENTS.md" "$claude_file"
            echo -e "${GREEN}✅ Fixed symlink: $claude_file -> AGENTS.md${NC}"
            ((converted++)) || true
        fi
    elif [ -f "$claude_file" ]; then
        # It's a regular file - check if content differs from AGENTS.md
        if cmp -s "$claude_file" "$agents_file"; then
            echo -e "${YELLOW}📝 Converting identical file to symlink: $claude_file${NC}"
        else
            echo -e "${RED}⚠ WARNING: CLAUDE.md differs from AGENTS.md: $claude_file${NC}"
            echo -e "${RED}  Creating backup at: $claude_file.backup${NC}"
            cp "$claude_file" "$claude_file.backup"
        fi
        
        # Convert to symlink
        rm "$claude_file"
        ln -s "AGENTS.md" "$claude_file"
        echo -e "${GREEN}✅ Converted to symlink: $claude_file -> AGENTS.md${NC}"
        ((converted++)) || true
    fi
done

# Find AGENTS.md files without corresponding CLAUDE.md
echo ""
echo "📄 Checking for AGENTS.md files without CLAUDE.md symlinks..."
find . -name "AGENTS.md" -not -path "*/.conductor/*" 2>/dev/null | while read -r agents_file; do
    dir=$(dirname "$agents_file")
    claude_file="$dir/CLAUDE.md"
    
    if [ ! -e "$claude_file" ]; then
        echo -e "${YELLOW}📝 Creating new symlink: $claude_file -> AGENTS.md${NC}"
        ln -s "AGENTS.md" "$claude_file"
        ((converted++)) || true
    fi
done

# Summary
echo ""
echo "📊 Summary:"
echo -e "${GREEN}Converted to symlinks: $converted${NC}"
echo -e "${BLUE}Already symlinks: $already_symlinks${NC}"
if [ $no_agents_md -gt 0 ]; then
    echo -e "${YELLOW}Missing AGENTS.md: $no_agents_md${NC}"
fi
if [ $errors -gt 0 ]; then
    echo -e "${RED}Errors: $errors${NC}"
fi

echo ""
echo "✅ All CLAUDE.md files are now symlinks to AGENTS.md"
echo "📌 Note: The old sync-agents-md.sh script is no longer needed and can be removed"

exit 0