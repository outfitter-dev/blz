# Troubleshooting Guide

Solutions to common BLZ issues and error messages.

## Quick Diagnosis

### Check Your Setup

```bash
# Verify BLZ is installed
blz --version

# List your sources
blz list

# Check if sources are populated
blz list --json | jq 'length'

# View recent search history
blz history -n5
```

---

## Installation Issues

### "command not found: blz"

**Cause**: BLZ not in PATH

**Solution**:

```bash
# Check if binary exists
ls ~/.local/bin/blz

# If exists, add to PATH in ~/.bashrc or ~/.zshrc:
export PATH="$HOME/.local/bin:$PATH"

# Reload shell config
source ~/.bashrc  # or source ~/.zshrc
```

### Install script fails

**Error**: `curl: (6) Could not resolve host`

**Solution**: Check internet connection

**Error**: `Permission denied`

**Solution**: Install to user directory:

```bash
curl -fsSL https://blz.run/install.sh | sh -s -- --dir ~/.local/bin
```

### Cargo install fails

**Error**: `error: failed to compile blz-cli`

**Solution**: Update Rust:

```bash
rustup update
cargo install --git https://github.com/outfitter-dev/blz blz-cli --force
```

---

## Source Management Issues

### "Source not found" when adding

**Error**: `Error: Failed to fetch https://example.com/llms.txt`

**Solutions**:

1. **Verify URL is accessible**:

```bash
curl -I https://example.com/llms.txt
# Should return HTTP 200
```

2. **Check URL format**: Must end in `/llms.txt` or `/llms-full.txt`

3. **Try with explicit URL**:

```bash
blz add example https://example.com/llms.txt
```

### "Source already exists"

**Error**: `Error: Source 'bun' already exists`

**Solutions**:

1. **Refresh existing source**:

```bash
blz refresh bun  # deprecated alias: blz update bun
```

2. **Remove and re-add**:

```bash
blz remove bun
blz add bun https://bun.sh/llms.txt
```

3. **Use different alias**:

```bash
blz add bun-docs https://bun.sh/llms.txt
```

### "Invalid llms.txt format"

**Error**: `Error: Failed to parse llms.txt`

**Cause**: Source file is not valid Markdown or llms.txt format

**Solution**: Verify file format manually:

```bash
curl https://example.com/llms.txt | head -50
```

Look for:

- Markdown headings (`#`, `##`, etc.)
- Structured content (not HTML/JSON/XML)

### Check source details

**View source information**:

```bash
# List with details
blz list --details

# JSON output for inspection
blz list --json | jq '.[] | select(.alias == "bun")'
```

---

## Search Issues

### Empty search results

**Problem**: `No results found for 'query'`

**Solutions**:

1. **Check sources exist**:

```bash
blz list
# Should show at least one source
```

2. **Verify source filter**:

```bash
# Wrong:
blz "test" -s bn  # Typo

# Correct:
blz "test" -s bun
```

3. **Try broader terms**:

```bash
# Too specific:
blz "bun.serve with websockets and https"

# Better:
blz "bun serve"
# or
blz "websockets"
```

4. **Remove source filter to search all**:

```bash
blz "query"  # Searches all sources
```

### Search is slow (>100ms)

**Expected**: First search after boot ~50-100ms (index loads from disk)
**Expected**: Subsequent searches <10ms (cached in memory)

**If all searches are slow**:

1. **Check source count and size**:

```bash
blz list --json | jq '[.[] | {alias, lines, headings}]'
```

2. **Rebuild index** (remove and re-add):

```bash
blz remove <alias>
blz add <alias> <url>
```

3. **Check system resources**:
   - Disk I/O (SSD vs HDD)
   - Available memory
   - CPU load

### "Query too short" error

**Error**: `Error: Query must be at least 2 characters`

**Solution**: Use longer search terms:

```bash
# Too short:
blz "a"

# Minimum:
blz "ab"

# Better:
blz "api"
```

---

## Get Command Issues

### "Invalid line range"

**Error**: `Error: Invalid line range: 'abc-def'`

**Solution**: Use numeric ranges:

```bash
# Wrong:
blz get bun:abc-def

# Correct:
blz get bun:100-150
```

