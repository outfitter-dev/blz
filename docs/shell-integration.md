<!-- TODO #5: Update this doc as we improve Zsh support (track improvements in issue #5) -->
# Shell Integration

Complete guide to shell completions and integration for `blz`.

## Quick Setup

### Zsh

```zsh
# Ensure completions directory exists
mkdir -p ~/.zsh/completions

# Generate and install completions
blz completions zsh > ~/.zsh/completions/_blz

# Add to .zshrc if not already present
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload
source ~/.zshrc
```

### Fish Shell

```fish
# Generate and install completions
blz completions fish > ~/.config/fish/completions/blz.fish

# Reload (or restart shell)
source ~/.config/fish/config.fish
```

### Bash

```bash
# Generate and install completions
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Reload (or restart shell)
source ~/.bashrc
```

## Features by Shell

### Fish (Most Complete)

Fish users get the best experience with dynamic completions:

```fish
# Static completions for commands and options
blz <TAB>                    # Shows all commands
blz search --<TAB>           # Shows all options for search

# Dynamic alias completions
blz search --alias <TAB>     # Shows: bun, node, test (your actual sources!)
blz get <TAB>                # Completes with your indexed aliases
blz update <TAB>             # Shows available sources to update
# blz diff <TAB>             # (Coming soon)

# Descriptions for everything
blz <TAB>
  add         (Add a new llms.txt source)
  search      (Search with 6ms latency!)
  get         (Get exact line ranges)
  completions (Generate shell completions)
```

### Bash

Standard completions for commands and options:

```bash
blz <TAB><TAB>           # Shows commands
blz search --<TAB><TAB>  # Shows options
```

### Zsh

Similar to Bash with better formatting:

```zsh
blz <TAB>                # Shows commands with descriptions
blz search --<TAB>       # Shows options
```

## Dynamic Completions (Fish)

### How It Works

Fish completions are enhanced with runtime data:

```fish
# This function queries your actual indexed sources
function __fish_blz_complete_aliases
    blz list --format json 2>/dev/null | python3 -c "
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

1. **Aliases** - Your actual indexed sources
2. **Commands** - All available subcommands
3. **Options** - Flags and parameters
4. **Values** - Format options (json/pretty)

## Auto-Updating Completions

### Install Script

Use the provided script to update all shells at once:

```bash
# After installing/updating blz
./scripts/install-completions.sh
```

This script:

- Detects installed shells
- Generates completions for each
- Installs in the right location
- Works on macOS and Linux

### Manual Update

When you update the `blz` binary:

```bash
# Regenerate for your shell
blz completions fish > ~/.config/fish/completions/blz.fish
blz completions bash > ~/.local/share/bash-completion/completions/blz
blz completions zsh > ~/.zsh/completions/_blz
```

### Fish Auto-Update Function

Add to your `config.fish` for automatic updates:

```fish
# Auto-update blz completions when binary changes
function __update_blz_completions --on-event fish_prompt
    set -l blz_bin (which blz 2>/dev/null)
    if test -z "$blz_bin"
        return
    end

    set -l completion_file "$HOME/.config/fish/completions/blz.fish"

    # Update if binary is newer than completions
    if not test -f "$completion_file"; or test "$blz_bin" -nt "$completion_file"
        blz completions fish > "$completion_file" 2>/dev/null
    end
end
```

## Advanced Fish Features

### Custom Completions

Add your own completions to `~/.config/fish/completions/blz.fish`:

```fish
# Complete with markdown files for add command
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "(ls *.md *.txt 2>/dev/null)"

# Add common URLs
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "bun" -d "https://bun.sh/llms.txt"
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "node" -d "https://nodejs.org/llms.txt"
```

### Abbreviations

Speed up common commands:

```fish
# Add to config.fish
abbr -a cs 'blz search'
abbr -a cg 'blz get'
abbr -a ca 'blz add'
abbr -a cl 'blz list'

# Usage
cs test         # Expands to: blz search test
cg bun --lines  # Expands to: blz get bun --lines
```

### Functions

Create helpful functions:

```fish
# Search and display best result
function blz-best
    set -l query $argv
    set -l result (blz search "$query" --limit 1 --format json | jq -r '.hits[0]')

    if test "$result" != "null"
        set -l alias (echo $result | jq -r '.alias')
        set -l lines (echo $result | jq -r '.lines')
        blz get $alias --lines $lines
    else
        echo "No results for: $query"
    end
end

# Add with shorthand
function blz-add-quick
    switch $argv[1]
        case bun
            blz add bun https://bun.sh/llms.txt
        case node
            blz add node https://nodejs.org/llms.txt
        case deno
            blz add deno https://deno.land/llms.txt
        case '*'
            echo "Unknown source: $argv[1]"
    end
end
```

## Integration Examples

### Fuzzy Search with fzf

```bash
# Fish/Bash/Zsh
function blz-fzf
    blz search "$1" --format json | \
    jq -r '.hits[] | "\(.alias):\(.lines) \(.heading_path | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs -I{} sh -c "blz get {}"'
end
```

### Alfred/Raycast Integration

Create a workflow script:

```bash
#!/bin/bash
# For Alfred/Raycast

query="$1"
results=$(blz search "$query" --format json)

echo "$results" | jq -r '.hits[] | {
    title: .heading_path | join(" > "),
    subtitle: "\(.alias) L\(.lines)",
    arg: "\(.alias) --lines \(.lines)"
}'
```

### Vim Integration

```vim
" Search blz from Vim
command! -nargs=1 BlzSearch
    \ :r!blz search "<args>" --limit 3

" Get specific lines
command! -nargs=+ BlzGet
    \ :r!blz get <args>
```

## Troubleshooting

### Completions Not Working

#### Fish

```fish
# Check if file exists
ls ~/.config/fish/completions/blz.fish

# Regenerate
blz completions fish > ~/.config/fish/completions/blz.fish

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
blz completions bash > ~/.local/share/bash-completion/completions/blz
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
blz list --format json

# If this works, completions should work
# If not, check that you have sources:
blz list
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
alias cs='blz search'
alias cg='blz get'
alias ca='blz add'

# Fish
alias cs 'blz search'
alias cg 'blz get'
alias ca 'blz add'
```

### Search History

Fish automatically provides history:

```fish
blz search <UP>  # Shows previous searches
```

For Bash/Zsh, use Ctrl+R for reverse search.

### Batch Operations

```bash
# Add multiple sources
for url in bun.sh/llms.txt deno.land/llms.txt; do
    name=$(echo $url | cut -d'.' -f1)
    blz add "$name" "https://$url"
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

1. Edit `crates/blz-cli/src/main.rs`
2. Update the `Commands` enum with better descriptions
3. Rebuild: `cargo build --release`
4. Test: `blz completions <shell>`

See [CONTRIBUTING.md](../CONTRIBUTING.md) for more details.
