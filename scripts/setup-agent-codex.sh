#!/usr/bin/env bash
# Codex agent wrapper for setup-agent-universal.sh
# Configures environment for OpenAI Codex container-based workspaces
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configure Codex-specific environment
# Codex runs in isolated containers, so skip bootstrap-fast
# but keep hooks since each container session needs them
export AGENT_TYPE=codex
export AGENT_BASE_BRANCH="${CODEX_BASE_BRANCH:-main}"
export AGENT_SKIP_HOOKS="${AGENT_SKIP_HOOKS:-0}"
export AGENT_SKIP_BOOTSTRAP_FAST="${AGENT_SKIP_BOOTSTRAP_FAST:-1}"
export AGENT_SKIP_CARGO_CHECK="${AGENT_SKIP_CARGO_CHECK:-0}"
export AGENT_SKIP_CARGO_BUILD="${AGENT_SKIP_CARGO_BUILD:-1}"
export AGENT_SKIP_CARGO_TOOLS="${AGENT_SKIP_CARGO_TOOLS:-1}"
export AGENT_SKIP_GO_TOOLS="${AGENT_SKIP_GO_TOOLS:-1}"
export AGENT_INSTALL_BUN="${AGENT_INSTALL_BUN:-1}"
export AGENT_TZ="${AGENT_TZ:-utc}"

# Execute universal setup
exec "${SCRIPT_DIR}/setup-agent-universal.sh" "$@"
