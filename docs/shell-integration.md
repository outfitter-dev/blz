# Shell Integration

Complete guide to shell completions and integration for @outfitter/cache.

## Quick Setup

### Fish Shell 🐟

```fish
# Generate and install completions
cache completions fish > ~/.config/fish/completions/cache.fish

# Reload (or restart shell)
source ~/.config/fish/config.fish
```

### Bash

```bash
# Generate and install completions
cache completions bash > ~/.local/share/bash-completion/completions/cache

# Reload (or restart shell)
source ~/.bashrc
```

### Zsh

```zsh
# Ensure completions directory exists
mkdir -p ~/.zsh/completions

# Generate and install completions
cache completions zsh > ~/.zsh/completions/_cache

# Add to .zshrc if not already present
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload
source ~/.zshrc
```

## Features by Shell

### Fish (Most Complete) 🌟

Fish users get the best experience with dynamic completions:

```fish
# Static completions for commands and options
cache <TAB>                    # Shows all commands
cache search --<TAB>           # Shows all options for search

# Dynamic alias completions
cache search --alias <TAB>     # Shows: bun, node, test (your actual sources!)
cache get <TAB>                # Completes with your cached aliases
cache update <TAB>             # Shows available sources to update
cache diff <TAB>               # Shows sources you can diff

# Descriptions for everything
cache <TAB>
  add         (Add a new llms.txt source)
  search      (Search with 6ms latency!)
  get         (Get exact line ranges)
  completions (Generate shell completions)
```

### Bash

Standard completions for commands and options:

```bash
cache <TAB><TAB>           # Shows commands
cache search --<TAB><TAB>  # Shows options
```

### Zsh

Similar to Bash with better formatting:

```zsh
cache <TAB>                # Shows commands with descriptions
cache search --<TAB>       # Shows options
```

## Dynamic Completions (Fish)

### How It Works

Fish completions are enhanced with runtime data:

```fish
# This function queries your actual cached sources
function __fish_cache_complete_aliases
    cache sources --format json 2>/dev/null | python3 -c "
import json, sys
try:
    sources = json.load(sys.stdin)
    for s in sources:
        print(s)
except:
    pass
"
end
```

### What Gets Completed

1. **Aliases** - Your actual cached sources
2. **Commands** - All available subcommands
3. **Options** - Flags and parameters
4. **Values** - Format options (json/pretty)

## Auto-Updating Completions

### Install Script

Use the provided script to update all shells at once:

```bash
# After installing/updating cache
./scripts/install-completions.sh
```

This script:
- Detects installed shells
- Generates completions for each
- Installs in the right location
- Works on macOS and Linux

### Manual Update

When you update the `cache` binary:

```bash
# Regenerate for your shell
cache completions fish > ~/.config/fish/completions/cache.fish
cache completions bash > ~/.local/share/bash-completion/completions/cache
cache completions zsh > ~/.zsh/completions/_cache
```

### Fish Auto-Update Function

Add to your `config.fish` for automatic updates:

```fish
# Auto-update cache completions when binary changes
function __update_cache_completions --on-event fish_prompt
    set -l cache_bin (which cache 2>/dev/null)
    if test -z "$cache_bin"
        return
    end
    
    set -l completion_file "$HOME/.config/fish/completions/cache.fish"
    
    # Update if binary is newer than completions
    if not test -f "$completion_file"; or test "$cache_bin" -nt "$completion_file"
        cache completions fish > "$completion_file" 2>/dev/null
    end
end
```

## Advanced Fish Features

### Custom Completions

Add your own completions to `~/.config/fish/completions/cache.fish`:

```fish
# Complete with markdown files for add command
complete -c cache -n "__fish_seen_subcommand_from add" \
    -a "(ls *.md *.txt 2>/dev/null)"

# Add common URLs
complete -c cache -n "__fish_seen_subcommand_from add" \
    -a "bun" -d "https://bun.sh/llms.txt"
complete -c cache -n "__fish_seen_subcommand_from add" \
    -a "node" -d "https://nodejs.org/llms.txt"
```

### Abbreviations

Speed up common commands:

```fish
# Add to config.fish
abbr -a cs 'cache search'
abbr -a cg 'cache get'
abbr -a ca 'cache add'
abbr -a cl 'cache sources'

# Usage
cs test         # Expands to: cache search test
cg bun --lines  # Expands to: cache get bun --lines
```

### Functions

