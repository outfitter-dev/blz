#!/usr/bin/env bash
# Factory agent wrapper for setup-agent-universal.sh
# Configures environment for Factory remote workspaces
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configure Factory-specific environment
export AGENT_TYPE=factory
export AGENT_BASE_BRANCH="${FACTORY_BASE_BRANCH:-main}"
export AGENT_CARGO_FETCH_ARGS="${FACTORY_CARGO_FETCH_ARGS:---locked}"
export AGENT_SKIP_BOOTSTRAP_FAST="${FACTORY_SKIP_BOOTSTRAP_FAST:-0}"
export AGENT_SKIP_CARGO_CHECK="${FACTORY_SKIP_CARGO_CHECK:-0}"
export AGENT_SKIP_CARGO_BUILD="${FACTORY_SKIP_CARGO_BUILD:-1}"
export AGENT_SKIP_CARGO_TOOLS="${AGENT_SKIP_CARGO_TOOLS:-1}"
export AGENT_SKIP_GO_TOOLS="${AGENT_SKIP_GO_TOOLS:-1}"
export AGENT_INSTALL_BUN="${AGENT_INSTALL_BUN:-1}"
export AGENT_TZ="${AGENT_TZ:-utc}"

# Execute universal setup
exec "${SCRIPT_DIR}/setup-agent-universal.sh" "$@"
