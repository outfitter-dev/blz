# Proposal: Adjacent Help Files for CLI Commands

**Status**: Proposed
**Author**: Claude (with @galligan)
**Date**: 2025-09-30
**Related Issues**: Documentation drift, help text maintenance

## Problem Statement

Currently, blz has a documentation drift problem:

1. **CLI help text** lives in clap annotations in `crates/blz-cli/src/commands/*.rs`
2. **Detailed documentation** lives in `docs/commands/*.md`
3. **These drift apart** over time

### Evidence of Drift

- Flavor-related flags were removed from code but docs still mentioned them
- `--output` deprecation wasn't consistently documented
- Help text in CLI is terse; docs have examples but may be outdated
- No automated checking that docs match implementation

### Current Workflow Pain Points

1. **Discovery**: Developer modifying a command doesn't see the full docs
2. **Maintenance**: Must remember to update docs in separate directory
3. **Review**: PRs changing commands may not update docs
4. **Verification**: No way to know if docs match current implementation

## Proposed Solution

Adopt the **adjacent help file pattern** inspired by TypeScript projects:

```
crates/blz-cli/src/commands/
├── add.rs                # Command implementation
├── add.help.md           # Rich help content (NEW)
├── search.rs             # Command implementation
├── search.help.md        # Rich help content (NEW)
├── update.rs
├── update.help.md
└── ...
```

### Key Principles

1. **Locality**: Help lives next to code
2. **Single source of truth**: Help file is embedded in binary AND used for docs
3. **Rich content**: Markdown format allows examples, formatting, links
4. **Compile-time embedding**: Zero runtime cost via `include_str!`
5. **Automated sync**: Build process ensures docs/ stays in sync

## Implementation Details

### 1. Help File Format

Each `<command>.help.md` file follows a standard template:

```markdown
# blz <command>

Brief one-line description.

## Usage

    blz <command> <ARGS> [OPTIONS]

## Arguments

    <ARG>  Description

## Options

    --flag <VALUE>  Description
    -f, --flag      Short description

## Examples

    # Example 1
    blz <command> example

    # Example 2 with explanation
    blz <command> --flag value

## Notes

Additional information, caveats, or related commands.

## See Also

- blz other-command
- Documentation: https://...
```

### 2. Embedding in Command Code

```rust
// crates/blz-cli/src/commands/add.rs
use clap::Args;

/// Add a documentation source to your local cache
#[derive(Args)]
#[command(
    long_about = include_str!("add.help.md"),
    after_help = "Run 'blz add --help' for more details"
)]
pub struct AddArgs {
    /// Source identifier
    #[arg(
        value_name = "SOURCE",
        help = "Unique source identifier (e.g., bun, react)"
    )]
    pub source: String,

    /// Documentation URL
    #[arg(
        value_name = "URL",
        help = "URL to fetch documentation from"
    )]
    pub url: String,

    /// Add aliases during initial setup
    #[arg(
        long = "alias",
        value_name = "ALIAS",
        help = "Add an alias (can be specified multiple times)"
    )]
    pub aliases: Vec<String>,
}
```

### 3. Build Script Integration

Create `crates/blz-cli/build.rs`:

```rust
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun if help files change
    println!("cargo:rerun-if-changed=src/commands/*.help.md");

    // Copy help files to docs/commands/
    let commands_dir = Path::new("src/commands");
    let docs_dir = Path::new("../../docs/commands");

    if let Ok(entries) = fs::read_dir(commands_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(filename) = path.file_name() {
                    let dest = docs_dir.join(filename);
                    fs::copy(&path, dest).ok();
                }
            }
        }
    }
}
```

### 4. Alternative: Symlinks (Simpler)

Instead of build script, use symlinks:

```bash
# One-time setup
cd docs/commands
ln -sf ../../crates/blz-cli/src/commands/add.help.md add.md
ln -sf ../../crates/blz-cli/src/commands/search.help.md search.md
# ... etc
```

**Pros**:
- Simpler, no build step
- Changes immediately visible in both locations
- Git tracks the symlink, not duplicate content

**Cons**:
- Symlinks can be confusing on Windows
- Some editors don't handle symlinks well

