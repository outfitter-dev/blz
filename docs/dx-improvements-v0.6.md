# DX Improvements for v0.6

Planned developer experience enhancements focused on agent-friendliness and CLI usability.

## Priority: High

### 1. `--json` Convenience Flag

**Description**: Shortcut for `--format json` to reduce verbosity in agent workflows.

**Usage**:
```bash
blz search "react hooks" --json
blz list --json
blz get react:1-10 --json
blz history --json
```

**Implementation**:
- Add `#[arg(long, conflicts_with = "format")]` to relevant command structs
- Boolean flag that sets format to JSON when true
- Mutually exclusive with `--format`

**Affected Commands**: search, list, get, history, lookup, completions

**Complexity**: Low (1-2 hours)

---

### 2. `blz info <source>` Command

**Description**: Display detailed metadata about a cached source.

**Usage**:
```bash
blz info react
blz info react --json
```

**Output (text)**:
```
Source: react
URL: https://react.dev/llms-full.txt
Variant: llms-full
Aliases: @react/docs, react-docs
Lines: 15,234
Size: 1.2 MB
Last Updated: 2025-10-02 14:30:00 UTC
ETag: "abc123..."
Checksum: sha256:def456...
Cache Location: ~/.local/share/blz/sources/react/
```

**Output (JSON)**:
```json
{
  "alias": "react",
  "url": "https://react.dev/llms-full.txt",
  "variant": "llms-full",
  "aliases": ["@react/docs", "react-docs"],
  "lines": 15234,
  "size_bytes": 1258291,
  "last_updated": "2025-10-02T14:30:00Z",
  "etag": "abc123...",
  "checksum": "sha256:def456...",
  "cache_path": "/Users/user/.local/share/blz/sources/react"
}
```

**Implementation**:
- Add `Info { alias: String, #[command(flatten)] format: FormatArg }` to Commands enum
- Read source metadata from registry
- Read file stats from filesystem
- Format output based on `--format` flag

**Complexity**: Medium (3-4 hours)

---

### 3. Multiple Source Search

**Description**: Search across specific sources instead of all or one.

**Usage**:
```bash
blz search "hooks" --source react,vue,svelte
blz search "async" -s bun,node
blz "typescript generics" --source ts,react  # Default search mode
```

**Implementation**:
- Change `source: Option<String>` to `sources: Vec<String>` in Search command
- Keep `--source` singular for backward compat, add `--sources` alias
- Support comma-separated values with `value_delimiter = ','`
- Filter results to only matching sources
- Update search logic to handle multiple source filters

**Complexity**: Medium (2-3 hours)

---

### 4. `--raw` Output Mode

**Description**: Plain text output without JSON wrapper or formatting, pipe-friendly.

**Usage**:
```bash
blz get react:100-150 --raw
blz get react:100-150 --raw | pbcopy
blz search "hooks" --raw  # Just the matched content
```

**Output (get command)**:
```
Line content here
More line content
Another line
```

**Implementation**:
- Add `Raw` variant to `OutputFormat` enum
- Implement `format_raw()` for search and get commands
- For get: just print content without line numbers or metadata
- For search: print matched snippets only, no headers/scores

**Complexity**: Low-Medium (2-3 hours)

---

## Priority: Medium

### 5. Batch Add from File

**Description**: Add multiple sources from a file for easier onboarding.

**Usage**:
```bash
blz add --from-file sources.txt
blz add --from-file sources.txt --yes  # Skip confirmations
```

**File Format (sources.txt)**:
```
react,https://react.dev/llms-full.txt
vue,https://vuejs.org/llms.txt
svelte,https://svelte.dev/llms.txt
# Comments supported
typescript,https://typescriptlang.org/llms-full.txt
```

**Implementation**:
- Add `#[arg(long = "from-file", conflicts_with = "url")]` to Add command
- Parse file line-by-line (skip comments and empty lines)
- Validate format: `alias,url` per line
- Loop through and add each source
- Show progress bar for multiple sources
- Collect and report errors at end

**Complexity**: Medium (3-4 hours)

---

### 6. `blz stats` Command

**Description**: Global cache statistics and health overview.

**Usage**:
```bash
blz stats
blz stats --json
```

**Output (text)**:
```
BLZ Cache Statistics
====================
Total Sources: 12
Total Size: 45.3 MB
Total Lines: 234,561
Cache Location: ~/.local/share/blz/

Sources:
  react (1.2 MB, 15,234 lines, updated 2 hours ago)
  vue (856 KB, 10,123 lines, updated 1 day ago)
  svelte (645 KB, 8,456 lines, updated 3 days ago)
  ...

Oldest Source: nextjs (updated 14 days ago)
```

