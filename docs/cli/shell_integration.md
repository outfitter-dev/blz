# Shell Integration

Complete guide to shell completions and integration for BLZ.

## Table of Contents

- [Supported Shells](#supported-shells)
- [Quick Setup](#quick-setup)
- [Zsh](#zsh)
- [Fish](#fish)
- [Bash](#bash)
- [PowerShell](#powershell)
- [Elvish](#elvish)
- [Integration Examples](#integration-examples)
- [Auto-Updating Completions](#auto-updating-completions)
- [Troubleshooting](#troubleshooting)
- [Platform-Specific Notes](#platform-specific-notes)
- [Tips & Tricks](#tips--tricks)
- [Contributing](#contributing)

## Supported Shells

BLZ provides completions for:

- **Zsh** - Rich completions (default on macOS) with optional dynamic aliases
- **Fish** - Dynamic alias/anchor completions via helper script
- **Bash** - Standard completions with broad compatibility
- **PowerShell** - Windows and cross-platform support (static + optional dynamic aliases)
- **Elvish** - Modern shell with structured data support

## Quick Setup

### Zsh

```zsh
mkdir -p ~/.zsh/completions
blz completions zsh > ~/.zsh/completions/_blz
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
source ~/.zshrc
```

### Fish

```fish
blz completions fish > ~/.config/fish/completions/blz.fish
source ~/.config/fish/config.fish
```

### Bash

```bash
blz completions bash > ~/.local/share/bash-completion/completions/blz
source ~/.bashrc
```

### PowerShell

```powershell
$profileDir = Split-Path -Parent $PROFILE
$completionFile = Join-Path $profileDir "blz-completions.ps1"
blz completions powershell > $completionFile
if (-not (Select-String -Path $PROFILE -Pattern $completionFile -Quiet)) {
    Add-Content $PROFILE "if (Test-Path `"$completionFile`") { . `"$completionFile`" }"
}
. $PROFILE
```

### Elvish

```elvish
blz completions elvish > ~/.elvish/lib/blz.elv
echo 'use blz' >> ~/.elvish/rc.elv
exec elvish
```

## Zsh

Rich completions with descriptions and better formatting.

### Installation

#### Standard Setup

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

#### Oh My Zsh

```zsh
# Install to OMZ custom folder
blz completions zsh > ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/blz/_blz

# Reload
omz reload
```

#### System-wide

```zsh
# macOS/Linux
sudo blz completions zsh > /usr/local/share/zsh/site-functions/_blz

# Rebuild cache
rm -f ~/.zcompdump && compinit
```

### Features

- Command completion with descriptions
- Option completion
- Dynamic alias/anchor completion (with helper script)

### Dynamic Alias Completion

Augment the static `_blz` script with live alias suggestions (canonical + metadata aliases) by sourcing the dynamic helper:

```zsh
# Add after compinit in ~/.zshrc
source /path/to/blz/scripts/blz-dynamic-completions.zsh
```

What it adds:

- `--source`/`-s` dynamic values for `blz query` and `blz get`
- Positional alias completion for `blz query`, `blz get`, `blz sync`, `blz rm`, `blz diff`, `blz map`, and `blz anchor list|get`
- Anchor value completion for `blz anchor get <alias> <anchor>`

It reads from `blz list --json` and merges canonical + metadata aliases. Falls back to the static `_blz` for everything else.

### Usage

```zsh
blz <TAB>                # Shows commands with descriptions
blz query --<TAB>        # Shows options
blz query <TAB>          # Complete aliases (with dynamic helper)
```

### Configuration

#### Completion Styles

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

### Aliases & Functions

```zsh
# Quick aliases
alias bs='blz search'
alias bg='blz get'
alias ba='blz add'
alias bl='blz list'
alias bu='blz refresh --all'  # deprecated alias: blz update --all

# Search function with fzf
blz-fzf() {
    local query="$*"
    blz search "$query" -f json | \
    jq -r '.results[] | "\(.alias):\(.lines) \(.headingPath | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs blz get'
}

# Quick search and display
blz-quick() {
    local result=$(blz search "$*" --limit 1 -f json | jq -r '.results[0] | "\(.alias) \(.lines)"')
    if [[ -n "$result" ]]; then
        blz get $result
    else
        echo "No results for: $*"
    fi
}
```

### Widget Integration

Add to `~/.zshrc` for interactive search:

```zsh
# Ctrl+B for blz search
blz-search-widget() {
    local selected=$(blz list -f json | jq -r '.[]' | fzf)
    if [[ -n "$selected" ]]; then
        BUFFER="blz search -s $selected "
        CURSOR=$#BUFFER
    fi
    zle redisplay
}
zle -N blz-search-widget
bindkey '^b' blz-search-widget
```

### Troubleshooting

#### Completions not loading

```zsh
# Check fpath
echo $fpath | tr ' ' '\n' | grep -E '(completion|function)'

# Verify file exists
ls ~/.zsh/completions/_blz

# Rebuild completion cache
rm -f ~/.zcompdump*
autoload -Uz compinit && compinit
```

#### Permission issues

```zsh
# Fix permissions
chmod 755 ~/.zsh/completions
chmod 644 ~/.zsh/completions/_blz
```

#### Debugging completions

```zsh
# Enable completion debugging
zstyle ':completion:*' verbose yes
zstyle ':completion:*:descriptions' format 'Completing %d'
zstyle ':completion:*:warnings' format 'No matches for: %d'
```

## Fish

Fish shell provides the richest completion experience with dynamic source completion.

### Installation

```fish
# Standard installation
blz completions fish > ~/.config/fish/completions/blz.fish

# Reload shell or source config
exec fish
# or
source ~/.config/fish/config.fish
```

### Features

#### Dynamic Completions

Fish completions query your actual indexed sources:

```fish
# Complete with your actual sources
blz search -s <TAB>     # Shows: anthropic, nextjs, tanstack...
blz get <TAB>           # Shows available sources
blz refresh <TAB>       # Lists sources you can refresh (`blz update` alias still works)
blz remove <TAB>        # Shows removable sources
```

#### Rich Descriptions

```fish
blz <TAB>
  add         Add a new llms.txt source
  search      Search across cached docs
  get         Get exact lines from a source
  list        List all cached sources
  sync        Fetch latest documentation from sources
```

### Customization

#### Add Custom Completions

Edit `~/.config/fish/completions/blz.fish`:

```fish
# Add common sources as completions
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "react" -d "https://react.dev/llms-full.txt"
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "vue" -d "https://vuejs.org/llms-full.txt"
```

#### Abbreviations

Add to `~/.config/fish/config.fish`:

```fish
# Quick commands
abbr -a bs 'blz search'
abbr -a bg 'blz get'
abbr -a ba 'blz add'
abbr -a bl 'blz list'
abbr -a bu 'blz refresh --all'  # deprecated alias: blz update --all

# Common searches
abbr -a bsh 'blz search hooks'
abbr -a bsa 'blz search async'
```

### Helper Functions

Add to `~/.config/fish/functions/`:

```fish
# ~/.config/fish/functions/blz-quick.fish
function blz-quick -d "Quick search and get first result"
    set -l result (blz search $argv --limit 1 -f json | jq -r '.results[0] | "\(.alias) \(.lines)"')
    if test -n "$result"
        blz get $result
    else
        echo "No results for: $argv"
    end
end

# ~/.config/fish/functions/blz-fzf.fish
function blz-fzf -d "Search with fzf preview"
    blz search $argv -f json | \
    jq -r '.results[] | "\(.alias):\(.lines) \(.headingPath | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs blz get'
end
```

### Dynamic Alias & Anchor Completion

Enable live alias and anchor suggestions by sourcing the dynamic helper in your Fish config:

```fish
# e.g., in ~/.config/fish/config.fish
source /path/to/blz/scripts/blz-dynamic-completions.fish
```

Adds:

- `--source`/`-s` dynamic values for `blz query` and `blz get`
- Positional alias completion for `blz query`, `blz get`, `blz sync`, `blz rm`, `blz diff`, `blz map`
- `blz anchor list <alias>` alias completion
- `blz anchor get <alias> <anchor>` anchor completion (after alias is provided)

### Auto-update Completions

Add to `~/.config/fish/config.fish`:

```fish
# Update completions when blz binary changes
function __auto_update_blz --on-event fish_prompt
    set -l blz_bin (command -v blz)
    set -l comp_file ~/.config/fish/completions/blz.fish

    if test -n "$blz_bin" -a "$blz_bin" -nt "$comp_file"
        blz completions fish > $comp_file 2>/dev/null
    end
end
```

### Integration

#### With fzf

```fish
# Interactive search
function blzi
    set -l query (commandline -b)
    set -l result (blz search "$query" -f json | \
        jq -r '.results[] | "\(.alias) \(.lines) \(.headingPath[-1])"' | \
        fzf --preview 'echo {} | cut -d" " -f1-2 | xargs blz get' \
            --preview-window=right:60%)

    if test -n "$result"
        echo $result | cut -d" " -f1-2 | xargs blz get
    end
end

# Bind to Ctrl+B
bind \cb blzi
```

#### With VS Code

```fish
# Open result in VS Code
function blz-code
    set -l result (blz search $argv --limit 1 -f json)
    if test -n "$result"
        set -l alias (echo $result | jq -r '.results[0].alias')
        set -l lines (echo $result | jq -r '.results[0].lines')
        set -l start (echo $lines | cut -d'-' -f1)

        # Open file at line
        code ~/.local/share/blz/$alias/llms.txt:$start
    end
end
```

### Tips

1. **History**: Use ↑/↓ to navigate command history
2. **Wildcards**: `blz query "react*"` works
3. **Pipes**: `blz list | grep anthropic`
4. **JSON**: Parse with `jq` for scripting

## Bash

Standard completions for commands and options.

### Installation

#### Linux

```bash
# Most distros - standard location
blz completions bash | sudo tee /usr/share/bash-completion/completions/blz

# User-specific (no sudo required)
mkdir -p ~/.local/share/bash-completion/completions
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Reload
source ~/.bashrc
```

#### macOS

```bash
# Install bash-completion first
brew install bash-completion@2

# Add to ~/.bash_profile
echo '[[ -r "$(brew --prefix)/etc/profile.d/bash_completion.sh" ]] && . "$(brew --prefix)/etc/profile.d/bash_completion.sh"' >> ~/.bash_profile

# Install completions
blz completions bash > $(brew --prefix)/etc/bash_completion.d/blz

# Reload
source ~/.bash_profile
```

### Features

- Command completion
- Option/flag completion
- Filename completion for paths

### Usage

```bash
blz <TAB><TAB>           # List commands
blz query --<TAB><TAB>   # List options
blz add <TAB><TAB>       # Complete filenames
```

### Troubleshooting

#### Completions not working

```bash
# Check bash-completion is loaded
type _init_completion

# If not found, ensure bash-completion is installed
# Then source it manually
source /usr/share/bash-completion/bash_completion
```

#### Old Bash version

Bash 3.x (macOS default) has limited completion support. Consider:

```bash
# Install newer Bash
brew install bash

# Add to /etc/shells
echo $(brew --prefix)/bin/bash | sudo tee -a /etc/shells

# Change default shell
chsh -s $(brew --prefix)/bin/bash
```

## PowerShell

Setup guide for PowerShell completions on Windows, macOS, and Linux.

### Installation

#### Windows PowerShell / PowerShell Core

```powershell
# Generate completions for the current session
blz completions powershell | Out-String | Invoke-Expression

# To make permanent, save to a file and load from your profile
$profileDir = Split-Path -Parent $PROFILE
$completionFile = Join-Path $profileDir "blz-completions.ps1"
blz completions powershell > $completionFile
if (-not (Select-String -Path $PROFILE -Pattern $completionFile -Quiet)) {
    Add-Content $PROFILE "if (Test-Path `"$completionFile`") { . `"$completionFile`" }"
}
. $PROFILE
```

#### Check Profile Location

```powershell
# View profile path
$PROFILE

# Check if profile exists
Test-Path $PROFILE

# Create profile if needed
if (!(Test-Path $PROFILE)) {
    New-Item -Type File -Path $PROFILE -Force
}
```

### Features

- Command completion
- Parameter completion
- Dynamic alias/anchor completion (with helper script)

### Usage

```powershell
blz <Tab>              # Cycle through commands
blz query -<Tab>       # Cycle through parameters
blz add <Tab>          # Complete with files
```

### Aliases & Functions

Add to your PowerShell profile:

```powershell
# Aliases
Set-Alias bs blz search
Set-Alias bg blz get
Set-Alias ba blz add
Set-Alias bl blz list

# Search function
function Blz-Search {
    param([string]$Query)
    blz search $Query --limit 10
}

# Quick get function
function Blz-Quick {
    param([string]$Query)
    $result = blz search $Query --limit 1 --json | ConvertFrom-Json
    if ($result) {
        blz get "$($result.results[0].alias):$($result.results[0].lines)"
    } else {
        Write-Host "No results for: $Query"
    }
}

# Update all sources
function Blz-UpdateAll {
    blz refresh --all  # deprecated alias: blz update --all
}
```

### Integration with Windows Terminal

#### Custom Key Bindings

Add to Windows Terminal settings.json:

```json
{
    "command": {
        "action": "sendInput",
        "input": "blz search "
    },
    "keys": "ctrl+b"
}
```

### PowerShell Core (Cross-platform)

```powershell
# Install PowerShell Core
# Windows: winget install Microsoft.PowerShell
# macOS: brew install powershell
# Linux: See https://aka.ms/powershell

# Use same installation steps as above
pwsh
# Then run the same completion setup commands as above
```

### Dynamic Alias Completion

For live alias suggestions (canonical + metadata aliases) when typing `blz` commands, source the dynamic completer in your PowerShell profile:

```powershell
# Add to your PowerShell profile (e.g., $PROFILE)
. "$HOME/path/to/blz/scripts/blz-dynamic-completions.ps1"
```

This adds:

- `--source`/`-s` dynamic values for `blz query` and `blz get`
- Positional alias completion for `blz query`, `blz get`, `blz sync`, `blz rm`, `blz diff`, `blz map`, and `blz anchor list|get`
- Anchor value completion for `blz anchor get <alias> <anchor>`

It reads from `blz list --json` and merges canonical + metadata aliases.

### Troubleshooting

#### Execution Policy

If scripts are blocked:

```powershell
# Check policy
Get-ExecutionPolicy

# Allow local scripts
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

#### Profile Not Loading

```powershell
# Test profile loads
. $PROFILE

# Check for errors
$Error[0]
```

#### Completions Not Working

```powershell
# Re-import completions
blz completions powershell | Out-String | Invoke-Expression

# Check if Tab completion is enabled
Get-PSReadLineKeyHandler -Key Tab
```

### Advanced Features

#### JSON Processing

```powershell
# Parse JSON output
$resp = blz search "hooks" -f json | ConvertFrom-Json
$resp.results | ForEach-Object {
    Write-Host "$($_.alias): $($_.headingPath -join ' > ')"
}

# Filter high-score results
$highScore = blz search "async" -f json | ConvertFrom-Json |
    Select-Object -ExpandProperty results |
    Where-Object { $_.score -gt 50 }
```

#### Pipeline Integration

```powershell
# Search and select with Out-GridView
blz search "react" --json |
    ConvertFrom-Json |
    Select-Object -ExpandProperty results |
    Select-Object alias, lines, @{N='Path';E={$_.headingPath -join ' > '}} |
    Out-GridView -PassThru |
    ForEach-Object { blz get "$($_.alias):$($_.lines)" }
```

## Elvish

Setup guide for Elvish shell completions.

### Installation

```elvish
# Generate and install completions
blz completions elvish > ~/.elvish/lib/blz.elv

# Import in rc.elv
echo 'use blz' >> ~/.elvish/rc.elv

# Reload
exec elvish
```

### Features

- Command completion
- Option completion
- Elvish-style argument completion

### Usage

```elvish
blz <Tab>           # Complete commands
blz query --<Tab>   # Complete options
```

### Configuration

Add to `~/.elvish/rc.elv`:

```elvish
# Aliases using Elvish functions
fn bs [@args]{ blz search $@args }
fn bg [@args]{ blz get $@args }
fn ba [@args]{ blz add $@args }
fn bl { blz list }

# Quick search function
fn blz-quick [query]{
    var result = (blz search $query --limit 1 --json | from-json)
    if (not-eq $result []) {
        var hit = $result[results][0]
        blz get $hit[alias]":"$hit[lines]
    } else {
        echo "No results for: "$query
    }
}
```

### Key Bindings

```elvish
# Bind Ctrl-B for search
set edit:insert:binding[Ctrl-B] = {
    edit:replace-input "blz search "
    edit:move-dot-eol
}
```

### Integration with Elvish Modules

#### Create a BLZ module

Create `~/.elvish/lib/blz-utils.elv`:

```elvish
# Search with preview
fn search-preview [query]{
    blz search $query -f json |
        from-json |
        each [resp]{ each [hit]{ echo $hit[alias]":"$hit[lines]" "(str:join " > " $hit[headingPath]) } $resp[results] }
}

# List sources with details
fn list-detailed {
    blz list -f json | from-json | each [source]{
        echo $source[alias]" - Fetched at: "$source[fetchedAt]
    }
}

# Batch add sources
fn add-batch [sources]{
    for source $sources {
        var name url = (str:split "=" $source)
        blz add $name $url
        echo "Added: "$name
    }
}
```

Use in rc.elv:

```elvish
use blz-utils

# Usage
blz-utils:search-preview "hooks"
blz-utils:add-batch [react=https://react.dev/llms.txt vue=https://vuejs.org/llms.txt]
```

### Pipeline Integration

```elvish
# Filter and process results
blz search "async" -f json |
    from-json |
    each [resp]{ each [hit]{ if (> $hit[score] 50) { echo "High score: "$hit[alias]" "$hit[lines] } } $resp[results] }

# Count results by source
blz search "test" -f json |
    from-json |
    each [resp]{ each [hit]{ put $hit[alias] } $resp[results] } |
    sort | uniq -c
```

### Troubleshooting

#### Completions not loading

```elvish
# Check if file exists
ls ~/.elvish/lib/blz.elv

# Check if module loads
use blz

# Regenerate if needed
blz completions elvish > ~/.elvish/lib/blz.elv
```

#### Module errors

```elvish
# Debug module loading
-source ~/.elvish/lib/blz.elv

# Check for syntax errors
elvish -compileonly ~/.elvish/lib/blz.elv
```

### Tips

1. **History**: Use Ctrl-R for reverse search
2. **Wildcards**: Elvish supports advanced globbing
3. **Structured data**: Elvish handles JSON natively with `from-json`
4. **Parallel execution**: Use `peach` for parallel processing

### Advanced Example

```elvish
# Parallel search across multiple queries
fn multi-search [queries]{
    peach [q]{
        echo "=== Results for "$q" ==="
        blz search $q --limit 3
    } $queries
}

# Interactive source selector
fn select-source {
    var sources = [(blz list -f json | from-json | each [s]{ put $s[alias] })]
    var selected = (echo $@sources | tr ' ' '\n' | fzf)
    if (not-eq $selected "") {
        edit:replace-input "blz search -s "$selected" "
    }
}
```

## Integration Examples

### Fuzzy Search with fzf

```bash
# Fish/Bash/Zsh
function blz-fzf
    blz search "$1" --json | \
    jq -r '.results[] | "\(.alias):\(.lines) \(.headingPath | join(" > "))"' | \
    fzf --preview 'echo {} | cut -d: -f1,2 | xargs -I{} sh -c "blz get {}"'
end
```

### Alfred/Raycast Integration

Create a workflow script:

```bash
#!/bin/bash
# For Alfred/Raycast

query="$1"
results=$(blz search "$query" --json)

echo "$results" | jq -r '.results[] | {
    title: .headingPath | join(" > "),
    subtitle: "\(.alias) L\(.lines)",
    arg: "\(.alias):\(.lines)"
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
- Works on macOS and Linux (and PowerShell if `pwsh` is available)

### Manual Update

When you update the `blz` binary:

```bash
# Regenerate for your shell
blz completions fish > ~/.config/fish/completions/blz.fish
blz completions bash > ~/.local/share/bash-completion/completions/blz
blz completions zsh > ~/.zsh/completions/_blz
blz completions elvish > ~/.elvish/lib/blz.elv
```

PowerShell:

```powershell
$profileDir = Split-Path -Parent $PROFILE
$completionFile = Join-Path $profileDir "blz-completions.ps1"
blz completions powershell > $completionFile
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
blz list --json

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

Works the same as Linux within WSL. For native Windows, use PowerShell completions.

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
blz query <UP>  # Shows previous searches
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

## Contributing

To improve completions:

1. Edit `crates/blz-cli/src/cli.rs` (static completions + command descriptions)
2. Update helper scripts in `scripts/` for dynamic completions
3. Rebuild: `cargo build --release`
4. Test: `blz completions <shell>` and `source scripts/blz-dynamic-completions.<shell>`

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for more details.
