#!/usr/bin/env bash
# Unified bootstrap for Factory remote workspaces + agents.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

BASE_BRANCH="${FACTORY_BASE_BRANCH:-main}"
CARGO_FETCH_ARGS="${FACTORY_CARGO_FETCH_ARGS:---locked}"
SKIP_BOOTSTRAP_FAST="${FACTORY_SKIP_BOOTSTRAP_FAST:-0}"
SKIP_CHECK="${FACTORY_SKIP_CARGO_CHECK:-0}"

log() {
  printf '\033[1;34m[factory-setup]\033[0m %s\n' "$*"
}
warn() {
  printf '\033[1;33m[factory-setup][warn]\033[0m %s\n' "$*" >&2
}
fail() {
  printf '\033[1;31m[factory-setup][error]\033[0m %s\n' "$*" >&2
  exit 1
}

ensure_git_state() {
  if ! command -v git >/dev/null 2>&1; then
    fail "git is required"
  fi
  git config --global --add safe.directory "$REPO_ROOT" >/dev/null 2>&1 || true
  log "Fetching latest refs"
  git fetch --all --tags --prune
  if git show-ref --verify --quiet "refs/remotes/origin/${BASE_BRANCH}"; then
    log "Fast-forwarding ${BASE_BRANCH}"
    git checkout "$BASE_BRANCH" >/dev/null 2>&1 || true
    git pull --ff-only origin "$BASE_BRANCH" || warn "Unable to fast-forward ${BASE_BRANCH}"
  else
    warn "Remote branch origin/${BASE_BRANCH} not found; skipping pull"
  fi
}

install_rustup() {
  if command -v rustup >/dev/null 2>&1; then
    return
  fi
  log "Installing rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
}

ensure_rust_toolchain() {
  install_rustup
  log "Updating Rust toolchain"
  rustup update --no-self-update stable
  rustup component add clippy rustfmt rust-src >/dev/null 2>&1 || warn "Unable to add rust components"
}

cargo_bootstrap() {
  if ! command -v cargo >/dev/null 2>&1; then
    fail "cargo missing after rustup install"
  fi
  log "Fetching cargo dependencies (${CARGO_FETCH_ARGS})"
  if ! cargo fetch ${CARGO_FETCH_ARGS}; then
    warn "cargo fetch ${CARGO_FETCH_ARGS} failed; retrying unlocked"
    cargo fetch || warn "cargo fetch failed"
  fi
  if [ "$SKIP_CHECK" != "1" ]; then
    log "Running cargo check"
    cargo check --workspace --all-targets || warn "cargo check reported issues"
  fi
}

ensure_corepack() {
  if command -v corepack >/dev/null 2>&1; then
    corepack enable >/dev/null 2>&1 || true
    return
  fi
  if command -v npm >/dev/null 2>&1; then
    log "Installing corepack globally"
    npm install -g corepack >/dev/null 2>&1 || warn "corepack install failed"
    corepack enable >/dev/null 2>&1 || true
  else
    warn "npm missing; skipping corepack"
  fi
}

install_global_node_tools() {
  local prettier_version="${PRETTIER_VERSION:-latest}"
  local markdownlint_version="${MARKDOWNLINT_VERSION:-latest}"
  local markdownlint_formatter_version="${MARKDOWNLINT_FORMATTER_VERSION:-latest}"
  if command -v npm >/dev/null 2>&1; then
    log "Installing npm CLIs (prettier@${prettier_version}, markdownlint-cli2@${markdownlint_version})"
    npm install -g \
      "prettier@${prettier_version}" \
      "markdownlint-cli2@${markdownlint_version}" \
      "markdownlint-cli2-formatter-default@${markdownlint_formatter_version}" \
      >/dev/null 2>&1 || warn "npm global install failed"
  elif command -v bun >/dev/null 2>&1; then
    log "Installing CLIs with bun"
    bun add --global \
      "prettier@${prettier_version}" \
      "markdownlint-cli2@${markdownlint_version}" \
      "markdownlint-cli2-formatter-default@${markdownlint_formatter_version}" \
      >/dev/null 2>&1 || warn "bun global install failed"
  else
    warn "No npm/bun found; skipping formatter installs"
  fi
}

ensure_bun() {
  if command -v bun >/dev/null 2>&1; then
    return
  fi
  if [ "${INSTALL_BUN:-1}" = "1" ]; then
    log "Installing Bun runtime"
    curl -fsSL https://bun.sh/install | bash >/dev/null 2>&1 || warn "Bun install failed"
    export PATH="$HOME/.bun/bin:$PATH"
  fi
}

setup_hooks() {
  if [ "$SKIP_BOOTSTRAP_FAST" = "1" ]; then
    return
  fi
  if [ -x "$REPO_ROOT/scripts/bootstrap-fast.sh" ]; then
    log "Running scripts/bootstrap-fast.sh"
    "$REPO_ROOT/scripts/bootstrap-fast.sh"
  else
    log "Running lefthook install"
    if command -v lefthook >/dev/null 2>&1; then
      lefthook install || warn "lefthook install failed"
    fi
  fi
}

run_fmt_docs() {
  if [ -x "$REPO_ROOT/scripts/fmt-docs.sh" ]; then
    log "Formatting docs"
    "$REPO_ROOT/scripts/fmt-docs.sh" || warn "fmt-docs encountered issues"
  fi
}

main() {
  ensure_git_state
  ensure_rust_toolchain
  cargo_bootstrap
  ensure_bun
  ensure_corepack
  install_global_node_tools
  setup_hooks
  run_fmt_docs
  log "Factory agent setup complete"
}

main "$@"