### "Lines out of range"

**Error**: `Error: Lines 10000-20000 exceed source length (5000 lines)`

**Solution**: Check source length first:

```bash
# View source details
blz list --json | jq '.[] | select(.alias == "bun") | {alias, lines}'

# Use valid range:
blz get bun:100-200
```

### "Source not found" when using get

**Error**: `Error: Source 'bn' not found`

**Solutions**:

1. **Check spelling**:

```bash
blz list  # Shows available aliases
```

2. **Use exact alias**:

```bash
blz get bun:100-150  # Not 'bn' or 'bunjs'
```

---

## Cache & Storage Issues

### "Permission denied" accessing cache

**Error**: `Error: Permission denied: /path/to/cache`

**Solutions**:

1. **Check ownership** (macOS):

```bash
ls -la ~/Library/Application\ Support/dev.outfitter.blz/
# Should be owned by your user
```

2. **Check ownership** (Linux):

```bash
ls -la ~/.local/share/blz/
# Should be owned by your user
```

3. **Fix permissions**:

```bash
# macOS
chmod -R u+rw ~/Library/Application\ Support/dev.outfitter.blz/

# Linux
chmod -R u+rw ~/.local/share/blz/
```

4. **Use custom directory**:

```bash
export BLZ_DATA_DIR=~/my-blz-cache
blz list  # Uses new location
```

### Cache corruption

**Symptoms**:

- Random crashes
- Empty search results for known sources
- "Invalid index" errors

**Solution**: Clear and rebuild:

```bash
# Remove corrupted source
blz remove <alias>

# Re-add source (rebuilds index)
blz add <alias> <url>
```

### "Disk full" errors

**Error**: `Error: No space left on device`

**Solutions**:

1. **Check cache size**:

```bash
# macOS
du -sh ~/Library/Application\ Support/dev.outfitter.blz/*

# Linux
du -sh ~/.local/share/blz/*
```

2. **Remove unused sources**:

```bash
blz list
blz remove <unused-alias>
```

3. **Clear all sources** (nuclear option):

```bash
# macOS
rm -rf ~/Library/Application\ Support/dev.outfitter.blz/*

# Linux
rm -rf ~/.local/share/blz/*

# Then re-add sources
blz add bun https://bun.sh/llms.txt
```

---

## Refresh Issues

### Refresh says "already up to date" but content is stale

**Problem**: ETag caching prevents fetch when content changed without ETag update

**Solution**: Force re-fetch by removing and re-adding:

```bash
blz remove bun
blz add bun https://bun.sh/llms.txt
```

### Refresh fails with network error

**Error**: `Error: Network request failed`

**Solutions**:

1. **Check internet connection**
2. **Retry refresh**:

```bash
blz refresh bun  # deprecated alias: blz update bun
```

3. **Check URL still valid**:

```bash
curl -I https://bun.sh/llms.txt
```

---

## Output & Format Issues

### JSON output is malformed

**Problem**: Can't parse with `jq`

**Solution**: Ensure clean JSON output:

```bash
# Use --json for clean output
blz "query" --json | jq '.results[0]'

# Set as default to avoid warnings
export BLZ_OUTPUT_FORMAT=json
```

### Colors showing up in piped output

**Problem**: ANSI color codes in output

**Solution**: Disable colors:

```bash
blz "query" --color never
# or
export NO_COLOR=1
blz "query"
```

### Unexpected output format

**Problem**: Getting text when expecting JSON

**Solution**: Set output format explicitly:

```bash
# Use --json flag
blz "query" --json

# Or set environment variable
export BLZ_OUTPUT_FORMAT=json
blz "query"
```

---

## Performance Issues

### Indexing takes longer than expected

**Expected**: ~100-200ms per MB of markdown

**If slower**:

1. **Check disk speed** (HDD vs SSD)
2. **Close resource-intensive apps**
3. **Monitor during index**:

```bash
blz add source url --debug
```

### High memory usage

**Expected**: <100MB for typical use

**If higher**:

1. **Check number of sources**:

```bash
blz list --json | jq 'length'
```

2. **Check total lines indexed**:

```bash
blz list --json | jq '[.[] | .lines] | add'
```

3. **Limit concurrent operations** (update one at a time):