Create helpful functions:

```fish
# Search and display best result
function cache-best
    set -l query $argv
    set -l result (cache search "$query" --limit 1 --format json | jq -r '.hits[0]')
    
    if test "$result" != "null"
        set -l alias (echo $result | jq -r '.alias')
        set -l lines (echo $result | jq -r '.lines')
        cache get $alias --lines $lines
    else
        echo "No results for: $query"
    end
end

# Add with shorthand
function cache-add-quick
    switch $argv[1]
        case bun
            cache add bun https://bun.sh/llms.txt
        case node
            cache add node https://nodejs.org/llms.txt
        case deno
            cache add deno https://deno.land/llms.txt
        case '*'
            echo "Unknown source: $argv[1]"
    end
end
```

## Integration Examples

### Fuzzy Search with fzf

```bash
# Fish/Bash/Zsh
function cache-fzf
    cache search "$1" --format json | \
    jq -r '.hits[] | "\(.alias):\(.lines) \(.heading_path | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs -I{} sh -c "cache get {}"'
end
```

### Alfred/Raycast Integration

Create a workflow script:

```bash
#!/bin/bash
# For Alfred/Raycast

query="$1"
results=$(cache search "$query" --format json)

echo "$results" | jq -r '.hits[] | {
    title: .heading_path | join(" > "),
    subtitle: "\(.alias) L\(.lines)",
    arg: "\(.alias) --lines \(.lines)"
}'
```

### Vim Integration

```vim
" Search cache from Vim
command! -nargs=1 CacheSearch 
    \ :r!cache search "<args>" --limit 3

" Get specific lines
command! -nargs=+ CacheGet
    \ :r!cache get <args>
```

## Troubleshooting

### Completions Not Working

#### Fish
```fish
# Check if file exists
ls ~/.config/fish/completions/cache.fish

# Regenerate
cache completions fish > ~/.config/fish/completions/cache.fish

# Reload
source ~/.config/fish/config.fish
```

#### Bash
```bash
# Check bash-completion is installed
type _init_completion

# If not, install it:
# macOS: brew install bash-completion
# Linux: apt/yum install bash-completion

# Regenerate completions
cache completions bash > ~/.local/share/bash-completion/completions/cache
```

#### Zsh
```zsh
# Check fpath includes completions dir
echo $fpath

# Rebuild completion cache
rm -f ~/.zcompdump
compinit
```

### Dynamic Completions Not Updating (Fish)

The dynamic completions query live data:

```fish
# Test the query function
cache sources --format json

# If this works, completions should work
# If not, check that you have sources:
cache sources
```

## Platform-Specific Notes

### macOS

Bash on macOS requires homebrew's bash-completion:

```bash
brew install bash-completion

# Add to .bash_profile
[[ -r "/usr/local/etc/profile.d/bash_completion.sh" ]] && \
    . "/usr/local/etc/profile.d/bash_completion.sh"
```

### Linux

Most distributions include bash-completion:

```bash
# Debian/Ubuntu
sudo apt install bash-completion

# Fedora/RHEL
sudo dnf install bash-completion

# Arch
sudo pacman -S bash-completion
```

### Windows (WSL)

Works the same as Linux within WSL. For native Windows, use PowerShell completions (coming soon).

## Tips & Tricks

### Quick Aliases

Add to your shell config:

```bash
# Bash/Zsh
alias cs='cache search'
alias cg='cache get'
alias ca='cache add'

# Fish
alias cs 'cache search'
alias cg 'cache get'
alias ca 'cache add'
```

### Search History

Fish automatically provides history:
```fish
cache search <UP>  # Shows previous searches
```

For Bash/Zsh, use Ctrl+R for reverse search.

### Batch Operations

```bash
# Add multiple sources
for url in bun.sh/llms.txt deno.land/llms.txt; do
    name=$(echo $url | cut -d'.' -f1)
    cache add "$name" "https://$url"
done
```

## Future Enhancements

Planned improvements:
- PowerShell completions
- Nushell support
- More dynamic completions for Bash/Zsh
- Completion of search queries from history
- IDE integrations (VS Code, IntelliJ)

## Contributing

To improve completions:

1. Edit `crates/cache-cli/src/main.rs`
2. Update the `Commands` enum with better descriptions
3. Rebuild: `cargo build --release`
4. Test: `cache completions <shell>`

See [CONTRIBUTING.md](../CONTRIBUTING.md) for more details.