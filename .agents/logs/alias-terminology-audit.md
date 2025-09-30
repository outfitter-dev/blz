# Alias Terminology Audit

## Problem Statement

The term "alias" is overloaded to mean two different things in blz, causing confusion in documentation, code, and user experience.

## Two Meanings of "Alias"

### 1. Canonical Name/Identifier (Primary)
**Code**: `LlmsJson.alias` field
**What it is**: The unique, on-disk directory name for a source
**Examples**: `"bun"`, `"react"`, `"node"`
**Used in**:
- `blz add <ALIAS> <url>` ← creates this
- Directory structure: `<data_dir>/<alias>/`
- Search results JSON: `"alias": "bun"`
- Internal: unique identifier, filesystem-safe

### 2. Metadata Aliases (Secondary)
**Code**: `Source.aliases` field (Vec<String>)
**What it is**: Alternate names that resolve to the canonical name
**Examples**: `["@scope/package", "bunjs", "nodejs"]`
**Used in**:
- `blz alias add <SOURCE> <ALIAS>` ← creates these
- Resolution: `"@scope/pkg"` → `"canonical-name"`
- CLI: accepts either canonical or metadata alias for most commands

## Documentation Confusion

### docs/cli.md (Line 219-220)
```markdown
- Each entry includes: `alias`, `source` (canonical handle), `url`, ...
- When available: `etag`, `lastModified`, and `aliases` (array of metadata aliases)
```
**Problem**: Uses "alias" for primary identifier AND "aliases" for alternates

### docs/cli.md (Line 566-572)
```markdown
# `blz alias`
Manage aliases for a source. Aliases are stored in source metadata...

blz alias add <SOURCE> <ALIAS>
blz alias rm <SOURCE> <ALIAS>
```
**Problem**: The command is called `alias` but it manages metadata aliases, not the canonical alias

### docs/sources.md (Line 9)
```markdown
- An **alias** - Short name for referencing (e.g., `bun`, `node`)
```
**Problem**: Doesn't distinguish between canonical and metadata aliases

### docs/commands/get.md (Line 8)
```markdown
- `<SOURCE>`: canonical source or metadata alias
```
**Problem**: Uses "SOURCE" to mean either, unclear what the difference is

### docs/commands/search.md (Line 9)
```markdown
- `--source, -s <SOURCE>`: restrict to a source (canonical or metadata alias)
```
**Problem**: Same inconsistency

### docs/registry/lookup.md (Lines 78-80, 87-99, 129)
```markdown
> Enter alias [claude-code]:

| Name | Slug | Aliases | Description |
|------|------|---------|-------------|
| Bun | `bun` | `bun`, `bunjs` | Fast all-in-one JavaScript runtime |

- **Smart Defaults**: Suggests kebab-case slug as default alias
```
**Problem**: Uses "slug" for what we call canonical "alias", and "aliases" for metadata aliases. Then prompts for "alias" when it means canonical name.

## Code Confusion

### Storage Layer
```rust
// This is the canonical name, not an alias!
pub fn list_sources(&self) -> Vec<String>  // Returns canonical names
pub fn exists(&self, alias: &str) -> bool  // Takes canonical name
```

### Commands
```rust
// add.rs
pub async fn execute_add(alias: &str, url: &str, ...)
// "alias" here means canonical name

// alias.rs
pub fn add_alias(source: &str, new_alias: &str)
// "new_alias" here means metadata alias
// "source" means canonical name
```

### Resolver
```rust
pub fn resolve_source(storage: &Storage, requested: &str) -> Result<Option<String>>
// Returns canonical name from either canonical or metadata alias
```

## User Confusion Examples

1. **Adding a source**:
   ```bash
   blz add react https://react.dev/llms.txt
   ```
   User thinks: "I'm setting up an alias 'react'"
   Actually: Creating canonical identifier "react"

2. **Adding a metadata alias**:
   ```bash
   blz alias add react @facebook/react
   ```
   User thinks: "Wait, I already have an alias 'react', what's this?"
   Actually: Adding alternate name "@facebook/react" → "react"

