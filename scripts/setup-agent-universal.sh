#!/usr/bin/env bash
# Universal agent setup for blz - supports Factory, Conductor, Devin, Codex
# Configure behavior via environment variables (see docs/development/agent-environment-setup.md)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

# ============================================================================
# Environment Variable Configuration with Defaults
# ============================================================================
AGENT_TYPE="${AGENT_TYPE:-unknown}"
AGENT_SKIP_HOOKS="${AGENT_SKIP_HOOKS:-0}"
AGENT_SKIP_BOOTSTRAP_FAST="${AGENT_SKIP_BOOTSTRAP_FAST:-0}"
AGENT_SKIP_CARGO_CHECK="${AGENT_SKIP_CARGO_CHECK:-0}"
AGENT_SKIP_CARGO_BUILD="${AGENT_SKIP_CARGO_BUILD:-0}"
AGENT_SKIP_CARGO_TOOLS="${AGENT_SKIP_CARGO_TOOLS:-0}"
AGENT_SKIP_GO_TOOLS="${AGENT_SKIP_GO_TOOLS:-0}"
AGENT_BASE_BRANCH="${AGENT_BASE_BRANCH:-main}"
AGENT_CARGO_FETCH_ARGS="${AGENT_CARGO_FETCH_ARGS:---locked}"
AGENT_INSTALL_BUN="${AGENT_INSTALL_BUN:-1}"
AGENT_TZ="${AGENT_TZ:-utc}"

# Tool version configuration
PRETTIER_VERSION="${PRETTIER_VERSION:-latest}"
MARKDOWNLINT_VERSION="${MARKDOWNLINT_VERSION:-latest}"
MARKDOWNLINT_FORMATTER_VERSION="${MARKDOWNLINT_FORMATTER_VERSION:-latest}"

# ============================================================================
# Logging Functions
# ============================================================================
log() {
  local prefix
  case "$AGENT_TYPE" in
    factory)   prefix='\033[1;34m[factory-setup]\033[0m' ;;
    conductor) prefix='\033[1;35m[conductor-setup]\033[0m' ;;
    devin)     prefix='\033[1;36m[devin-setup]\033[0m' ;;
    codex)     prefix='\033[1;32m[codex-setup]\033[0m' ;;
    *)         prefix='\033[1;37m[agent-setup]\033[0m' ;;
  esac
  printf '%b %s\n' "$prefix" "$*"
}

warn() {
  printf '\033[1;33m[%s][warn]\033[0m %s\n' "$AGENT_TYPE" "$*" >&2
}

fail() {
  printf '\033[1;31m[%s][error]\033[0m %s\n' "$AGENT_TYPE" "$*" >&2
  exit 1
}

log_step() {
  echo -e "\033[0;34m▶\033[0m $1"
}

log_success() {
  echo -e "  \033[0;32m✓\033[0m $1"
}

log_warning() {
  echo -e "  \033[1;33m⚠\033[0m $1"
}

