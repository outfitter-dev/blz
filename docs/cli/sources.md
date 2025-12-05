# Managing Sources

This guide covers everything about managing documentation sources in BLZ.

## Understanding Sources

A **source** is an indexed copy of documentation from a URL, typically in `llms.txt` format. Each source has:

- An **alias** - Short name for referencing (e.g., `bun`, `node`)
- A **URL** - Where the documentation comes from
- **Metadata** - Fetch time, ETag, size, checksum

## Adding Sources

### Basic Usage

```bash
blz add <alias> <url>
```

Example:

```bash
blz add bun https://bun.sh/llms.txt
```

### What Happens When You Add

1. **Fetch** - Downloads the content from the URL
2. **Parse** - Extracts structure using tree-sitter-markdown
3. **Index** - Builds search index with Tantivy
4. **Store** - Saves to local filesystem

### Naming Conventions

Choose short, memorable aliases:

- ✅ Good: `bun`, `node`, `react`, `vue`
- ❌ Avoid: `my-bun-docs`, `bunjs-latest-documentation`

### Adding Multiple Sources

```bash
# Add several documentation sources
blz add bun https://bun.sh/llms.txt
blz add node https://nodejs.org/llms.txt
blz add deno https://deno.land/llms.txt
blz add ts https://typescriptlang.org/llms.txt
```

## Listing Sources

### View All Sources

```bash
blz list
```

Output:

```
Cached sources:

  bun https://bun.sh/llms.txt
    Fetched: 2025-08-23 00:55:33
    Lines: 364

  node https://nodejs.org/api/all.json
    Fetched: 2025-08-23 00:58:17
    Lines: 108600
```

### JSON Format

For scripting and automation:

```bash
blz list --json
```

Output:

```json
[
  {
    "alias": "bun",
    "url": "https://bun.sh/llms.txt",
    "fetched_at": "2025-08-23T00:55:33Z",
    "lines": 364
  }
]
```

## Updating Sources (Coming Soon)

> Note: Update functionality is not yet implemented in the MVP

### Future: Refresh Single Source

```bash
blz refresh bun  # deprecated alias: blz update bun
```

This will:

- Check ETag/Last-Modified headers
- Skip if unchanged (304 Not Modified)
- Re-index only if content changed

### Future: Refresh All Sources

```bash
blz refresh --all  # deprecated alias: blz update --all
```

Updates all sources efficiently using conditional requests.

## Storage Location

Sources are stored in platform-specific directories:

### macOS

```
~/Library/Application Support/dev.outfitter.blz/
  bun/
    llms.txt        # Latest upstream text
    llms.json       # Parsed TOC + line map
    .index/         # Search index (Tantivy)
    .archive/       # Historical snapshots
    settings.toml   # Per-source configuration
```

### Linux

```
~/.local/share/blz/
  bun/
    llms.txt
    llms.json
    .index/
    .archive/
    settings.toml
```

### Windows

```
%APPDATA%\outfitter\blz\
  bun\
    llms.txt
    llms.json
    .index\
    .archive\
    settings.toml
```

## Source Metadata

Each source stores metadata in `llms.json`:

```json
{
  "alias": "bun",
  "source": {
    "url": "https://bun.sh/llms.txt",
    "etag": "\"abc123\"",
    "last_modified": "Wed, 21 Oct 2025 07:28:00 GMT",
    "fetched_at": "2025-08-23T00:55:33.378Z",
    "sha256": "base64hash..."
  },
  "toc": [...],
  "line_index": {
    "total_lines": 364,
    "byte_offsets": false
  },
  "diagnostics": []
}
```

## Managing Disk Space

### Check Storage Usage

```bash
# macOS/Linux
du -sh ~/Library/Application\ Support/dev.outfitter.blz/*

# Example output:
# 128K    bun
# 4.8M    node
# 64K     deno
```

### Remove a Source

Currently manual - delete the directory:

```bash
# macOS
rm -rf ~/Library/Application\ Support/dev.outfitter.blz/bun

# Linux
rm -rf ~/.local/share/blz/bun
```

## Source Types

### Standard llms.txt

Most sources follow the llms.txt format:

```markdown
# Project Documentation

## Section 1
Content...

## Section 2
Content...
```

### JSON Documents

BLZ can handle JSON documents (like Node.js API):

```bash
blz add node https://nodejs.org/api/all.json
```

### Markdown Files

Any valid Markdown works:

```bash
blz add readme https://raw.githubusercontent.com/user/repo/main/README.md
```

## Best Practices

### 1. Use Descriptive Aliases

- `bun` not `b`
- `react` not `r`
- `typescript` not `ts-docs-latest`

### 2. Group Related Sources

```bash
# Frontend frameworks
blz add react https://react.dev/llms.txt
blz add vue https://vuejs.org/llms.txt
blz add svelte https://svelte.dev/llms.txt

# Build tools
blz add vite https://vitejs.dev/llms.txt
blz add webpack https://webpack.js.org/llms.txt
```

### 3. Regular Updates

Once implemented, update regularly:

```bash
# Future: Daily refresh
blz refresh --all  # deprecated alias: blz update --all
```

### 4. Monitor Storage

BLZ is efficient, but check occasionally:

```bash
blz list  # Shows line counts
```

## Troubleshooting

### 404 Not Found

The URL might be incorrect:

```bash
# Wrong
blz add bun https://bun.sh/docs/llms.txt

# Correct
blz add bun https://bun.sh/llms.txt
```

### Network Errors

Check your internet connection and try again.

### Parse Errors

BLZ handles malformed documents gracefully, but check diagnostics in the JSON:

```bash
cat ~/Library/Application\ Support/dev.outfitter.blz/bun/llms.json | jq .diagnostics
```

## Advanced Usage

### Custom Sources

Create your own llms.txt:

```bash
# Create a custom documentation file
cat > my-docs.txt << EOF
# My Project

## Installation
npm install my-project

## Usage
import { thing } from 'my-project'
EOF

# Serve it locally
python3 -m http.server 8000

# Add to cache
blz add myproject http://localhost:8000/my-docs.txt
```

### Scripting Source Management

```bash
#!/bin/bash
# Batch add sources from a list

sources=(
  "bun,https://bun.sh/llms.txt"
  "node,https://nodejs.org/llms.txt"
  "deno,https://deno.land/llms.txt"
)

for source in "${sources[@]}"; do
  IFS=',' read -r alias url <<< "$source"
  echo "Adding $alias from $url"
  blz add "$alias" "$url"
done
```

## Next Steps

- Learn about [searching](search.md) your indexed sources
- Understand the storage layout details in [AGENTS.md](../../AGENTS.md#storage-layout)
- Read about [architecture](../architecture/README.md) for technical details