3. **Searching**:
   ```bash
   blz search "hooks" --source @facebook/react
   ```
   User thinks: "Is this searching the alias or the source?"
   Actually: Resolving metadata alias to canonical name

## Proposed Terminology Fix

### Option 1: Use "Source" Everywhere (RECOMMENDED)
- `LlmsJson.alias` → `LlmsJson.source`
- `blz add <source> <url>`
- `blz alias add <source> <alias>`
- CLI: `<SOURCE>: unique source identifier (accepts source or alias)`
- Docs: "Each source has a unique **identifier** and optional **aliases**"
- **Benefits**: Matches user mental model, aligns with domain language, eliminates "alias" overloading
- **Drawback**: Need to rename existing `LlmsJson.source` field to `source_metadata` or `metadata`

### Option 2: Rename Canonical "Alias" to "Name"
- `LlmsJson.alias` → `LlmsJson.name`
- `blz add <name> <url>`
- `blz alias add <name> <alias>` (metadata alias)
- CLI: `<NAME>: unique source identifier`
- Docs: "Each source has a unique **name** and optional **aliases**"
- **Benefits**: Simple, clear, no field collision issues
- **Drawback**: Doesn't match existing mental model ("source" used throughout docs)

### Option 3: Keep but Clarify with "Canonical"
- `LlmsJson.alias` stays (call it "canonical alias" in docs)
- `Source.aliases` stays (call them "alternate aliases" or "metadata aliases")
- `blz add <canonical-alias> <url>`
- `blz alias add <canonical-alias> <alternate-alias>`
- Update all docs to always say "canonical alias" vs "metadata alias"
- **Benefits**: Minimal code changes
- **Drawback**: Still confusing, requires constant qualification

### Option 4: Registry-Inspired (Slug + Aliases)
- `LlmsJson.alias` → `LlmsJson.slug`
- `blz add <slug> <url>`
- `blz alias add <slug> <alias>`
- Docs: "Each source has a unique **slug** (kebab-case identifier) and optional **aliases**"
- **Benefits**: Matches registry pattern, clear kebab-case expectation
- **Drawback**: "Slug" is web/URL terminology, may not resonate with all users

## Breaking Changes Analysis

### Minimal (Option 3 - Documentation Only)
- No code changes
- Only update documentation terminology
- Add glossary/terminology section
- Still confusing but at least documented

### Medium (Option 2 - Name / Option 4 - Slug)
- Rename `LlmsJson.alias` → `LlmsJson.name` or `slug`
- Update all CLI help text
- Update all documentation
- Migration: keep deserializing both fields with `#[serde(alias = "alias")]`
- JSON output changes (breaking for API consumers)

### Maximum (Option 1 - Source) - RECOMMENDED
- Rename `LlmsJson.alias` → `LlmsJson.source`
- Rename `LlmsJson.source` → `LlmsJson.metadata` (avoid field collision)
- Rename all function parameters: `alias: &str` → `source: &str`
- Update all CLI help text to use "source" consistently
- Update all documentation to distinguish "source" from "alias"
- Most intuitive for users despite larger change surface
- Eliminates confusion at the root cause

## Recommendation

**Use "source" as the canonical term everywhere** to eliminate confusion:

**Benefits**:
- Consistent with existing user mental model: "What source are you searching?"
- Clear distinction: **source** (unique identifier) vs **aliases** (alternate names)
- Aligns with domain language: "documentation source", "source code"
- Eliminates overloading of "alias" term completely
- Simpler to explain: "Every source has a unique identifier and optional aliases"

**Key Design Principles**:
1. **Default to "source" everywhere**: All commands, documentation, and code should use "source" as the primary term
2. **Aliases resolve to sources**: Any alias (metadata alias, registry alias, etc.) should resolve to a canonical source identifier
3. **Search should work with any alias**: Users should be able to search/reference sources using any registered alias
4. **Avoid overloading "alias"**: The term "alias" should ONLY refer to alternate names, never the canonical identifier

**Implementation**:
1. Rename `LlmsJson.alias` → `LlmsJson.source` (with `#[serde(alias = "alias")]` for backward compat)
2. Update all function signatures: `alias: &str` → `source: &str`
3. Keep resolver working with both canonical source and any alias
4. **Add `--alias` flag to `blz add` command** to specify aliases during initial setup
5. Update CLI help text to use "source" consistently
6. Update all documentation to use "source" and "alias" correctly
7. Add migration note in CHANGELOG

