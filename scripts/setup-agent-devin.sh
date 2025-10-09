#!/usr/bin/env bash
# Devin agent wrapper for setup-agent-universal.sh
# Configures environment for Devin VM snapshot-based workspaces
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configure Devin-specific environment
# Devin uses VM snapshots, so we skip hooks and bootstrap-fast
# since those are configured once in the VM snapshot
export AGENT_TYPE=devin
export AGENT_BASE_BRANCH="${DEVIN_BASE_BRANCH:-main}"
export AGENT_SKIP_HOOKS="${AGENT_SKIP_HOOKS:-1}"
export AGENT_SKIP_BOOTSTRAP_FAST="${AGENT_SKIP_BOOTSTRAP_FAST:-1}"
export AGENT_SKIP_CARGO_CHECK="${AGENT_SKIP_CARGO_CHECK:-0}"
export AGENT_SKIP_CARGO_BUILD="${AGENT_SKIP_CARGO_BUILD:-1}"
export AGENT_SKIP_CARGO_TOOLS="${AGENT_SKIP_CARGO_TOOLS:-0}"
export AGENT_SKIP_GO_TOOLS="${AGENT_SKIP_GO_TOOLS:-0}"
export AGENT_INSTALL_BUN="${AGENT_INSTALL_BUN:-1}"
export AGENT_TZ="${AGENT_TZ:-utc}"

# Execute universal setup
exec "${SCRIPT_DIR}/setup-agent-universal.sh" "$@"
