# Next release notes (pre-cutover)

This file preserves the former `CHANGELOG.md` Unreleased section for the
release-please migration. Split or merge these notes into the first
release-please PR as needed.

## Unreleased (archived)

### Breaking Changes

- **MCP Server Command Renamed** ([BLZ-258](https://linear.app/outfitter/issue/BLZ-258)): The command to launch the MCP server has been renamed from `blz mcp` to `blz mcp-server`
  - This change allows users to add Model Context Protocol documentation as a source using the natural alias `mcp`
  - **Action Required**: Update MCP server configurations in Claude Code, Cursor, Windsurf, and other AI coding assistants
  - **Before**: `blz mcp` or `"args": ["mcp"]`
  - **After**: `blz mcp-server` or `"args": ["mcp-server"]`
  - Example configuration update:
    ```json
    {
      "mcpServers": {
        "blz": {
          "command": "blz",
          "args": ["mcp-server"]
        }
      }
    }
    ```

### Added
- **Claude Code Plugin**: Official plugin for integrating BLZ documentation search into Claude Code workflows
  - **Commands**: Single `/blz` command handling search, retrieval, and source management
  - **Agents**: `@blz:blazer` for search, retrieval, and source management workflows
  - **Skills**: `blz-docs-search` for search patterns, `blz-source-management` for source management
  - **Dependency Scanning**: Automatic discovery of documentation candidates from Cargo.toml and package.json
  - **Local Installation**: Support for local development with `/plugin install /path/to/.claude-plugin`
  - **Documentation**: Comprehensive guides in `docs/agents/claude-code.md` and plugin README
- **Table of contents enhancements**: New filtering and navigation controls for `blz toc`
  - `--limit <N>`: Trim output to first N headings
  - `--max-depth <1-6>`: Restrict results to headings at or above specified depth
  - `--filter <expr>`: Search heading paths with boolean expressions (e.g., `API AND NOT deprecated`)
  - Improved agent workflows for hierarchical document navigation
- **Unified `find` command** ([BLZ-229](https://linear.app/outfitter/issue/BLZ-229)): New command consolidating `search` and `get` with automatic pattern-based dispatch
  - **Smart routing**: Citations (e.g., `bun:120-142`) trigger retrieve mode; text queries trigger search mode
- **Heading-level filtering**: `-H` flag filters results by Markdown heading level (1-6)
    - Single level: `-H 2` (only h2)
    - Range syntax: `-H 2-4` (h2 through h4)
    - Comparison: `-H <=2` (h1 and h2)
    - List: `-H 1,3,5` (specific levels)
  - **New `level` field**: Search results now include heading level (1-6) for filtering and display
  - **Configurable defaults**: `BLZ_DEFAULT_LIMIT` environment variable controls default search limit
  - **Agent prompt**: New `blz --prompt find` provides comprehensive guidance for AI agents

### Changed
- **CLI prompts migration** ([BLZ-240](https://linear.app/outfitter/issue/BLZ-240)): Replaced `dialoguer` with `inquire` for interactive CLI prompts
  - Better API ergonomics with cleaner configuration chaining
  - Improved type safety for prompt handling
  - Enhanced features including built-in validators and autocompletion support
  - Zero breaking changes - CLI behavior remains identical for users
  - Affected commands: `blz remove`, `blz lookup`, `blz registry create-source`
- **Terminology clarity**: Renamed `blz anchors` to `blz toc` for clearer intent (table of contents)
  - Better alignment with internal types (`LlmsJson.toc`)
  - Clearer separation: `toc` for document structure, `--anchors` for anchor metadata
  - Renamed `--mappings` to `--anchors` for better clarity (old flag remains as hidden alias)
  - Backward compatibility: `blz anchors` and `--mappings` remain as hidden aliases
  - No breaking changes for existing users
- CLI: Rename `update` command to `refresh` ([BLZ-262](https://linear.app/outfitter/issue/BLZ-262))
- **Plugin Structure**: Consolidated Claude plugin assets under `.claude-plugin/` for clarity
- **Agent References**: Updated plugin commands to use `@blz:blazer` for unified documentation operations

### Deprecated
- `blz update` is now hidden and emits a warning. Use `blz refresh` instead.
- `blz search` and `blz get` are now hidden and emit deprecation warnings. Use `blz find` instead.
  - Both commands continue to work and route through `find` internally
  - Will be removed in a future major version

### Fixed
- **Language filtering consistency** ([BLZ-261](https://linear.app/outfitter/issue/BLZ-261)): Improved locale detection and fallback behavior
  - Moved default language setting from `Fetcher` to `AddRequest` for consistent application
  - Consolidated language filter logic to ensure `--no-language-filter` flag properly disables filtering
  - Added `apply_language_filter` method to centralize URL validation before downloads
  - Improved test coverage with dedicated language filtering test suite