**Migration path**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsJson {
    /// Unique identifier for this source.
    ///
    /// This is the canonical source identifier used for directory names,
    /// search results, and internal references. Should be URL-safe and
    /// filesystem-safe (typically kebab-case).
    #[serde(alias = "alias")]  // Accept old "alias" field for compatibility
    pub source: String,

    /// Source metadata including alternate names (aliases).
    pub source_metadata: Source,
    // ...
}
```

This allows old JSON files to be read (they have "alias" field) while new ones write "source".

**Note on "source" field collision**: The current `LlmsJson.source` field (which is a `Source` struct) should be renamed to `source_metadata` or `metadata` to avoid naming conflict with the new `source: String` field.

## CLI Command Changes

### `blz add` - New Signature
```bash
blz add <SOURCE> <URL> [OPTIONS]

Arguments:
  <SOURCE>  Unique identifier for this source (e.g., bun, react)
  <URL>     URL to fetch documentation from

Options:
  --alias <ALIAS>    Add an alias for this source (can be specified multiple times)
  -y, --yes          Skip confirmation prompts
  --quiet            Suppress output
```

**Examples**:
```bash
# Basic add
blz add bun https://bun.sh/llms-full.txt

# Add with aliases
blz add react https://react.dev/llms-full.txt --alias @facebook/react --alias reactjs

# Add with multiple aliases from registry discovery
blz add vue https://vuejs.org/llms-full.txt --alias vuejs --alias @vue/core
```

### `blz alias` - Simplified Single Command
```bash
blz alias <SOURCE> [OPTIONS]

Options:
  --alias <ALIAS>      Add an alias (can be specified multiple times)
  --remove <ALIAS>     Remove a specific alias
  --remove all         Remove all aliases
  --list               List all aliases for this source (default if no flags)

# If run with no flags, defaults to --list
blz alias <SOURCE>   # Same as: blz alias <SOURCE> --list
```

**Examples**:
```bash
# Add aliases
blz alias bun --alias bunjs --alias @oven/bun

# Remove specific alias
blz alias bun --remove bunjs

# Remove all aliases
blz alias bun --remove all

# List aliases (explicit or default)
blz alias bun --list
blz alias bun        # Same as above

# Combine operations in one command
blz alias bun --alias new-alias --remove old-alias
```

**Additional convenience**:
```bash
# List aliases for ALL sources
blz alias

# Example output:
# bun: bunjs, @oven/bun
# react: @facebook/react, reactjs
# node: nodejs, js
```

**Benefits over nested subcommands**:
- Single command to remember: `blz alias`
- Flags make intent clear
- Supports batch operations naturally
- More compact than `blz alias add/rm`
- Follows common CLI patterns (git config, npm config, etc.)
- Default behavior (no args) is useful: shows all aliases at a glance

### Implementation Notes

#### `blz add` with `--alias`

**CLI Argument**:
```rust
#[derive(Args)]
pub struct AddArgs {
    /// Unique source identifier
    pub source: String,

    /// URL to fetch documentation from
    pub url: String,

    /// Aliases for this source (can be specified multiple times)
    #[arg(long = "alias", value_name = "ALIAS")]
    pub aliases: Vec<String>,

    // ... other flags
}
```

**Execution Flow**:
1. Validate source identifier (must be filesystem-safe)
2. Validate each alias (can be more relaxed, e.g., `@scope/package`)
3. Fetch and parse documentation
4. Create `Source` struct with `aliases` field populated
5. Save to both `llms.json` and `metadata.json` with aliases included
6. User can immediately search using source identifier or any alias

#### `blz alias` Unified Command

**CLI Argument**:
```rust
#[derive(Args)]
pub struct AliasArgs {
    /// Source to manage aliases for (if omitted, lists all)
    pub source: Option<String>,

    /// Add aliases (can be specified multiple times)
    #[arg(long = "alias", value_name = "ALIAS")]
    pub add: Vec<String>,

