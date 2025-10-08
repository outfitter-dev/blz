#!/usr/bin/env sh
# Manual installer for the optional blz-dev binary.
#
# Usage:
#   ./install-dev.sh [extra cargo install flags]
# Example:
#   ./install-dev.sh --root "$HOME/.local/share/blz-dev"
#
# The script leaves the primary `blz` installation untouched and installs only the
# dev-profile binary (`blz-dev`). Pass through any additional flags to `cargo install`
# such as `--root`, `--locked`, or `--force`.

set -eu

if ! command -v cargo >/dev/null 2>&1; then
    printf 'cargo not found on PATH; please install Rust toolchain first.\n' >&2
    exit 1
fi

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

exec cargo install \
    --path "$REPO_DIR/crates/blz-cli" \
    --bin blz-dev \
    --features dev-profile \
    "$@"
