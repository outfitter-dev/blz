# Fish Completions

Fish shell provides the richest completion experience with dynamic source completion.

## Installation

```fish
# Standard installation
blz completions fish > ~/.config/fish/completions/blz.fish

# Reload shell or source config
exec fish
# or
source ~/.config/fish/config.fish
```

## Features

### Dynamic Completions

Fish completions query your actual indexed sources:

```fish
# Complete with your actual sources
blz search -s <TAB>     # Shows: anthropic, nextjs, tanstack...
blz get <TAB>           # Shows available sources
blz update <TAB>        # Lists sources you can update
blz remove <TAB>        # Shows removable sources
```

### Rich Descriptions

```fish
blz <TAB>
  add         Add a new llms.txt source
  search      Search across cached docs
  get         Get exact lines from a source
  list        List all cached sources
  update      Update sources
```

## Customization

### Add Custom Completions

Edit `~/.config/fish/completions/blz.fish`:

```fish
# Add common sources as completions
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "react" -d "https://react.dev/llms-full.txt"
complete -c blz -n "__fish_seen_subcommand_from add" \
    -a "vue" -d "https://vuejs.org/llms-full.txt"
```

### Abbreviations

Add to `~/.config/fish/config.fish`:

```fish
# Quick commands
abbr -a bs 'blz search'
abbr -a bg 'blz get'
abbr -a ba 'blz add'
abbr -a bl 'blz list'
abbr -a bu 'blz update --all'

# Common searches
abbr -a bsh 'blz search hooks'
abbr -a bsa 'blz search async'
```

## Helper Functions

Add to `~/.config/fish/functions/`:

```fish
# ~/.config/fish/functions/blz-quick.fish
function blz-quick -d "Quick search and get first result"
    set -l result (blz search $argv --limit 1 -f json | jq -r '.[] | "\(.alias) \(.lines)"')
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

## Dynamic Alias & Anchor Completion

Enable live alias and anchor suggestions by sourcing the dynamic helper in your Fish config:

```fish
# e.g., in ~/.config/fish/config.fish
source /path/to/blz/scripts/blz-dynamic-completions.fish
```

Adds:
- `--alias`/`-s` dynamic values for `blz search`
- Positional alias completion for `blz get`, `blz update`, `blz remove`, `blz diff`, `blz anchors`
- `blz anchor list <alias>` alias completion
- `blz anchor get <alias> <anchor>` anchor completion (after alias is provided)
```

## Auto-update Completions

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

## Integration

### With fzf

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

### With VS Code

```fish
# Open result in VS Code
function blz-code
    set -l result (blz search $argv --limit 1 -f json)
    if test -n "$result"
        set -l alias (echo $result | jq -r '.[0].alias')
        set -l lines (echo $result | jq -r '.[0].lines')
        set -l start (echo $lines | cut -d'-' -f1)

        # Open file at line
        code ~/.local/share/dev.outfitter.blz/$alias/llms.txt:$start
    end
end
```

## Tips

1. **History**: Use ↑/↓ to navigate command history
2. **Wildcards**: `blz search react*` works
3. **Pipes**: `blz list | grep anthropic`
4. **JSON**: Parse with `jq` for scripting
