#!/usr/bin/env bash
# Manage git hook bypass for emergency pushes
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
BYPASS_FILE="$REPO_ROOT/.hooks/allow-strict-bypass"

usage() {
  cat <<EOF
Usage: $0 <command> [options]

Manage git hook bypass for emergency situations.

Commands:
  enable [--force]    Enable bypass (creates .hooks/allow-strict-bypass)
                      Use --force to skip confirmation
  disable             Disable bypass (removes .hooks/allow-strict-bypass)
  status              Check current bypass status

Examples:
  # Enable bypass with confirmation
  $0 enable

  # Enable bypass without confirmation (for scripts)
  $0 enable --force

  # Disable bypass
  $0 disable

  # Check status
  $0 status

Note: The bypass file is git-ignored to prevent accidental commits.
EOF
}

enable_bypass() {
  local force=false
  if [[ "${1:-}" == "--force" ]]; then
    force=true
  fi

  if [[ -f "$BYPASS_FILE" ]]; then
    echo "✓ Bypass already enabled"
    exit 0
  fi

  if [[ "$force" != "true" ]]; then
    echo "⚠️  Warning: Enabling hook bypass will skip clippy and tests on push."
    echo "This should only be used in emergency situations."
    echo ""
    read -p "Are you sure you want to enable bypass? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      echo "Cancelled."
      exit 1
    fi
  fi

  mkdir -p "$(dirname "$BYPASS_FILE")"
  cat > "$BYPASS_FILE" <<EOF
# Git hook bypass enabled
# Created: $(date)
# 
# To disable, run: scripts/hooks-bypass.sh disable
EOF

  echo "✓ Hook bypass enabled"
  echo "  Pre-push clippy and tests will be skipped"
  echo "  Remember to disable after your emergency push:"
  echo "    scripts/hooks-bypass.sh disable"
}

disable_bypass() {
  if [[ ! -f "$BYPASS_FILE" ]]; then
    echo "✓ Bypass already disabled"
    exit 0
  fi

  rm "$BYPASS_FILE"
  echo "✓ Hook bypass disabled"
  echo "  Pre-push clippy and tests will run normally"
}

status() {
  if [[ -f "$BYPASS_FILE" ]]; then
    echo "⚠️  Bypass is ENABLED"
    echo "  File: $BYPASS_FILE"
    echo "  Created: $(stat -c %y "$BYPASS_FILE" 2>/dev/null || stat -f "%Sm" "$BYPASS_FILE")"
    echo ""
    echo "To disable: scripts/hooks-bypass.sh disable"
    exit 1  # Exit 1 to indicate bypass is active
  else
    echo "✓ Bypass is disabled (normal hook behavior)"
    exit 0
  fi
}

# Main
case "${1:-}" in
  enable)
    enable_bypass "${2:-}"
    ;;
  disable)
    disable_bypass
    ;;
  status)
    status
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    echo "Error: Unknown command '${1:-}'"
    echo ""
    usage
    exit 1
    ;;
esac