log_error() {
  echo -e "  \033[0;31m✗\033[0m $1"
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# Git State Management
# ============================================================================
ensure_git_state() {
  if ! has_cmd git; then
    fail "git is required"
  fi

  git config --global --add safe.directory "$REPO_ROOT" >/dev/null 2>&1 || true

  # Only fetch/pull for Factory (remote workspaces need fresh code)
  if [ "$AGENT_TYPE" = "factory" ]; then
    log "Fetching latest refs"
    git fetch --all --tags --prune
    if git show-ref --verify --quiet "refs/remotes/origin/${AGENT_BASE_BRANCH}"; then
      log "Fast-forwarding ${AGENT_BASE_BRANCH}"
      git checkout "$AGENT_BASE_BRANCH" >/dev/null 2>&1 || true
      git pull --ff-only origin "$AGENT_BASE_BRANCH" || warn "Unable to fast-forward ${AGENT_BASE_BRANCH}"
    else
      warn "Remote branch origin/${AGENT_BASE_BRANCH} not found; skipping pull"
    fi
  fi
}

# ============================================================================
# Rust Toolchain Management
# ============================================================================
install_rustup() {
  if has_cmd rustup; then
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

# ============================================================================
# Worktree Detection & Shared Target Setup
# ============================================================================
setup_shared_target() {
  # Skip if CARGO_TARGET_DIR already explicitly set by user
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    log_success "CARGO_TARGET_DIR already set: ${CARGO_TARGET_DIR}"
    return
  fi

  # Detect worktrees
  if ! has_cmd git; then
    return
  fi

  local worktree_count
  worktree_count=$(git worktree list 2>/dev/null | wc -l)

  if [ "$worktree_count" -le 1 ]; then
    # Single worktree or not a git repo - use default behavior
    return
  fi

  # Multiple worktrees detected - set up shared target
  local repo_root
  repo_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "$REPO_ROOT")
  local shared_target="${repo_root}/target-shared"

  log_step "Git worktrees detected (${worktree_count} total)"
  log_success "Configuring shared target directory: ${shared_target}"

  export CARGO_TARGET_DIR="${shared_target}"

  # For conductor, persist to shell rc for convenience
  if [ "$AGENT_TYPE" = "conductor" ]; then
    local shell_rc=""
    # Try shell-specific version variables first, then fall back to $SHELL
    if [ -n "${BASH_VERSION:-}" ] && [ -f "$HOME/.bashrc" ]; then
      shell_rc="$HOME/.bashrc"
    elif [ -n "${ZSH_VERSION:-}" ] && [ -f "$HOME/.zshrc" ]; then
      shell_rc="$HOME/.zshrc"
    elif [[ "${SHELL:-}" == *bash ]] && [ -f "$HOME/.bashrc" ]; then
      shell_rc="$HOME/.bashrc"
    elif [[ "${SHELL:-}" == *zsh ]] && [ -f "$HOME/.zshrc" ]; then
      shell_rc="$HOME/.zshrc"
    fi

    if [ -n "$shell_rc" ]; then
      if ! grep -q "CARGO_TARGET_DIR.*target-shared" "$shell_rc" 2>/dev/null; then
        log "Adding CARGO_TARGET_DIR to ${shell_rc}"
        cat >> "$shell_rc" << EOF

# blz worktree shared target (added by setup-agent-conductor.sh)
export CARGO_TARGET_DIR="${shared_target}"
EOF
        log_success "Shell configuration updated"
      fi
    fi
  fi

  log_success "Shared target enabled: ${CARGO_TARGET_DIR}"
  echo "         This saves disk space by sharing compilation artifacts across worktrees."
  echo "         To disable: unset CARGO_TARGET_DIR"
}

# ============================================================================
# Cargo Bootstrap
# ============================================================================
cargo_bootstrap() {
  if ! has_cmd cargo; then
    fail "cargo missing after rustup install"
  fi

  # Configure shared target if worktrees detected
  setup_shared_target

  log "Fetching cargo dependencies (${AGENT_CARGO_FETCH_ARGS})"
  if ! cargo fetch ${AGENT_CARGO_FETCH_ARGS}; then
    warn "cargo fetch ${AGENT_CARGO_FETCH_ARGS} failed; retrying unlocked"
    cargo fetch || warn "cargo fetch failed"
  fi

  if [ "$AGENT_SKIP_CARGO_CHECK" != "1" ]; then
    log "Running cargo check"
    cargo check --workspace --all-targets || warn "cargo check reported issues"
  fi

  if [ "$AGENT_SKIP_CARGO_BUILD" != "1" ] && [ "$AGENT_TYPE" = "conductor" ]; then
    log "Building workspace"
    if cargo build --workspace --all-targets --quiet; then
      log_success "Build successful"
    else
      warn "Build failed (workspace still usable, but may need fixes)"
    fi
  fi
}

# ============================================================================
# Cargo Tools (Conductor-specific)
# ============================================================================
install_cargo_tools() {
  if [ "$AGENT_SKIP_CARGO_TOOLS" = "1" ]; then
    return
  fi

  log_step "Installing critical cargo tools..."

  local tools=(
    "cargo-deny:cargo-deny"
    "cargo-shear:cargo-shear"
    "cargo-nextest:cargo-nextest"
    "sccache:sccache"
    "commitlint-rs:commitlint"
    "cargo-watch:cargo-watch"
  )

  for tool_spec in "${tools[@]}"; do
    local crate="${tool_spec%%:*}"
    local bin="${tool_spec##*:}"

    if has_cmd "$bin"; then
      log_success "$bin already installed"
    else
      log_warning "Installing $crate..."
      if cargo install "$crate" --quiet; then
        log_success "$crate installed"

        # Configure sccache if just installed
        if [ "$bin" = "sccache" ]; then
          mkdir -p .cargo
          if ! grep -q 'rustc-wrapper = "sccache"' .cargo/config.toml 2>/dev/null; then
            if grep -q '^\[build\]' .cargo/config.toml 2>/dev/null; then
              sed -i.bak '/^\[build\]/a\
rustc-wrapper = "sccache"
' .cargo/config.toml && rm .cargo/config.toml.bak
            else
              echo "" >> .cargo/config.toml
              echo "[build]" >> .cargo/config.toml
              echo 'rustc-wrapper = "sccache"' >> .cargo/config.toml
            fi
            log_success "sccache configured in .cargo/config.toml"
          fi
        fi
      else
        warn "Failed to install $crate"
      fi
    fi
  done
}

# ============================================================================
# Go Tools (Conductor-specific)
# ============================================================================
check_go_tools() {
  if [ "$AGENT_SKIP_GO_TOOLS" = "1" ]; then
    return
  fi

  log_step "Checking Go-based linting tools..."

  if has_cmd yamlfmt; then
    log_success "yamlfmt found: $(which yamlfmt)"
  else
    log_warning "yamlfmt not found (used by pre-commit hooks)"
    echo "         Install: go install github.com/google/yamlfmt/cmd/yamlfmt@v0.10.0"
  fi

  if has_cmd actionlint; then
    log_success "actionlint found: $(which actionlint)"
  else
    log_warning "actionlint not found (used by pre-commit hooks)"
    echo "         Install: go install github.com/rhysd/actionlint/cmd/actionlint@latest"
  fi

  if has_cmd shellcheck; then
    log_success "shellcheck found (enhances actionlint)"
  else
    log_warning "shellcheck not found (optional, enhances actionlint diagnostics)"
  fi
}

# ============================================================================
# Node.js Tooling
# ============================================================================
ensure_corepack() {
  if has_cmd corepack; then
    corepack enable >/dev/null 2>&1 || true
    return
  fi
  if has_cmd npm; then
    log "Installing corepack globally"
    npm install -g corepack >/dev/null 2>&1 || warn "corepack install failed"
    corepack enable >/dev/null 2>&1 || true
  else
    warn "npm missing; skipping corepack"
  fi
}

install_global_node_tools() {
  if has_cmd npm; then
    log "Installing npm CLIs (prettier@${PRETTIER_VERSION}, markdownlint-cli2@${MARKDOWNLINT_VERSION})"
    npm install -g \
      "prettier@${PRETTIER_VERSION}" \
      "markdownlint-cli2@${MARKDOWNLINT_VERSION}" \
      "markdownlint-cli2-formatter-default@${MARKDOWNLINT_FORMATTER_VERSION}" \
      >/dev/null 2>&1 || warn "npm global install failed"
  elif has_cmd bun; then
    log "Installing CLIs with bun"
    bun add --global \
      "prettier@${PRETTIER_VERSION}" \
      "markdownlint-cli2@${MARKDOWNLINT_VERSION}" \
      "markdownlint-cli2-formatter-default@${MARKDOWNLINT_FORMATTER_VERSION}" \
      >/dev/null 2>&1 || warn "bun global install failed"
  else
    warn "No npm/bun found; skipping formatter installs"
  fi
}

check_node_tools() {
  log_step "Checking markdown and document formatting tools..."

  if has_cmd markdownlint-cli2; then
    log_success "markdownlint-cli2 found: $(which markdownlint-cli2)"
  elif has_cmd npx; then
    log_success "npx available (will use as markdownlint-cli2 fallback)"
  elif has_cmd bunx; then
    log_success "bunx available (will use as markdownlint-cli2 fallback)"
  else
    log_warning "No markdown linter found (markdownlint-cli2, npx, or bunx)"
    echo "         Install: npm install -g markdownlint-cli2 markdownlint-cli2-formatter-default"
  fi

  if has_cmd prettier; then
    log_success "prettier found: $(which prettier)"
  elif has_cmd npx; then
    log_success "npx available (will use as prettier fallback)"
  elif has_cmd bunx; then
    log_success "bunx available (will use as prettier fallback)"
  else
    log_warning "No prettier found (prettier, npx, or bunx)"
    echo "         Install: npm install -g prettier"
  fi
}

ensure_bun() {
  if has_cmd bun; then
    if [ "$AGENT_TYPE" = "conductor" ]; then
      log_success "bun found: $(which bun)"
    fi
    return
  fi
  if [ "${AGENT_INSTALL_BUN}" = "1" ]; then
    log "Installing Bun runtime"
    curl -fsSL https://bun.sh/install | bash >/dev/null 2>&1 || warn "Bun install failed"
    export PATH="$HOME/.bun/bin:$PATH"
  fi
}

# ============================================================================
# Lefthook Installation
# ============================================================================
ensure_lefthook() {
  if has_cmd lefthook; then
    return
  fi

  log "Installing lefthook"

  # Try npm first (most universal - single binary, no dependencies, cross-platform)
  if has_cmd npm; then
    if npm install -g lefthook >/dev/null 2>&1; then
      log "lefthook installed via npm"
      return
    fi
  fi

  # Try Go second (available in many dev environments)
  if has_cmd go; then
    if go install github.com/evilmartians/lefthook@latest >/dev/null 2>&1; then
      export PATH="$HOME/go/bin:$PATH"
      log "lefthook installed via Go"
      return
    fi
  fi

  # Try Homebrew third (macOS specific)
  if has_cmd brew; then
    if brew install lefthook >/dev/null 2>&1; then
      log "lefthook installed via Homebrew"
      return
    fi
  fi

  warn "Could not install lefthook via npm, Go, or Homebrew"
  warn "Manual installation required: https://lefthook.dev/installation/"
}

# ============================================================================
# Git Hooks Setup
# ============================================================================
setup_hooks() {
  if [ "$AGENT_SKIP_HOOKS" = "1" ]; then
    log "Skipping git hooks setup (AGENT_SKIP_HOOKS=1)"
    return
  fi

  ensure_lefthook

  if [ "$AGENT_SKIP_BOOTSTRAP_FAST" = "1" ]; then
    log "Running lefthook install"
    if has_cmd lefthook; then
      lefthook install || warn "lefthook install failed"
    else
      warn "lefthook not available; skipping git hooks setup"
    fi
  else
    if [ -x "$REPO_ROOT/scripts/bootstrap-fast.sh" ]; then
      log "Running scripts/bootstrap-fast.sh"
      "$REPO_ROOT/scripts/bootstrap-fast.sh"
    else
      log "Running lefthook install"
      if has_cmd lefthook; then
        lefthook install || warn "lefthook install failed"
      else
        warn "lefthook not available; skipping git hooks setup"
      fi
    fi
  fi
}

# ============================================================================
# Documentation Formatting
# ============================================================================
run_fmt_docs() {
  if [ -x "$REPO_ROOT/scripts/fmt-docs.sh" ]; then
    log "Formatting docs"
    "$REPO_ROOT/scripts/fmt-docs.sh" || warn "fmt-docs encountered issues"
  fi
}

# ============================================================================
# Validation (Conductor-specific)
# ============================================================================
run_validation() {
  if [ "$AGENT_TYPE" != "conductor" ]; then
    return
  fi

  log_step "Running validation checks..."

  # Run a quick test to ensure everything works
  TEST_OUTPUT=$(cargo test --workspace --lib 2>&1) || true
  if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    log_success "Library tests passed"
  else
    log_warning "Some library tests failed (check with: cargo test)"
  fi

  # Check formatting
  if cargo fmt --all --check >/dev/null 2>&1; then
    log_success "Code formatting is correct"
  else
    log_warning "Code needs formatting (run: cargo fmt --all)"
  fi
}

# ============================================================================
# Main Execution
# ============================================================================
main() {
  echo "🔧 Setting up blz workspace for $AGENT_TYPE..."
  echo ""

  # Verify workspace structure
  if [ ! -f "Cargo.toml" ]; then
    fail "Cargo.toml not found. Are we in the workspace root?"
  fi

  # Execute setup steps based on agent type
  ensure_git_state
  ensure_rust_toolchain
  cargo_bootstrap

  # Conductor-specific steps
  if [ "$AGENT_TYPE" = "conductor" ]; then
    check_go_tools
    check_node_tools
    install_cargo_tools
    ensure_bun
  else
    # Factory/Devin/Codex steps
    ensure_bun
    ensure_corepack
    install_global_node_tools
  fi

  setup_hooks
  run_fmt_docs
  run_validation

  echo ""
  log "$AGENT_TYPE agent setup complete"

  if [ "$AGENT_TYPE" = "conductor" ]; then
    echo ""
    echo "💡 Next steps:"
    echo "   • Click 'Run' button to start auto-reload development"
    echo "   • Run './scripts/agent-check.sh' for Rust diagnostics"
    echo "   • Run 'cargo test' to run all tests"
    echo "   • Check 'just --list' or 'make help' for available commands"
  fi
}

main "$@"
