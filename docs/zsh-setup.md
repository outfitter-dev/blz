# Zsh Shell Integration for blz

This guide covers how to set up Zsh shell integration for `blz`, including completions and custom functions.

## Basic Completion Setup

Generate and install Zsh completions:

```bash
# Generate completions
blz completions zsh > ~/.zsh/completions/_blz

# Or to a standard location
blz completions zsh > /usr/local/share/zsh/site-functions/_blz
```

## Enable Completions

Add to your `~/.zshrc`:

```bash
# Add custom completion directory if not already configured
fpath=(~/.zsh/completions $fpath)

# Initialize completions
autoload -Uz compinit && compinit
```

## Reload Shell Configuration

```bash
# Reload Zsh configuration
source ~/.zshrc

# Or restart your terminal
```

## Testing Completions

After setup, test that completions work:

```bash
# Should show available commands
blz <TAB>

# Should complete aliases
blz search <TAB>

# Should show options
blz search --<TAB>
```

## Advanced: Custom Functions

Add helpful functions to your `~/.zshrc`:

```bash
# Quick search function
bzs() {
  if [ -z "$1" ]; then
    echo "Usage: bzs <query>"
    return 1
  fi
  blz search "$@"
}

# Search specific alias
bza() {
  if [ -z "$2" ]; then
    echo "Usage: bza <alias> <query>"
    return 1
  fi
  blz search "$2" --alias "$1"
}

# Update all sources with summary
bzupdate() {
  echo "Updating all blz sources..."
  blz update --all
}

# Quick add source
bzadd() {
  if [ -z "$2" ]; then
    echo "Usage: bzadd <alias> <url>"
    return 1
  fi
  blz add "$1" "$2"
}
```

## Troubleshooting

### Completions Not Working

1. **Check fpath**: Ensure your completions directory is in fpath:
   ```bash
   echo $fpath
   ```

2. **Rebuild completion cache**:
   ```bash
   rm -f ~/.zcompdump
   compinit
   ```

3. **Check completion file**: Ensure the completion file exists and is readable:
   ```bash
   ls -la ~/.zsh/completions/_blz
   ```

### Permission Issues

If you get permission errors:

```bash
# Fix permissions
chmod 755 ~/.zsh/completions
chmod 644 ~/.zsh/completions/_blz
```

### Completions Out of Date

Regenerate completions after updating blz:

```bash
blz completions zsh > ~/.zsh/completions/_blz
exec zsh  # Restart shell
```

## Oh My Zsh Users

If using Oh My Zsh, you can place completions in:

```bash
blz completions zsh > ~/.oh-my-zsh/completions/_blz
```

Then reload:

```bash
omz reload
```

## Tips

1. **Use aliases**: The completion system will automatically complete your indexed source aliases
2. **Combine with fzf**: For fuzzy finding through results:
   ```bash
   blz search "$1" | fzf
   ```
3. **Set up keybindings**: Add to `~/.zshrc`:
   ```bash
   # Ctrl+B for quick blz search
   bindkey -s '^B' 'blz search '
   ```

## See Also

- [Shell Integration Guide](./shell-integration.md) - General shell setup
- [CLI Documentation](./cli.md) - Complete command reference