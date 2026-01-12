#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PLUGIN_META_DIR="${REPO_ROOT}/.claude-plugin"
PLUGIN_JSON="${PLUGIN_META_DIR}/plugin.json"
AGENTS_DIR="${REPO_ROOT}/packages/agents"

MARKETPLACE_NAME="blz-local"
PLUGIN_NAME="blz"

INSTALL=0
SCOPE="user"
DATA_DIR_OVERRIDE=""

info() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
error() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
  cat <<'USAGE'
Install the BLZ Claude plugin via a local marketplace.

Usage:
  ./scripts/install-claude-plugin-local.sh [options]

Options:
  --install           Run "claude plugin" commands after setup
  --scope <scope>     Install scope when using --install (user|project). Default: user
  --data-dir <path>   Override BLZ data directory for the local marketplace
  -h, --help          Show this help message
USAGE
}

resolve_data_dir() {
  if [[ -n "${DATA_DIR_OVERRIDE}" ]]; then
    printf '%s\n' "${DATA_DIR_OVERRIDE}"
    return
  fi

  if [[ -n "${BLZ_DATA_DIR:-}" ]]; then
    printf '%s\n' "${BLZ_DATA_DIR}"
    return
  fi

  local slug="blz"
  local dot_slug=".blz"
  if [[ "${BLZ_PROFILE:-}" =~ ^[dD][eE][vV]$ ]]; then
    slug="blz-dev"
    dot_slug=".blz-dev"
  fi

  if [[ -n "${XDG_DATA_HOME:-}" ]]; then
    printf '%s\n' "${XDG_DATA_HOME}/${slug}"
    return
  fi

  printf '%s\n' "${HOME}/${dot_slug}"
}

plugin_version() {
  if command -v python3 >/dev/null 2>&1; then
    PLUGIN_JSON_PATH="${PLUGIN_JSON}" python3 - <<'PY'
import json
import os

path = os.environ.get("PLUGIN_JSON_PATH", "")
with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)
print(data.get("version", ""))
PY
    return
  fi

  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "${PLUGIN_JSON}" | head -n 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install)
      INSTALL=1
      shift
      ;;
    --scope)
      SCOPE="${2:-}"
      shift 2
      ;;
    --data-dir)
      DATA_DIR_OVERRIDE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      error "unknown option: $1"
      ;;
  esac
done

if [[ "${SCOPE}" != "user" && "${SCOPE}" != "project" ]]; then
  error "--scope must be 'user' or 'project'"
fi

if [[ ! -f "${PLUGIN_JSON}" ]]; then
  error "missing ${PLUGIN_JSON}"
fi

if [[ ! -d "${AGENTS_DIR}" ]]; then
  error "missing ${AGENTS_DIR}"
fi

DATA_DIR="$(resolve_data_dir)"
if [[ -z "${DATA_DIR}" || "${DATA_DIR}" == "/" ]]; then
  error "resolved data dir is unsafe: ${DATA_DIR}"
fi

PLUGIN_DIR="${DATA_DIR}/plugin"
MARKETPLACE_DIR="${DATA_DIR}/.claude-plugin"
MARKETPLACE_PATH="${MARKETPLACE_DIR}/marketplace.json"

VERSION="$(plugin_version)"
if [[ -z "${VERSION}" ]]; then
  error "failed to read plugin version from ${PLUGIN_JSON}"
fi

if [[ -d "${PLUGIN_DIR}" ]]; then
  info "removing existing plugin at ${PLUGIN_DIR}"
  rm -rf "${PLUGIN_DIR}"
fi

mkdir -p "${PLUGIN_DIR}"
mkdir -p "${PLUGIN_DIR}/packages"
mkdir -p "${MARKETPLACE_DIR}"

cp -R "${PLUGIN_META_DIR}" "${PLUGIN_DIR}/.claude-plugin"
cp -R "${AGENTS_DIR}" "${PLUGIN_DIR}/packages/agents"

cat > "${MARKETPLACE_PATH}" <<EOF
{
  "name": "${MARKETPLACE_NAME}",
  "owner": {
    "name": "Outfitter",
    "email": "team@outfitter.dev"
  },
  "plugins": [
    {
      "name": "${PLUGIN_NAME}",
      "source": "./plugin",
      "description": "Fast local documentation search with llms.txt indexing.",
      "version": "${VERSION}",
      "author": {
        "name": "Outfitter",
        "url": "https://github.com/outfitter-dev"
      }
    }
  ]
}
EOF

info "local plugin ready at ${PLUGIN_DIR}"
info "local marketplace written to ${MARKETPLACE_PATH}"

if [[ "${INSTALL}" -eq 1 ]]; then
  if ! command -v claude >/dev/null 2>&1; then
    error "claude CLI not found; install it or rerun without --install"
  fi

  add_output="$(claude plugin marketplace add "${DATA_DIR}" 2>&1 || true)"
  if [[ -n "${add_output}" ]]; then
    if echo "${add_output}" | grep -qi "already"; then
      warn "marketplace already added"
    else
      info "${add_output}"
    fi
  fi

  install_output="$(claude plugin install "${PLUGIN_NAME}@${MARKETPLACE_NAME}" --scope "${SCOPE}" 2>&1 || true)"
  if [[ -n "${install_output}" ]]; then
    if echo "${install_output}" | grep -qi "already"; then
      warn "plugin already installed"
    else
      info "${install_output}"
    fi
  fi
fi

cat <<EOF

Next steps:
  claude plugin marketplace add "${DATA_DIR}"
  claude plugin install ${PLUGIN_NAME}@${MARKETPLACE_NAME} --scope ${SCOPE}
EOF