```bash
# Instead of:
blz refresh --all  # deprecated alias: blz update --all

# Do one at a time:
for src in $(blz list --json | jq -r '.[].alias'); do
  blz refresh "$src"  # deprecated alias: blz update "$src"
done
```

---

## Configuration Issues

### Config file not loading

**Problem**: Changes to `config.toml` not taking effect

**Solutions**:

1. **Check config file location**:

```bash
# Linux/macOS (XDG)
cat ~/.config/blz/config.toml

# macOS (Application Support)
cat ~/Library/Application\ Support/dev.outfitter.blz/config.toml
```

2. **Verify TOML syntax**:

```bash
# Invalid TOML will be silently ignored
# Use a TOML validator or check for syntax errors
```

3. **Use explicit config path**:

```bash
export BLZ_CONFIG=/path/to/config.toml
blz "query"
```

### Environment variables not working

**Problem**: `BLZ_*` variables ignored

**Solutions**:

1. **Verify variable is set**:

```bash
echo $BLZ_OUTPUT_FORMAT
# Should print: json
```

2. **Export variable** (not just set):

```bash
# Wrong:
BLZ_OUTPUT_FORMAT=json

# Correct:
export BLZ_OUTPUT_FORMAT=json
```

3. **Check variable naming**:

```bash
# Wrong:
export BLZ_FORMAT=json

# Correct:
export BLZ_OUTPUT_FORMAT=json
```

---

## Shell Integration Issues

### Tab completion not working

**Problem**: Pressing TAB doesn't complete commands

**Solutions**:

1. **Generate completions**:

```bash
# Fish
blz completions fish > ~/.config/fish/completions/blz.fish

# Bash
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Zsh
blz completions zsh > ~/.zsh/completions/_blz
```

2. **Restart shell** or reload config:

```bash
# Fish
source ~/.config/fish/config.fish

# Bash
source ~/.bashrc

# Zsh
source ~/.zshrc
```

### Completions outdated after upgrade

**Problem**: Completions show old commands/flags

**Solution**: Regenerate completions:

```bash
# Re-run completions command for your shell
blz completions fish > ~/.config/fish/completions/blz.fish
# Then restart shell
```

---

## Getting More Help

### Enable Debug Output

```bash
blz "query" --debug
```

Shows:

- Query parsing
- Index loading time
- Search execution time
- Result scoring

### Enable Profiling

```bash
blz "query" --profile
```

Shows:

- Memory usage
- CPU time
- Disk I/O

### Enable Verbose Mode

```bash
blz "query" --verbose
```

Shows:

- Detailed operation logs
- Configuration loading
- Cache access patterns

### View Command Documentation

```bash
# Get help for specific command
blz search --help
blz get --help

# Generate full CLI docs
blz docs

# Generate JSON docs for parsing
blz docs --json
```

### Report a Bug

Include in bug report:

1. **BLZ version**: `blz --version`
2. **OS**: `uname -a` (Linux/macOS) or `systeminfo` (Windows)
3. **Command**: Full command that fails
4. **Error output**: Complete error message
5. **Debug output**: `blz <command> --debug`
6. **Source info**: `blz list --json` (if relevant)

File at: <https://github.com/outfitter-dev/blz/issues>

---

## Common Error Messages

### "Failed to build index"

**Likely cause**: Corrupted source file or parse error

**Solution**:

```bash
# Remove and re-add source
blz remove <alias>
blz add <alias> <url>
```

### "Invalid UTF-8"

**Likely cause**: Source file contains non-UTF-8 characters

**Solution**: Report to source maintainer or try alternative URL

### "Connection timeout"

**Likely cause**: Network issue or slow server

**Solutions**:

```bash
# Retry the operation
blz add <alias> <url>

# Check URL is accessible
curl -I <url>
```

### "ETag mismatch"

**Likely cause**: Source changed during fetch

**Solution**: Retry the operation:

```bash
blz refresh <alias>  # deprecated alias: blz update <alias>
```

---

## See Also

- [Commands](commands.md) - Complete command reference
- [Configuration](configuration.md) - Configuration options
- [Search Guide](search.md) - Search syntax and patterns
- [Sources](sources.md) - Managing documentation sources
