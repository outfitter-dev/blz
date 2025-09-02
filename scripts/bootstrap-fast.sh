#!/usr/bin/env bash
set -euo pipefail

echo "🚀 blz bootstrap (fast hooks + tools)"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

# Helpers
has() { command -v "$1" >/dev/null 2>&1; }
note() { echo -e "\033[0;34mℹ\033[0m $*"; }
ok() { echo -e "\033[0;32m✓\033[0m $*"; }
warn() { echo -e "\033[1;33m⚠\033[0m $*"; }
die() { echo -e "\033[0;31m✗\033[0m $*"; exit 1; }

# 1) Ensure lefthook is installed and hooks set up
if ! has lefthook; then
  warn "lefthook not found. Installing via cargo..."
  cargo install lefthook || die "Failed to install lefthook"
fi
lefthook install || die "lefthook install failed"
ok "lefthook hooks installed"

# 2) Ensure speed tools: nextest and sccache
if ! has cargo-nextest; then
  note "Installing cargo-nextest for fast tests..."
  cargo install cargo-nextest || die "Failed to install cargo-nextest"
  ok "cargo-nextest installed"
else
  ok "cargo-nextest present"
fi

if ! has sccache; then
  note "Installing sccache to speed builds..."
  cargo install sccache || die "Failed to install sccache"
  ok "sccache installed"
else
  ok "sccache present"
fi

# 2b) Ensure commitlint-rs for commit message linting
if ! has commitlint; then
  note "Installing commitlint-rs for commit message linting..."
  cargo install commitlint-rs || warn "Failed to install commitlint-rs (you can install manually: cargo install commitlint-rs)"
else
  ok "commitlint present"
fi

# 3) Configure Cargo to use sccache (local-only)
CARGO_CFG=".cargo/config.toml"
mkdir -p .cargo
if ! grep -q "^rustc-wrapper\s*=\s*\"sccache\"" "$CARGO_CFG" 2>/dev/null; then
  note "Configuring Cargo to use sccache (build.rustc-wrapper)"
  # Add or extend [build] section
  if grep -q "^\[build\]" "$CARGO_CFG" 2>/dev/null; then
    # Append to existing build section (avoid duplicates)
    awk '
      BEGIN{printed=0}
      {print}
      /^\[build\]/{inbuild=1; next}
    ' "$CARGO_CFG" > "$CARGO_CFG.tmp" && mv "$CARGO_CFG.tmp" "$CARGO_CFG"
  else
    echo "[build]" >> "$CARGO_CFG"
  fi
  echo 'rustc-wrapper = "sccache"' >> "$CARGO_CFG"
  ok "sccache enabled in .cargo/config.toml"
else
  ok "sccache already enabled in .cargo/config.toml"
fi

# 4) Ensure rustfmt & clippy available (best-effort)
if has rustup; then
  note "Ensuring rustfmt + clippy via rustup..."
  rustup component add rustfmt clippy || warn "Could not add components via rustup (non-fatal)"
else
  warn "rustup not found; assuming rustfmt/clippy available in toolchain"
fi

# 5) Prime hooks with a quick run (non-blocking)
note "Running pre-commit once to prime caches..."
lefthook run pre-commit || true

echo
ok "Bootstrap complete. Local pushes will run strict Clippy + tests."
echo "Tip: to temporarily bypass strict push checks, run: scripts/hooks-bypass.sh enable --force"
echo "      (then remove with: scripts/hooks-bypass.sh disable)"
