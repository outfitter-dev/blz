#!/usr/bin/env bash
# Build the Claude Code plugin distribution from canonical sources
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PLUGIN_DIR="${REPO_ROOT}/claude-plugin"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

# Ensure plugin directory exists
mkdir -p "${PLUGIN_DIR}"/{.claude-plugin,agents,commands,skills}

log_info "Building Claude Code plugin..."

# Sync blz-docs-searcher agent (canonical version in .claude/)
if [[ -f "${REPO_ROOT}/.claude/agents/blz-docs-searcher.md" ]]; then
    cp "${REPO_ROOT}/.claude/agents/blz-docs-searcher.md" "${PLUGIN_DIR}/agents/"
    log_success "Synced blz-docs-searcher agent"
else
    log_warning "Canonical blz-docs-searcher.md not found in .claude/agents/"
fi

# Sync blz-source-manager agent (canonical version in .claude/)
if [[ -f "${REPO_ROOT}/.claude/agents/blz-source-manager.md" ]]; then
    cp "${REPO_ROOT}/.claude/agents/blz-source-manager.md" "${PLUGIN_DIR}/agents/"
    log_success "Synced blz-source-manager agent"
else
    log_warning "Canonical blz-source-manager.md not found in .claude/agents/"
fi

# Note: Commands and skills in claude-plugin/ are currently the canonical versions
# Commands: claude-plugin/commands/ (canonical)
# Skills: claude-plugin/skills/ (canonical)
# Agents: .claude/agents/ (canonical, synced to claude-plugin/)

# Verify plugin.json exists
if [[ ! -f "${PLUGIN_DIR}/.claude-plugin/plugin.json" ]]; then
    log_warning "plugin.json not found! Plugin may not work correctly."
fi

# Verify README exists
if [[ ! -f "${PLUGIN_DIR}/README.md" ]]; then
    log_warning "README.md not found! Consider adding plugin documentation."
fi

log_info "Plugin structure:"
tree -L 2 "${PLUGIN_DIR}" || ls -lR "${PLUGIN_DIR}"

log_success "Plugin build complete at: ${PLUGIN_DIR}"

# Optional: Show what changed
if command -v git &> /dev/null; then
    echo ""
    log_info "Git status:"
    git -C "${REPO_ROOT}" status --short "${PLUGIN_DIR}" || true
fi