**Output (JSON)**:
```json
{
  "total_sources": 12,
  "total_size_bytes": 47533056,
  "total_lines": 234561,
  "cache_location": "/Users/user/.local/share/blz",
  "sources": [
    {
      "alias": "react",
      "size_bytes": 1258291,
      "lines": 15234,
      "last_updated": "2025-10-02T14:30:00Z",
      "age_hours": 2
    }
  ],
  "oldest_source": {
    "alias": "nextjs",
    "age_days": 14
  }
}
```

**Implementation**:
- Add `Stats { #[command(flatten)] format: FormatArg }` to Commands enum
- Iterate through all sources in registry
- Collect metadata (size, lines, timestamps)
- Calculate aggregates
- Format output

**Complexity**: Medium (3-4 hours)

---

### 7. Source Name Tab Completion

**Description**: Dynamic completion for source names in get/search commands.

**Usage**:
```bash
blz get re<TAB>  # Completes to: react
blz search "hooks" --source v<TAB>  # Completes to: vue
```

**Implementation**:
- Requires shell completion script enhancement
- Generate dynamic completions that read from registry
- Update `clap_complete` generation to include custom completions
- May need shell-specific implementations

**Note**: This is shell-dependent and requires user to regenerate completions after adding sources.

**Complexity**: High (6-8 hours, shell-specific work)

---

### 8. Edit Source Shortcut

**Description**: Quick access to view source URL or open in browser.

**Usage**:
```bash
blz edit react           # Opens URL in $BROWSER
blz edit react --url     # Just prints the URL
blz edit react --local   # Opens local cached file in $EDITOR
```

**Implementation**:
- Add `Edit { alias: String, #[arg(long)] url: bool, #[arg(long)] local: bool }` to Commands
- Read source URL from registry
- Use `open` (macOS), `xdg-open` (Linux), `start` (Windows) for browser
- Use `$EDITOR` env var for local file editing
- Fallback to printing URL if opener not available

**Complexity**: Medium (3-4 hours)

---

### 9. Copy to Clipboard

**Description**: Automatically copy output to system clipboard.

**Usage**:
```bash
blz get react:100-150 --copy
blz search "hooks" --limit 5 --copy
```

**Implementation**:
- Add `#[arg(long)]` copy flag to get and search commands
- Detect platform (macOS/Linux/Windows)
- Use platform-specific clipboard commands:
  - macOS: `pbcopy`
  - Linux: `xclip` or `xsel`
  - Windows: `clip.exe`
- Pipe output to clipboard command
- Print confirmation message
- Error gracefully if clipboard tool not available

**Complexity**: Medium (2-3 hours)

---

### 10. Search History Enhancements

**Description**: Better management of search history.

**Usage**:
```bash
blz history --clear              # Clear all history
blz history --clear-before "2025-09-01"  # Clear old entries
blz search "hooks" --no-history  # Don't save this search
```

**Implementation**:
- Add `#[arg(long)] clear: bool` and `#[arg(long)] clear_before: Option<String>` to History command
- Add `#[arg(long)] no_history: bool` to Search command
- Implement clear logic in history module
- Date parsing for `--clear-before`
- Update search command to skip history save when `--no-history` set

**Complexity**: Low-Medium (2-3 hours)

---

## Implementation Order (Recommended)

1. **#1 - `--json` flag** (Quick win, immediate value)
2. **#4 - `--raw` output** (Pipe-friendly, agent value)
3. **#3 - Multiple source search** (High demand feature)
4. **#2 - `blz info`** (Debugging/inspection value)
5. **#10 - History enhancements** (Low hanging fruit)
6. **#6 - `blz stats`** (Overview utility)
7. **#9 - Copy to clipboard** (Nice to have)
8. **#5 - Batch add** (Onboarding improvement)
9. **#8 - Edit shortcut** (Convenience feature)
10. **#7 - Tab completion** (Complex, platform-specific)

---

## Testing Requirements

Each feature should include:
- Unit tests for core logic
- Integration tests for CLI behavior
- JSON output validation tests
- Error handling tests
- Documentation updates (help text, README)

---

## Breaking Changes

None of these features introduce breaking changes. All are additive or opt-in.

---

## Documentation Updates Needed

- README examples section
- Command reference docs (via `blz docs`)
- Agent usage guide (.agents/instructions/use-blz.md)
- Man page updates

---

## Agent-Specific Considerations

**Most valuable for agents** (in order):
1. `--json` flag (ubiquitous need)
2. `--raw` output (piping to other tools)
3. Multiple source search (scoped searches)
4. `blz info` (metadata inspection)
5. Batch add (setup automation)

**Most valuable for humans**:
1. Copy to clipboard (workflow integration)
2. `blz stats` (overview)
3. Edit shortcut (quick access)
4. Tab completion (discoverability)
5. History management (cleanup)
