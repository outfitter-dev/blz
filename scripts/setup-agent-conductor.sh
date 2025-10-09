#!/usr/bin/env bash
# Conductor agent wrapper for setup-agent-universal.sh
# Configures environment for Conductor local Mac workspaces
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configure Conductor-specific environment
export AGENT_TYPE=conductor
export AGENT_BASE_BRANCH="${CONDUCTOR_BASE_BRANCH:-main}"
export AGENT_SKIP_HOOKS="${AGENT_SKIP_HOOKS:-0}"
export AGENT_SKIP_BOOTSTRAP_FAST="${AGENT_SKIP_BOOTSTRAP_FAST:-0}"
export AGENT_SKIP_CARGO_CHECK="${AGENT_SKIP_CARGO_CHECK:-0}"
export AGENT_SKIP_CARGO_BUILD="${AGENT_SKIP_CARGO_BUILD:-0}"
export AGENT_SKIP_CARGO_TOOLS="${AGENT_SKIP_CARGO_TOOLS:-0}"
export AGENT_SKIP_GO_TOOLS="${AGENT_SKIP_GO_TOOLS:-0}"
export AGENT_INSTALL_BUN="${AGENT_INSTALL_BUN:-1}"
export AGENT_TZ="${AGENT_TZ:-local}"

# Execute universal setup
exec "${SCRIPT_DIR}/setup-agent-universal.sh" "$@"
