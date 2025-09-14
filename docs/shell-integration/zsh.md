# Zsh Completions

Setup guide for Zsh shell completions.

## Installation

### Standard Setup

```zsh
# Create completions directory
mkdir -p ~/.zsh/completions

# Generate completions
blz completions zsh > ~/.zsh/completions/_blz

# Add to ~/.zshrc
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit

# Reload
source ~/.zshrc
```

### Oh My Zsh

```zsh
# Install to OMZ custom folder
blz completions zsh > ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/blz/_blz

# Reload
omz reload
```

### System-wide

```zsh
# macOS/Linux
sudo blz completions zsh > /usr/local/share/zsh/site-functions/_blz

# Rebuild cache
rm -f ~/.zcompdump && compinit
```

## Features

- Command completion with descriptions
- Option completion
- Basic argument completion

## Dynamic Alias Completion

Augment the static `_blz` script with live alias suggestions (canonical + metadata aliases) by sourcing the dynamic helper:

```zsh
# Add after compinit in ~/.zshrc
source /path/to/blz/scripts/blz-dynamic-completions.zsh
```

What it adds:

- `--alias`/`-s`/`--source` dynamic values for `blz search`
- Positional alias completion for `blz get`, `blz update`, `blz remove`, `blz anchors`, and `blz anchor list|get`
- Anchor value completion for `blz anchor get <alias> <anchor>`

It reads from `blz list --output json` and merges canonical + metadata aliases. Falls back to the static `_blz` for everything else.

## Configuration

### Completion Styles

Add to `~/.zshrc`:

```zsh
# Better completion display
zstyle ':completion:*' menu select
zstyle ':completion:*' group-name ''
zstyle ':completion:*:descriptions' format '%B%d%b'

# Case-insensitive completion
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'

# Colorized completions
zstyle ':completion:*' list-colors ${(s.:.)LS_COLORS}
```

## Aliases & Functions

```zsh
# Quick aliases
alias bs='blz search'
alias bg='blz get'
alias ba='blz add'
alias bl='blz list'
alias bu='blz update --all'

# Search function with fzf
blz-fzf() {
    local query="$*"
    blz search "$query" -o json | \
    jq -r '.results[] | "\(.alias):\(.lines) \(.headingPath | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs blz get'
}

# Quick search and display
blz-quick() {
    local result=$(blz search "$*" --limit 1 -o json | jq -r '.results[0] | "\(.alias) \(.lines)"')
    if [[ -n "$result" ]]; then
        blz get $result
    else
        echo "No results for: $*"
    fi
}
```

## Widget Integration

Add to `~/.zshrc` for interactive search:

```zsh
# Ctrl+B for blz search
blz-search-widget() {
    local selected=$(blz list -o json | jq -r '.[]' | fzf)
    if [[ -n "$selected" ]]; then
        BUFFER="blz search -s $selected "
        CURSOR=$#BUFFER
    fi
    zle redisplay
}
zle -N blz-search-widget
bindkey '^b' blz-search-widget
```

## Troubleshooting

### Completions not loading

```zsh
# Check fpath
echo $fpath | tr ' ' '\n' | grep -E '(completion|function)'

# Verify file exists
ls ~/.zsh/completions/_blz

# Rebuild completion cache
rm -f ~/.zcompdump*
autoload -Uz compinit && compinit
```

### Permission issues

```zsh
# Fix permissions
chmod 755 ~/.zsh/completions
chmod 644 ~/.zsh/completions/_blz
```

### Debugging completions

```zsh
# Enable completion debugging
zstyle ':completion:*' verbose yes
zstyle ':completion:*:descriptions' format 'Completing %d'
zstyle ':completion:*:warnings' format 'No matches for: %d'
```

## Advanced Setup

For detailed configuration including custom functions and troubleshooting, see [zsh-setup.md](./zsh-setup.md).