## Benefits

### For Developers

✅ **Locality**: See help when editing command code
- Help file appears in same directory view
- PR diffs show help changes alongside code changes
- Easier to keep in sync

✅ **Clear ownership**: Command author owns both code and help
- Reduces "someone else will update the docs" problem
- Help is part of the command definition

✅ **Better reviews**: PRs show help changes
- Reviewers can verify help matches implementation
- Missing help updates are obvious

### For Users

✅ **Consistent help**: CLI help and web docs are identical
- No confusion from outdated docs
- Same examples work everywhere

✅ **Rich CLI help**: More detailed help in terminal
- Examples right in `--help`
- No need to visit website for basic usage

✅ **Accurate information**: Help can't drift from implementation
- Build fails if help file missing
- Easy to verify help matches code

### For Maintainers

✅ **Single source of truth**: One file to maintain
- Update help once, propagates everywhere
- No duplicate content to keep in sync

✅ **Automated verification**: Can check help completeness
- Lint rule: every command must have `.help.md`
- CI can verify help files exist

✅ **Documentation site**: Can auto-generate from help files
- Process markdown for web rendering
- Add navigation, search, etc.

## Drawbacks & Mitigations

### Drawback 1: Clap's Markdown Rendering

**Problem**: Clap doesn't natively render markdown in terminal help

**Mitigation Options**:
1. Accept plain markdown in terminal (still readable)
2. Use a help formatter crate (e.g., `clap-markdown`)
3. Strip markdown formatting for terminal, keep for docs
4. Use ANSI codes in help files for basic formatting

**Recommendation**: Start with option 1 (plain markdown), iterate if needed

### Drawback 2: Help Files Clutter src/commands/

**Problem**: More files in commands directory

**Mitigation**:
- Help files have `.help.md` extension, easy to filter
- Benefits of locality outweigh clutter
- Can use subdirectory structure if needed:
  ```
  commands/
  ├── add/
  │   ├── mod.rs
  │   └── help.md
  └── search/
      ├── mod.rs
      └── help.md
  ```

### Drawback 3: Embedding Size

**Problem**: Including full help in binary increases size

**Mitigation**:
- Help text compresses well
- Binary size increase is minimal (~few KB total)
- Can strip help in release builds if needed (not recommended)

**Analysis**:
- Current docs/ markdown: ~50KB total
- Compressed in binary: ~10-15KB
- Negligible compared to typical binary size (several MB)

## Migration Plan

### Phase 1: Pilot (1-2 commands)

1. Create `.help.md` files for 2 commands:
   - `add.help.md` (complex command, good test case)
   - `search.help.md` (most commonly used)

2. Update command structs to embed help

3. Choose approach: build script vs symlinks

4. Verify:
   - `blz add --help` shows rich content
   - `docs/commands/add.md` stays in sync
   - Build process works correctly

### Phase 2: Rollout (All commands)

1. Create `.help.md` for remaining commands
2. Update all command structs
3. Remove old standalone docs (or convert to symlinks)
4. Update contributor documentation

### Phase 3: Automation

1. Add lint rule: "Every command must have `.help.md`"
2. CI check: Verify help files exist for all commands
3. Optional: Pre-commit hook to sync docs/

### Phase 4: Enhancement (Optional)

1. Add help formatter for better terminal rendering
2. Auto-generate docs site navigation from help files
3. Add examples to help that run as integration tests

## File Structure After Migration

```
crates/blz-cli/src/commands/
├── add.rs
├── add.help.md
├── alias.rs
├── alias.help.md
├── config.rs
├── config.help.md
├── diff.rs
├── diff.help.md
├── get.rs
├── get.help.md
├── history.rs
├── history.help.md
├── list.rs
├── list.help.md
├── lookup.rs
├── lookup.help.md
├── mod.rs
├── remove.rs
├── remove.help.md
├── search.rs
├── search.help.md
├── update.rs
├── update.help.md
└── upgrade.rs
    └── upgrade.help.md

docs/commands/
├── add.md -> ../../crates/blz-cli/src/commands/add.help.md
├── alias.md -> ../../crates/blz-cli/src/commands/alias.help.md
├── search.md -> ../../crates/blz-cli/src/commands/search.help.md
└── ... (all symlinks)
```

