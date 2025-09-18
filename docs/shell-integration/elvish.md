# Elvish Completions

Setup guide for Elvish shell completions.

## Installation

```elvish
# Generate and install completions
blz completions elvish > ~/.elvish/lib/blz.elv

# Import in rc.elv
echo 'use blz' >> ~/.elvish/rc.elv

# Reload
exec elvish
```

## Features

- Command completion
- Option completion
- Elvish-style argument completion

## Usage

```elvish
blz <Tab>           # Complete commands
blz search --<Tab>  # Complete options
```

## Configuration

Add to `~/.elvish/rc.elv`:

```elvish
# Aliases using Elvish functions
fn bs [@args]{ blz search $@args }
fn bg [@args]{ blz get $@args }
fn ba [@args]{ blz add $@args }
fn bl { blz list }

# Quick search function
fn blz-quick [query]{
    var result = (blz search $query --limit 1 -f json | from-json)
    if (not-eq $result []) {
        var hit = $result[0]
        blz get $hit[alias] --lines $hit[lines]
    } else {
        echo "No results for: "$query
    }
}
```

## Key Bindings

```elvish
# Bind Ctrl-B for search
set edit:insert:binding[Ctrl-B] = {
    edit:replace-input "blz search "
    edit:move-dot-eol
}
```

## Integration with Elvish Modules

### Create a blz module

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

## Pipeline Integration

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

## Troubleshooting

### Completions not loading

```elvish
# Check if file exists
ls ~/.elvish/lib/blz.elv

# Check if module loads
use blz

# Regenerate if needed
blz completions elvish > ~/.elvish/lib/blz.elv
```

### Module errors

```elvish
# Debug module loading
-source ~/.elvish/lib/blz.elv

# Check for syntax errors
elvish -compileonly ~/.elvish/lib/blz.elv
```

## Tips

1. **History**: Use Ctrl-R for reverse search
2. **Wildcards**: Elvish supports advanced globbing
3. **Structured data**: Elvish handles JSON natively with `from-json`
4. **Parallel execution**: Use `peach` for parallel processing

## Advanced Example

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