    /// Remove specific alias or "all"
    #[arg(long = "remove", value_name = "ALIAS")]
    pub remove: Option<String>,

    /// List aliases (default if no other flags specified)
    #[arg(long = "list")]
    pub list: bool,
}
```

**Execution Logic**:
```rust
pub async fn execute(args: AliasArgs) -> Result<()> {
    match args.source {
        None => {
            // List all sources with their aliases
            list_all_aliases()
        }
        Some(source) => {
            if args.list || (args.add.is_empty() && args.remove.is_none()) {
                // List aliases for specific source
                list_source_aliases(&source)
            } else {
                // Perform add/remove operations
                if !args.add.is_empty() {
                    add_aliases(&source, &args.add)?;
                }
                if let Some(to_remove) = args.remove {
                    if to_remove == "all" {
                        remove_all_aliases(&source)?;
                    } else {
                        remove_alias(&source, &to_remove)?;
                    }
                }
                Ok(())
            }
        }
    }
}
```

**Benefits**:
- Single command with clear flags for all alias operations
- Supports batch add/remove in one invocation
- Default behavior is useful (list all or list specific)
- Natural CLI pattern that's easy to remember

## Files Requiring Updates

### Code
- `crates/blz-core/src/types.rs` - LlmsJson struct, rename fields
- `crates/blz-core/src/storage.rs` - Function parameters (alias → source)
- `crates/blz-cli/src/commands/add.rs` - Add `--alias` flag support, update parameter names
- `crates/blz-cli/src/commands/*.rs` - All command implementations (parameter renaming)
- `crates/blz-cli/src/cli.rs` - CLI argument names and help text
- `crates/blz-cli/src/utils/resolver.rs` - Documentation updates

### Documentation
- `README.md` - All examples and API references
- `docs/sources.md` - Terminology section
- `docs/cli.md` - All command docs
- `docs/getting-started.md` - Examples
- `docs/commands/*.md` - Individual command docs
- `docs/registry/lookup.md` - Terminology alignment

### Tests
- All integration tests using "alias" variable names
- Test assertions checking JSON output

## Command Design Summary

### Why This Design Works

The unified `blz alias` command with flags is superior to nested subcommands for several reasons:

1. **Cognitive Load**: One command to remember (`blz alias`) instead of two (`blz alias add`, `blz alias rm`)

2. **Discoverability**: `blz alias --help` shows all operations in one place

3. **Flexibility**: Supports batch operations naturally:
   ```bash
   blz alias bun --alias foo --alias bar --remove old
   ```

4. **Progressive Complexity**:
   - Simple: `blz alias bun` (list aliases)
   - Medium: `blz alias bun --alias new` (add one)
   - Advanced: `blz alias bun --alias a --alias b --remove c` (batch)

5. **Familiar Pattern**: Similar to `git config`, `npm config`, `docker tag`, etc.

6. **Default Behavior**: Running with no args (`blz alias`) or no flags (`blz alias bun`) does something useful (lists)

### Command Comparison

```bash
# OLD (nested subcommands)
blz alias add bun bunjs
blz alias add bun @oven/bun
blz alias rm bun old-alias

# NEW (unified with flags)
blz alias bun --alias bunjs --alias @oven/bun --remove old-alias

# Listing (NEW only)
blz alias              # List all
blz alias bun          # List for source
```

## Glossary to Add

After fixing terminology, add this to docs:

```markdown
## Terminology

- **Source**: The unique identifier for a documentation source (e.g., `bun`, `react`, `node`). Used as the directory name, in search results, and for all internal references. Must be URL-safe and filesystem-safe (typically kebab-case).
- **Alias**: An alternate name that resolves to a source identifier (e.g., `@scope/package` → `react`, `bunjs` → `bun`). Sources can have multiple aliases. Managed via `blz alias` command.
- **URL**: The location where documentation is fetched from (e.g., `https://bun.sh/llms-full.txt`).
- **Registry alias**: An alias defined in the built-in registry (e.g., registry defines `bunjs` as an alias for `bun` source).
- **Metadata alias**: An alias you add locally via `blz alias add` (e.g., `@facebook/react` → `react`).

**Key principle**: "source" = canonical identifier, "alias" = alternate name. Never use "alias" to mean the canonical identifier.
```

## Example User Flows with New Terminology

### Adding a source
```bash
# Add a source with identifier "bun"
blz add bun https://bun.sh/llms-full.txt

# Add with aliases in one step
blz add bun https://bun.sh/llms-full.txt --alias bunjs --alias @oven/bun

# "bun" is now the canonical source identifier
# Can be referenced via: bun, bunjs, @oven/bun
```

### Managing aliases
```bash
# Add aliases to an existing source
blz alias bun --alias runtime-alt --alias @scope/bun

# List aliases for a source
blz alias bun
# Output: bunjs, @oven/bun, runtime-alt, @scope/bun

# List all sources with their aliases
blz alias
# Output:
# bun: bunjs, @oven/bun, runtime-alt, @scope/bun
# react: @facebook/react, reactjs

# Remove specific alias
blz alias bun --remove runtime-alt

# Remove all aliases
blz alias bun --remove all

# Combine operations
blz alias bun --alias new-one --remove old-one
```

### Searching with sources or aliases
```bash
# All of these work (resolve to source "bun")
blz search "runtime" --source bun
blz search "runtime" --source bunjs
blz search "runtime" --source @oven/bun

# Shorthand also works
blz "runtime" bun
blz "runtime" bunjs
blz "runtime" @oven/bun
```

### Registry lookup
```bash
# Lookup suggests source identifier + any registry aliases
$ blz lookup "bun"
Found 1 match:

1. Bun (bun)
   Registry aliases: bunjs
   Fast all-in-one JavaScript runtime
   https://bun.sh/llms-full.txt

# Add using suggested source identifier (registry aliases automatically included)
blz add bun https://bun.sh/llms-full.txt

# Or add with custom aliases in addition to registry ones
blz add bun https://bun.sh/llms-full.txt --alias @oven/bun --alias bun-runtime

# Results in source "bun" with aliases: bunjs (from registry), @oven/bun, bun-runtime
```

## Current State & Impact on Codebase

### The "Spaghetti Code" Problem

The current terminology confusion has led to inconsistent patterns throughout the codebase:

1. **Inconsistent variable naming**:
   ```rust
   // Sometimes "alias" means canonical identifier
   pub fn execute_add(alias: &str, url: &str, ...)

   // Sometimes "source" means canonical identifier
   pub fn add_alias(source: &str, new_alias: &str)

   // Sometimes "source" means Source struct
   pub source: Source,
   ```

2. **Confusing function signatures**:
   - `storage.exists(alias)` - takes canonical, named "alias"
   - `resolver.resolve_source(requested)` - takes either, returns canonical
   - `add_alias(source, new_alias)` - source = canonical, new_alias = metadata alias

3. **Documentation inconsistency**:
   - Some places say "source or alias"
   - Some places say "canonical alias or metadata alias"
   - Some places just say "source" but mean "canonical identifier"
   - Registry docs use "slug" for what we call "alias"

4. **User experience confusion**:
   - `blz add <alias>` - creating canonical identifier, not an "alias"
   - `blz alias add` - adding metadata aliases, not managing the canonical
   - `--source` flag accepts either source or alias (unclear without docs)

### Why "Source" Fixes This

By standardizing on "source" as the canonical term:

1. **Code clarity**: `pub fn add(source: &str, url: &str)` - clear what you're adding
2. **Consistent naming**: `source: String` for identifier, `aliases: Vec<String>` for alternates
3. **User mental model**: "I'm adding a source" vs "I'm adding an alias to a source"
4. **Documentation simplicity**: "source identifier (or alias)" instead of "canonical alias or metadata alias"
5. **Registry alignment**: Registry shows "source" + "aliases", not "slug" + "aliases"

### Migration Priority

1. **High Priority** - Core types and storage layer (eliminates root confusion)
2. **Medium Priority** - CLI commands and help text (user-facing clarity)
3. **Lower Priority** - Documentation updates (can happen incrementally)

The change is invasive but necessary to eliminate the terminology debt that's making the codebase harder to maintain and understand.
