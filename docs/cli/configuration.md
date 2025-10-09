# Configuration

Complete guide to configuring BLZ through global config files, per-source settings, and environment variables.

## Table of Contents

- [Overview](#overview)
- [Configuration Precedence](#configuration-precedence)
- [Configuration Locations](#configuration-locations)
- [Global Config](#global-config)
- [Per-Source Settings](#per-source-settings)
- [Environment Variables](#environment-variables)
- [Defaults](#defaults)
- [CLI State Files](#cli-state-files)

## Overview

BLZ provides multiple configuration mechanisms:

- **Global config** - `config.toml` defines defaults for all sources
- **Per-source settings** - `settings.toml` in each source's cache directory
- **Environment variables** - Override configuration at runtime
- **CLI flags** - Override everything on a per-command basis
- **CLI state files** - Persistent preferences and history

This layered approach allows you to set global defaults while customizing behavior per source or per invocation.

## Configuration Precedence

Configuration is merged from lowest to highest priority:

1. **Built-in defaults** - Hardcoded sensible defaults
2. **Global `config.toml`** - Your global configuration file
3. **`config.local.toml`** - Optional local overrides (same directory as `config.toml`)
4. **Per-source `settings.toml`** - Source-specific overrides
5. **Environment variables** - `BLZ_*` variables override files
6. **CLI flags** - Command-line arguments override everything

### Example

```bash
# Built-in default: refresh_hours = 24

# config.toml sets: refresh_hours = 48

# source/react/settings.toml sets: refresh_hours = 12

# Environment variable: BLZ_REFRESH_HOURS=6

# CLI flag: blz update react --refresh-hours 1

# Actual value used: 1 (CLI flag wins)
```text

## Configuration Locations

### Global Config Directory

Where `config.toml` lives:

**Linux:**

```text
~/.config/blz/config.toml
```text

**macOS:**

```text
~/Library/Application Support/dev.outfitter.blz/config.toml
```text

**Windows:**

```text
%APPDATA%\dev.outfitter.blz\config.toml
```text

**Override:**

```bash
# Point to specific file
export BLZ_CONFIG=/path/to/config.toml

# Or specify directory containing config.toml
export BLZ_CONFIG_DIR=/path/to/dir
export BLZ_GLOBAL_CONFIG_DIR=/path/to/global/config
```text

### Cache Root Directory

Where indexed sources are stored:

**Linux:**

```text
~/.local/share/dev.outfitter.blz/
```text

**macOS:**

```text
~/Library/Application Support/dev.outfitter.blz/
```text

**Windows:**

```text
%APPDATA%\dev.outfitter.blz\
```text

**Override:**

```bash
export BLZ_ROOT=/path/to/cache
```text

### Per-Source Directory Structure

```text
<cache_root>/
├── config.toml              # Global config
├── config.local.toml        # Optional local overrides
├── blz.json                 # CLI state (preferences, history)
└── <alias>/
    ├── llms.txt             # Cached documentation
    ├── llms.json            # Parsed structure
    ├── settings.toml        # Per-source overrides
    └── .index/              # Tantivy search index
```text

## Global Config

The global config file (`config.toml`) defines defaults for all sources.

### Full Example

```toml
[defaults]
# How often to check for updates (hours)
refresh_hours = 24

# Maximum archived versions to keep
max_archives = 10

# Enable/disable network fetches
fetch_enabled = true

# Link following policy: "none" | "first_party" | "allowlist"
follow_links = "first_party"

# Domains to follow when follow_links = "allowlist"
allowlist = ["developer.mozilla.org", "docs.rs"]

[paths]
# Override cache root (optional)
# root = "/absolute/path/to/cache"
```text

### Configuration Keys

#### `[defaults]`

**`refresh_hours`** (integer)

- Hours between automatic refresh checks
- Default: `24`
- Example: `refresh_hours = 48` (check every 2 days)

**`max_archives`** (integer)

- Number of archived versions to keep per source
- Default: `10`
- Example: `max_archives = 5`

**`fetch_enabled`** (boolean)

- Enable/disable network fetches
- Default: `true`
- Example: `fetch_enabled = false` (offline mode)

**`follow_links`** (string)

- Link following policy
- Options: `"none"`, `"first_party"`, `"allowlist"`
- Default: `"first_party"`
- Examples:
  - `"none"` - Don't follow any links
  - `"first_party"` - Only follow links on same domain
  - `"allowlist"` - Only follow links to domains in allowlist

**`allowlist`** (array of strings)

- Domains to follow when `follow_links = "allowlist"`
- Default: `[]`
- Example: `allowlist = ["react.dev", "github.com"]`

#### `[paths]`

**`root`** (string)

- Absolute path to cache root directory
- Optional - overrides platform default
- Example: `root = "/custom/path/to/cache"`

### Local Overrides

Create `config.local.toml` in the same directory as `config.toml` for machine-specific overrides:

```toml
# config.local.toml
[defaults]
# Override just what you need
refresh_hours = 12
```text

This file is merged on top of `config.toml` and can be git-ignored for local preferences.

## Per-Source Settings

Each source can have its own `settings.toml` file that overrides global defaults.

### Location

```text
<cache_root>/<alias>/settings.toml
```text

For example:

```text
~/.local/share/dev.outfitter.blz/react/settings.toml
```text

### Example

```toml
[meta]
name = "react"
display_name = "React Documentation"
homepage = "https://react.dev"
repo = "https://github.com/facebook/react"

[fetch]
# Check React docs more frequently
refresh_hours = 12

# Follow React-specific links
follow_links = "first_party"
allowlist = ["react.dev", "github.com"]

[index]
# Allow larger heading blocks for React docs
max_heading_block_lines = 500
```text

### Configuration Keys

#### `[meta]`

Optional metadata for display purposes:

- **`name`** - Canonical source name
- **`display_name`** - Human-friendly name
- **`homepage`** - Project homepage URL
- **`repo`** - Repository URL

#### `[fetch]`

Override fetch behavior for this source:

- **`refresh_hours`** - Source-specific refresh interval
- **`follow_links`** - Link policy for this source
- **`allowlist`** - Domain allowlist for this source

#### `[index]`

Source-specific indexing options:

- **`max_heading_block_lines`** - Maximum lines in a heading block

### Notes

- Only keys present in `settings.toml` override global config
- Missing keys inherit from global `config.toml`
- Per-source settings take precedence over global config

## Environment Variables

Environment variables provide runtime configuration overrides.

### Configuration Variables

**`BLZ_CONFIG`**

- Absolute path to `config.toml` file
- Example: `export BLZ_CONFIG=/path/to/config.toml`

**`BLZ_CONFIG_DIR`**

- Directory containing `config.toml`
- Example: `export BLZ_CONFIG_DIR=/path/to/dir`

**`BLZ_GLOBAL_CONFIG_DIR`**

- Override global configuration directory
- Example: `export BLZ_GLOBAL_CONFIG_DIR=~/.config/blz`

**`BLZ_ROOT`**

- Override cache root directory
- Example: `export BLZ_ROOT=/custom/cache`

### Behavior Variables

**`BLZ_REFRESH_HOURS`**

- Integer hours between refresh checks
- Example: `export BLZ_REFRESH_HOURS=48`

**`BLZ_MAX_ARCHIVES`**

- Integer count of archived versions
- Example: `export BLZ_MAX_ARCHIVES=5`

**`BLZ_FETCH_ENABLED`**

- Enable/disable network fetches
- Values: `1`, `true`, `yes`, `on` (case-insensitive)
- Example: `export BLZ_FETCH_ENABLED=false`

**`BLZ_FOLLOW_LINKS`**

- Link following policy
- Values: `none`, `first_party`, `allowlist`
- Example: `export BLZ_FOLLOW_LINKS=allowlist`

**`BLZ_ALLOWLIST`**

- Comma-separated list of domains
- Example: `export BLZ_ALLOWLIST=react.dev,github.com`

### CLI Behavior Variables

**`BLZ_OUTPUT_FORMAT`**

- Default CLI output format
- Values: `json`, `text`, `jsonl`
- Example: `export BLZ_OUTPUT_FORMAT=json`

**`BLZ_SUPPRESS_DEPRECATIONS`**

- Suppress deprecation warnings
- Values: `1`, `true`, `yes`, `on`
- Example: `export BLZ_SUPPRESS_DEPRECATIONS=true`

**`BLZ_FORCE_NON_INTERACTIVE`**

- Skip confirmation prompts
- Values: `1`, `true`, `yes`, `on`
- Example: `export BLZ_FORCE_NON_INTERACTIVE=true`

**`NO_COLOR`**

- Disable ANSI colors in output
- Values: Any value disables colors
- Example: `export NO_COLOR=1`

### Process Management Variables

**`BLZ_DISABLE_GUARD`**

- Disable parent process watchdog
- Values: `1`, `true`, `yes`, `on`
- Example: `export BLZ_DISABLE_GUARD=true`

**`BLZ_PARENT_GUARD_INTERVAL_MS`**

- Watchdog poll interval (100-10000ms)
- Default: `500`
- Example: `export BLZ_PARENT_GUARD_INTERVAL_MS=1000`

**`BLZ_PARENT_GUARD_TIMEOUT_MS`**

- Watchdog timeout before force exit (milliseconds)
- Example: `export BLZ_PARENT_GUARD_TIMEOUT_MS=5000`

**`BLZ_PARENT_GUARD_TIMEOUT_SECS`**

- Watchdog timeout (seconds, alternative to `_MS`)
- Example: `export BLZ_PARENT_GUARD_TIMEOUT_SECS=5`

### Deprecated Variables

**`BLZ_PREFER_LLMS_FULL`**

- No longer used
- BLZ automatically prefers `llms-full.txt` when available

## Defaults

Built-in defaults are applied when no configuration is provided.

### Default Values

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party"
allowlist = []

[cli]
# CLI presentation defaults (stored in blz.json)
show = []
snippet_lines = 3
score_precision = 1
```text

### Overriding Defaults

You can override defaults at multiple levels:

```bash
# Global config
cat > ~/.config/blz/config.toml <<EOF
[defaults]
refresh_hours = 48
EOF

# Per-source settings
cat > ~/.local/share/dev.outfitter.blz/react/settings.toml <<EOF
[fetch]
refresh_hours = 12
EOF

# Environment variable
export BLZ_REFRESH_HOURS=6

# CLI flag (highest priority)
blz update react --refresh-hours 1
```text

## CLI State Files

BLZ maintains state files alongside configuration:

### `blz.json`

Stores CLI preferences and per-source overrides:

**Location:** Same directory as `config.toml`

**Contents:**

- CLI presentation preferences (`show`, `snippet_lines`, `score_precision`)
- Per-source `preferred_flavor` overrides
- Other UI preferences

**Example:**

```json
{
  "preferences": {
    "show": ["source", "score"],
    "snippet_lines": 5
  },
  "sources": {
    "react": {
      "preferred_flavor": "full"
    }
  }
}
```text

### `history.jsonl`

Stores search history:

**Location:** Same directory as `config.toml`

**Format:** JSON Lines (one JSON object per line)

**Example:**

```jsonl
{"timestamp":"2024-01-01T12:00:00Z","query":"react hooks","source":"react"}
{"timestamp":"2024-01-01T12:05:00Z","query":"async await","source":null}
```text

See [`blz history`](commands.md#blz-history) for working with search history.

## Common Scenarios

### Offline Mode

Disable network fetches globally:

```toml
# config.toml
[defaults]
fetch_enabled = false
```text

Or per-session:

```bash
export BLZ_FETCH_ENABLED=false
blz "query"
```text

### Aggressive Refresh

Check for updates more frequently:

```toml
# config.toml
[defaults]
refresh_hours = 6
```text

### Custom Cache Location

Move cache to different location:

```toml
# config.toml
[paths]
root = "/mnt/external/blz-cache"
```text

Or:

```bash
export BLZ_ROOT=/mnt/external/blz-cache
```text

### Per-Project Config

Use different config for different projects:

```bash
# Project A
cd ~/projects/project-a
export BLZ_CONFIG_DIR=./.blz
blz "query"

# Project B
cd ~/projects/project-b
export BLZ_CONFIG_DIR=./.blz
blz "query"
```text

### CI/CD Configuration

Non-interactive mode with JSON output:

```bash
export BLZ_FORCE_NON_INTERACTIVE=true
export BLZ_OUTPUT_FORMAT=json
export BLZ_SUPPRESS_DEPRECATIONS=true

blz "api" | jq '.results[0]'
```text

## See Also

- [Commands](commands.md) - Complete command reference
- [CLI Overview](README.md) - CLI installation and usage
- [Sources](sources.md) - Managing documentation sources
