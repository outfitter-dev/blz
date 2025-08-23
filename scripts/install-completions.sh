#!/usr/bin/env bash

# Auto-install completions for multiple shells
# This script can be called after installing/updating the binary

set -e

BINARY="${1:-blz}"
BINARY_PATH="$(which $BINARY 2>/dev/null || echo "$HOME/.cargo/bin/$BINARY")"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: $BINARY not found in PATH or ~/.cargo/bin"
    exit 1
fi

echo "Installing shell completions for $BINARY..."

# Fish
if [ -d "$HOME/.config/fish/completions" ]; then
    "$BINARY_PATH" completions fish > "$HOME/.config/fish/completions/$BINARY.fish"
    echo "✅ Fish completions installed"
fi

# Bash
if [ -d "$HOME/.local/share/bash-completion/completions" ]; then
    "$BINARY_PATH" completions bash > "$HOME/.local/share/bash-completion/completions/$BINARY"
    echo "✅ Bash completions installed"
elif [ -d "/usr/local/etc/bash_completion.d" ]; then
    "$BINARY_PATH" completions bash > "/usr/local/etc/bash_completion.d/$BINARY"
    echo "✅ Bash completions installed (system-wide)"
fi

# Zsh
if [ -d "$HOME/.zsh/completions" ]; then
    "$BINARY_PATH" completions zsh > "$HOME/.zsh/completions/_$BINARY"
    echo "✅ Zsh completions installed"
elif [ -d "/usr/local/share/zsh/site-functions" ]; then
    "$BINARY_PATH" completions zsh > "/usr/local/share/zsh/site-functions/_$BINARY"
    echo "✅ Zsh completions installed (system-wide)"
fi

echo ""
echo "Completions installed! Reload your shell or run:"
echo "  Fish: source ~/.config/fish/config.fish"
echo "  Bash: source ~/.bashrc"
echo "  Zsh:  exec zsh"