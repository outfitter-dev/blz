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

# Elvish
if [ -d "$HOME/.elvish/lib" ]; then
    "$BINARY_PATH" completions elvish > "$HOME/.elvish/lib/$BINARY.elv"
    echo "✅ Elvish completions installed"
fi

# PowerShell (pwsh)
if command -v pwsh >/dev/null 2>&1; then
    PROFILE_PATH="$(pwsh -NoProfile -Command '$PROFILE' 2>/dev/null || true)"
    PROFILE_PATH="$(printf '%s' "$PROFILE_PATH" | tr -d '\r')"
    if [ -n "$PROFILE_PATH" ]; then
        PROFILE_DIR="$(dirname "$PROFILE_PATH")"
        COMPLETIONS_FILE="$PROFILE_DIR/${BINARY}-completions.ps1"
        mkdir -p "$PROFILE_DIR"
        "$BINARY_PATH" completions powershell > "$COMPLETIONS_FILE"
        if [ ! -f "$PROFILE_PATH" ] || ! grep -Fq "$COMPLETIONS_FILE" "$PROFILE_PATH"; then
            {
                echo ""
                echo "# Load ${BINARY} completions"
                echo "if (Test-Path \"$COMPLETIONS_FILE\") { . \"$COMPLETIONS_FILE\" }"
            } >> "$PROFILE_PATH"
        fi
        echo "✅ PowerShell completions installed"
    fi
fi

echo ""
echo "Completions installed! Reload your shell or run:"
echo "  Fish: source ~/.config/fish/config.fish"
echo "  Bash: source ~/.bashrc"
echo "  Zsh:  exec zsh"
echo "  Elvish: exec elvish"
echo "  PowerShell: . \$PROFILE"