## Example: add.help.md

```markdown
# blz add

Add a documentation source to your local cache.

## Usage

    blz add <SOURCE> <URL> [OPTIONS]

## Arguments

    <SOURCE>
        Unique identifier for this source (e.g., bun, react, node)
        Used for directory name and all references

    <URL>
        URL to fetch documentation from
        Typically ends in llms.txt or llms-full.txt

## Options

    --alias <ALIAS>
        Add an alias for this source (can be specified multiple times)
        Aliases let you reference sources by alternate names

    -y, --yes
        Skip confirmation prompt

    --quiet
        Suppress output messages

## Examples

    # Basic add
    blz add bun https://bun.sh/llms-full.txt

    # Add with aliases
    blz add react https://react.dev/llms-full.txt --alias reactjs --alias @facebook/react

    # Add multiple sources from registry
    blz lookup "javascript runtime"
    blz add bun https://bun.sh/llms-full.txt --alias bunjs

## Notes

- Source identifiers should be filesystem-safe (lowercase, hyphens okay)
- Aliases can use more relaxed formatting (e.g., @scope/package)
- BLZ automatically prefers llms-full.txt when available

## See Also

- blz alias    - Manage aliases after adding a source
- blz update   - Refresh source documentation
- blz remove   - Remove a source
- blz lookup   - Search registry for sources
```

## Comparison with Current Approach

### Current: Separate Files

```
crates/blz-cli/src/commands/add.rs
  → Short help in clap annotations
  → 5-10 word descriptions

docs/commands/add.md
  → Full documentation
  → Examples, detailed explanations
  → Often drifts from actual implementation
```

**Developer workflow**:
1. Edit command in `add.rs`
2. Remember to update `docs/commands/add.md`
3. Hope they stay in sync
4. PR reviewer may not notice doc drift

### Proposed: Adjacent Files

```
crates/blz-cli/src/commands/add.rs
  → Command implementation
  → Embeds add.help.md

crates/blz-cli/src/commands/add.help.md
  → Full rich help content
  → Examples, usage, detailed explanations
  → Automatically available in CLI and docs

docs/commands/add.md
  → Symlink to add.help.md (or copy via build)
```

**Developer workflow**:
1. Edit command in `add.rs`
2. See `add.help.md` right there in same directory
3. Update help file as part of same change
4. PR shows both code and help changes
5. Automated sync keeps docs/ up to date

## Open Questions

1. **Markdown rendering**: Accept plain markdown in CLI or add formatter?
   - **Recommendation**: Start plain, can enhance later

2. **Symlinks vs build script**: Which approach for docs/ sync?
   - **Recommendation**: Symlinks (simpler, works on most platforms)

3. **Help file naming**: `.help.md` vs `.md` vs `help.md`?
   - **Recommendation**: `.help.md` (clear intent, easy to filter)

4. **Directory structure**: Flat or nested?
   - **Recommendation**: Flat for now (only ~15 commands)

## Success Metrics

After implementation, we should see:

✅ **Reduced drift**: Help and docs always match
✅ **Faster updates**: Help updated in same commit as code
✅ **Better reviews**: PR reviewers see help changes
✅ **Improved docs**: More examples, better maintained
✅ **Developer satisfaction**: Easier to maintain commands

## References

- Pattern seen in TypeScript projects (e.g., `<command>.help.ts`)
- Similar to Rust's `include_str!` for embedding resources
- Clap documentation: https://docs.rs/clap/latest/clap/
- Inspiration: git manpages adjacent to code

## Next Steps

1. **Get approval** on approach (symlinks vs build script)
2. **Create pilot** with `add.help.md` and `search.help.md`
3. **Test integration** with clap and docs site
4. **Document pattern** for contributors
5. **Roll out** to all commands
6. **Add CI checks** to enforce pattern

## Appendix: Alternative Considered

### Approach: Keep Separate But Check

Instead of adjacent files, add CI check that docs match code.

**Why rejected**:
- Still requires maintaining two files
- Check can detect drift but doesn't prevent it
- Doesn't improve developer experience
- Doesn't make help easier to find/update
