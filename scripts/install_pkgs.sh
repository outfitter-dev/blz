#!/usr/bin/env bash
# Claude Code session hook entry point.
# Runs the universal agent setup only when an AGENT_TYPE has been provided.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

if [ -z "${AGENT_TYPE:-}" ]; then
  # Nothing to do for non-agent sessions.
  exit 0
fi

echo "[install_pkgs] AGENT_TYPE=${AGENT_TYPE} detected; running universal setup"
exec "${REPO_ROOT}/scripts/setup-agent-universal.sh"
