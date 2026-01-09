# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Release-please will manage this file after the migration lands. Archived
pre-cutover notes live in `docs/release/next-release-notes.md`.

## [1.5.5](https://github.com/outfitter-dev/blz/compare/v1.5.4...v1.5.5) (2026-01-09)


### Bug Fixes

* **ci:** build linux on ubuntu-22.04 ([#417](https://github.com/outfitter-dev/blz/issues/417)) ([84ad65f](https://github.com/outfitter-dev/blz/commit/84ad65f1145667e1859a75dbbeafbd754c0c455a))

## [1.5.4](https://github.com/outfitter-dev/blz/compare/v1.5.3...v1.5.4) (2026-01-09)


### Bug Fixes

* **mcp:** add crates.io description ([#415](https://github.com/outfitter-dev/blz/issues/415)) ([f13422c](https://github.com/outfitter-dev/blz/commit/f13422ccc4d81578808e4f0a1aa7c03eefd5f3d3))


### Documentation

* **workflow:** refresh release checklist ([#414](https://github.com/outfitter-dev/blz/issues/414)) ([896d277](https://github.com/outfitter-dev/blz/commit/896d27702331821103016ca935bf90988b64028e))

## [1.5.3](https://github.com/outfitter-dev/blz/compare/v1.5.2...v1.5.3) (2026-01-08)


### Bug Fixes

* **release:** wait on crates.io index + pass homebrew shas ([#412](https://github.com/outfitter-dev/blz/issues/412)) ([1f14b50](https://github.com/outfitter-dev/blz/commit/1f14b503abf13cf98297b702f326b3f8404bd0a5))

## [1.5.2](https://github.com/outfitter-dev/blz/compare/v1.5.1...v1.5.2) (2026-01-08)


### Bug Fixes

* **ci:** use macos-14 for darwin builds ([#410](https://github.com/outfitter-dev/blz/issues/410)) ([196eb8b](https://github.com/outfitter-dev/blz/commit/196eb8b85d294616899d0c3e8950b9a138d480d8))
* **mcp:** default prompts capability fields ([#411](https://github.com/outfitter-dev/blz/issues/411)) ([7b8094c](https://github.com/outfitter-dev/blz/commit/7b8094c1ea170c70f47957088c85b8bb02e53989))
* **release:** auto-label homebrew tap PRs ([#408](https://github.com/outfitter-dev/blz/issues/408)) ([e20abdf](https://github.com/outfitter-dev/blz/commit/e20abdf444fd83d931987ebeda25fa3adba00872))

## [1.5.1](https://github.com/outfitter-dev/blz/compare/v1.5.0...v1.5.1) (2026-01-07)


### Bug Fixes

* **release:** disable draft releases ([#403](https://github.com/outfitter-dev/blz/issues/403)) ([d9dc1a2](https://github.com/outfitter-dev/blz/commit/d9dc1a227a418f2d111a64e31e757ff5d6a70d2c))
* **release:** sync Cargo.lock on release PRs ([#406](https://github.com/outfitter-dev/blz/issues/406)) ([7cf94f0](https://github.com/outfitter-dev/blz/commit/7cf94f071ebf8bbf4a2a78a74ba061838640064b))

## [1.5.0](https://github.com/outfitter-dev/blz/compare/v1.3.0...v1.5.0) (2026-01-07)


### ⚠ BREAKING CHANGES

* **cli:** MCP server command renamed from `blz mcp` to `blz mcp-server`

### Features

* **cli:** add --reindex flag to refresh command [BLZ-265] ([#329](https://github.com/outfitter-dev/blz/issues/329)) ([2739ce7](https://github.com/outfitter-dev/blz/commit/2739ce75580b6676fee64d12a7315d0bd9ea0064))
* **cli:** add boolean filtering to toc ([#320](https://github.com/outfitter-dev/blz/issues/320)) ([a48c2cb](https://github.com/outfitter-dev/blz/commit/a48c2cb7a62d26c53e151d13a535a00ae6f2da0a))
* **cli:** add pagination state to toc command [BLZ-249] ([#325](https://github.com/outfitter-dev/blz/issues/325)) ([437b93b](https://github.com/outfitter-dev/blz/commit/437b93bb36462593fbf0a7e7b75d577744096c3f))
* **cli:** add pagination state to toc command [BLZ-250] ([#324](https://github.com/outfitter-dev/blz/issues/324)) ([f357e6f](https://github.com/outfitter-dev/blz/commit/f357e6f548beafc5a66d6e1997dcd9bc8638ffef))
* **cli:** add toc limit and depth controls ([#318](https://github.com/outfitter-dev/blz/issues/318)) ([04b4729](https://github.com/outfitter-dev/blz/commit/04b47290344004abf435529afbab76b99961de85))
* **cli:** add unified find command with pattern-based dispatch ([#339](https://github.com/outfitter-dev/blz/issues/339)) ([bcb94c6](https://github.com/outfitter-dev/blz/commit/bcb94c689a7151bb363df598830ce7eab0d08dab))
* **cli:** enhance toc with heading level operators and tree view [BLZ-256] ([#323](https://github.com/outfitter-dev/blz/issues/323)) ([207245e](https://github.com/outfitter-dev/blz/commit/207245e70db60d11181e38e3da8742910086b53a))
* **cli:** make --filter flag extensible for future filters [BLZ-266] ([#330](https://github.com/outfitter-dev/blz/issues/330)) ([6c77420](https://github.com/outfitter-dev/blz/commit/6c7742008950c93436e99168f4dcb2ba7c6ca093))
* **cli:** rename anchors command to toc ([#317](https://github.com/outfitter-dev/blz/issues/317)) ([b814b45](https://github.com/outfitter-dev/blz/commit/b814b459aeee41ffe5175bde28c64712b577544d))
* **cli:** show filter status and reason in info command [BLZ-267] ([#331](https://github.com/outfitter-dev/blz/issues/331)) ([55218af](https://github.com/outfitter-dev/blz/commit/55218af5ed6e48bc764f9422101a63bccd878adb))
* **cli:** support boolean toc filters ([#343](https://github.com/outfitter-dev/blz/issues/343)) ([3beb640](https://github.com/outfitter-dev/blz/commit/3beb640b34ebda76fd09ab0ff57099b8c2ecc2b9))
* **core:** normalize heading display and search aliases [BLZ-243] ([#314](https://github.com/outfitter-dev/blz/issues/314)) ([7980d4a](https://github.com/outfitter-dev/blz/commit/7980d4ad36152cc2fd4f4e902e7fb32fb056d130))
* **core:** persist language filter preference per-source [BLZ-263] ([#327](https://github.com/outfitter-dev/blz/issues/327)) ([ca12f56](https://github.com/outfitter-dev/blz/commit/ca12f56a477795e8fa6415ee52ef3bb73453a87c))
* **mcp:** simplify claude plugin to single command and agent ([#338](https://github.com/outfitter-dev/blz/issues/338)) ([8ebd32b](https://github.com/outfitter-dev/blz/commit/8ebd32bf66fd779e12e187bbe69367f261399571))
* **search:** add headings-only flag [BLZ-228] ([#316](https://github.com/outfitter-dev/blz/issues/316)) ([c9fb032](https://github.com/outfitter-dev/blz/commit/c9fb0327e53649a6a91818ae1219086e006dd25e))
* **search:** boost heading matches with # prefix ([#315](https://github.com/outfitter-dev/blz/issues/315)) ([7f9152b](https://github.com/outfitter-dev/blz/commit/7f9152b012e49ff7cb3e9bc95301f11a79c4561d))


### Bug Fixes

* **add:** improve llms.txt resolution guidance ([#387](https://github.com/outfitter-dev/blz/issues/387)) ([9c2a0ff](https://github.com/outfitter-dev/blz/commit/9c2a0ff60702544666cecb60c2f1fead04b52e37))
* **ci:** resolve clippy and release-please token ([#391](https://github.com/outfitter-dev/blz/issues/391)) ([fd79c53](https://github.com/outfitter-dev/blz/commit/fd79c53b67510eb0e00d4ce603f3455abb577e9c))
* **ci:** skip claude review for bot-created PRs ([#367](https://github.com/outfitter-dev/blz/issues/367)) ([57e942a](https://github.com/outfitter-dev/blz/commit/57e942a513997e12cd915f14f83d0cd26436817c))
* **cli:** allow multiple inputs to find ([#369](https://github.com/outfitter-dev/blz/issues/369)) ([6569a57](https://github.com/outfitter-dev/blz/commit/6569a5743c77557a8abce003f680b326e756388e))
* **cli:** apply language filtering in refresh command [BLZ-264] ([#328](https://github.com/outfitter-dev/blz/issues/328)) ([eb474f0](https://github.com/outfitter-dev/blz/commit/eb474f0a814eb80b214a58a716114b3df6cbfb2e))
* **cli:** improve shell completions and docs ([#388](https://github.com/outfitter-dev/blz/issues/388)) ([6efee25](https://github.com/outfitter-dev/blz/commit/6efee25ae0058c07e9e9782bc4d99f24cad4d035))
* **cli:** include headings count in info output [BLZ-152] ([#273](https://github.com/outfitter-dev/blz/issues/273)) ([80870dd](https://github.com/outfitter-dev/blz/commit/80870dd72bd9c51cc58829627f2521109246367e))
* **cli:** rename mcp command to mcp-server, allow mcp as source alias [BLZ-258] ([#333](https://github.com/outfitter-dev/blz/issues/333)) ([9997dfd](https://github.com/outfitter-dev/blz/commit/9997dfd03b587706c619178a4bcd121711380c31))
* **core:** improve language filtering with hybrid url+text detection [BLZ-236] ([#302](https://github.com/outfitter-dev/blz/issues/302)) ([93ee123](https://github.com/outfitter-dev/blz/commit/93ee123e3b14d3cbb6a996a7d137b1464237c2a6))
* **core:** refactor url resolver error message ([#390](https://github.com/outfitter-dev/blz/issues/390)) ([dbe585f](https://github.com/outfitter-dev/blz/commit/dbe585f5d3563cff82e2b47fe4a1fedf68c043ab))
* hook syntax error and skill MCP documentation ([#365](https://github.com/outfitter-dev/blz/issues/365)) ([4c888b6](https://github.com/outfitter-dev/blz/commit/4c888b6b86091730477b0f40317c332959f50088))
* **release:** enable cargo-workspace plugin ([#399](https://github.com/outfitter-dev/blz/issues/399)) ([b3d4df8](https://github.com/outfitter-dev/blz/commit/b3d4df871beedc79522c418a6e9cbaa088f08775))
* **release:** switch release-please to node strategy ([#400](https://github.com/outfitter-dev/blz/issues/400)) ([4aa010a](https://github.com/outfitter-dev/blz/commit/4aa010a578b8a8519d1f88dada2e900c0c7f09c0))
* **tests:** improve test error handling with panic instead of assert(false) ([#370](https://github.com/outfitter-dev/blz/issues/370)) ([7361c61](https://github.com/outfitter-dev/blz/commit/7361c61bc2b675d5894c9ca78b018eae7f7237af))
* unblock v1.3 crate publishing ([#298](https://github.com/outfitter-dev/blz/issues/298)) ([fd64be7](https://github.com/outfitter-dev/blz/commit/fd64be70c85cc6647addf43405b0cf1e3f2ea884))


### Performance

* **dx:** optimize build and test performance [BLZ-237] ([#303](https://github.com/outfitter-dev/blz/issues/303)) ([e97dc67](https://github.com/outfitter-dev/blz/commit/e97dc67017dbebb74e060df0932b59a3662a5691))
* **dx:** Optimize git hooks for faster development workflow [BLZ-235] ([#301](https://github.com/outfitter-dev/blz/issues/301)) ([a9bf0f3](https://github.com/outfitter-dev/blz/commit/a9bf0f3d294934f52bbe0f090cd36485cef5ca9e))
* **hooks:** defer fetcher network tests to CI ([#382](https://github.com/outfitter-dev/blz/issues/382)) ([b1646a6](https://github.com/outfitter-dev/blz/commit/b1646a6b5a42b9b58c66a99b9a36de69b4d0d694))


### Refactoring

* **cli:** migrate prompts from dialoguer to inquire [BLZ-240] ([#311](https://github.com/outfitter-dev/blz/issues/311)) ([6edc3f8](https://github.com/outfitter-dev/blz/commit/6edc3f81dd7f530fa4a404116dec3d5328e46072))
* **cli:** rename --mappings to --anchors in toc command ([#322](https://github.com/outfitter-dev/blz/issues/322)) ([830e78b](https://github.com/outfitter-dev/blz/commit/830e78b305295fa9e2c570fe6bdd6b89e2a9508a))
* **cli:** rename update to refresh command [BLZ-262] ([#326](https://github.com/outfitter-dev/blz/issues/326)) ([f0ad6bb](https://github.com/outfitter-dev/blz/commit/f0ad6bb0d1c8b5f8f2cebbe03afdcecb5a5ca1a6))
* **core:** extract refresh helpers for MCP reuse ([#374](https://github.com/outfitter-dev/blz/issues/374)) ([3107eee](https://github.com/outfitter-dev/blz/commit/3107eee30b029c6f5835608a19f4af32989eefdf))
* **mcp:** add action-based find tool ([#375](https://github.com/outfitter-dev/blz/issues/375)) ([8acf93e](https://github.com/outfitter-dev/blz/commit/8acf93e123f303e525eb5ccf2852dc2c79043a88))
* **mcp:** add blz tool for source actions ([#377](https://github.com/outfitter-dev/blz/issues/377)) ([b0bb471](https://github.com/outfitter-dev/blz/commit/b0bb4711b8e40f677eef4c36df4bfe5d158e903f))


### Documentation

* add comprehensive release flow migration plan ([#344](https://github.com/outfitter-dev/blz/issues/344)) ([d8b2888](https://github.com/outfitter-dev/blz/commit/d8b2888756bcd9ec18deef89084d7b23948fb3be))
* add language filtering migration guide [BLZ-268] ([#332](https://github.com/outfitter-dev/blz/issues/332)) ([073f42d](https://github.com/outfitter-dev/blz/commit/073f42dd08a276165c945d836df0ad1e603c963f))
* **changelog:** add unified find command [BLZ-229] ([8e63a6d](https://github.com/outfitter-dev/blz/commit/8e63a6de6f60783d3c24c588fe1fe717069e82ef))
* **cli:** document toc filtering ([#321](https://github.com/outfitter-dev/blz/issues/321)) ([9708486](https://github.com/outfitter-dev/blz/commit/97084860f89142418a45ce6ccebe9eaede58cd4c))
* **factory:** add Factory command templates [BLZ-138] ([#275](https://github.com/outfitter-dev/blz/issues/275)) ([0842935](https://github.com/outfitter-dev/blz/commit/0842935eabd9985fd23835c394eedf40db7cc24d))
* improve MCP documentation ([#299](https://github.com/outfitter-dev/blz/issues/299)) ([2195a92](https://github.com/outfitter-dev/blz/commit/2195a9219337085f529c1ca8f2bcbd87235a4fa1))
* **release:** align docs with release-please ([#350](https://github.com/outfitter-dev/blz/issues/350)) ([0d8a9b9](https://github.com/outfitter-dev/blz/commit/0d8a9b91cea2462a372f5b9e734de3e15ede7f3d))
* **release:** align release-please docs ([#381](https://github.com/outfitter-dev/blz/issues/381)) ([e9dc394](https://github.com/outfitter-dev/blz/commit/e9dc394451edede34dbf6274f21e65f43f737792))
* **release:** archive prerelease notes ([#345](https://github.com/outfitter-dev/blz/issues/345)) ([b0b704f](https://github.com/outfitter-dev/blz/commit/b0b704ff82ef2d3d9f0bbde9da3ab98b20c0fcde))
* **release:** document release-please flow and archive legacy workflows ([#354](https://github.com/outfitter-dev/blz/issues/354)) ([f82c177](https://github.com/outfitter-dev/blz/commit/f82c1770031541942d1864374484a0006660d884))
* **release:** draft 1.4.0 changelog ([#378](https://github.com/outfitter-dev/blz/issues/378)) ([d207ef8](https://github.com/outfitter-dev/blz/commit/d207ef8778f21aca42945e263e5b9509ab0fd06e))
* replace deprecated 'blz get' with 'blz find' in documentation ([#364](https://github.com/outfitter-dev/blz/issues/364)) ([a6023a3](https://github.com/outfitter-dev/blz/commit/a6023a3890aeb5caf33cfd785320744713950d75))
* streamline blazer.md and use-blz.md for agents ([#366](https://github.com/outfitter-dev/blz/issues/366)) ([6a49f3c](https://github.com/outfitter-dev/blz/commit/6a49f3c377b3c955b07f4492af89611f8703e5fc))
* **workflows:** align version management with release-please ([#380](https://github.com/outfitter-dev/blz/issues/380)) ([7b097b9](https://github.com/outfitter-dev/blz/commit/7b097b94312bc621cc7d2b76f8d74638b4deb8e8))

## [Unreleased]

## [1.4.0] - 2026-01-05

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
- **MCP tool consolidation** ([BLZ-297](https://linear.app/outfitter/issue/BLZ-297)): MCP tools are now `find` + `blz` with action-based dispatch; legacy tool names are removed.
  - Previous tool names: `blz_find`, `blz_list_sources`, `blz_add_source`, `blz_run_command`, `blz_learn`
  - New tool surface:
    - `find` actions: `search`, `get`, `toc`
    - `blz` actions: `list`, `add`, `remove`, `refresh`, `info`, `validate`, `history`, `help`
  - Migration examples:
    ```json
    {"tool":"blz_find","query":"async patterns"}
    {"tool":"find","action":"search","query":"async patterns"}

    {"tool":"blz_add_source","alias":"bun"}
    {"tool":"blz","action":"add","alias":"bun"}

    {"tool":"blz_run_command","command":"validate","alias":"bun"}
    {"tool":"blz","action":"validate","alias":"bun"}

    {"tool":"blz_learn"}
    {"tool":"blz","action":"help"}
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

## [1.3.0] - 2025-10-18

### Added
- **MCP Server v1.0** ([BLZ-206](https://linear.app/outfitter/issue/BLZ-206)): Native Rust-based Model Context Protocol server (`blz mcp`)
  - Sub-50ms search latency with direct `blz-core` integration (P50: 0.177ms, P95: 0.42ms) ([BLZ-208](https://linear.app/outfitter/issue/BLZ-208))
  - Unified `find` tool for search and snippet retrieval with context modes (none, symmetric, all) ([BLZ-208](https://linear.app/outfitter/issue/BLZ-208))
  - **Response format optimization**: `format` parameter on `find` tool with concise/detailed modes for 30-65% token savings
  - Source management tools: `list-sources`, `source-add` ([BLZ-209](https://linear.app/outfitter/issue/BLZ-209))
  - Read-only diagnostic commands via `run-command` whitelist ([BLZ-210](https://linear.app/outfitter/issue/BLZ-210))
  - Embedded learning resources via `learn-blz` prompts ([BLZ-210](https://linear.app/outfitter/issue/BLZ-210))
  - Custom `blz://` URI resources for sources and registry ([BLZ-211](https://linear.app/outfitter/issue/BLZ-211))
  - Interactive documentation discovery with `discover-docs` prompt ([BLZ-212](https://linear.app/outfitter/issue/BLZ-212))
  - <1 KB handshake payload for efficient agent integration
  - Security: Read-only by default, whitelisted commands, path sanitization
  - Performance targets validated: Search < 10ms P50 (58x faster), < 50ms P95 (119x faster)
  - Comprehensive documentation: Setup guides for Claude Code and Cursor, tool reference, security review

### Documentation
- **MCP Server documentation** ([BLZ-215](https://linear.app/outfitter/issue/BLZ-215)): Comprehensive guides for setup and usage
  - Claude Desktop integration examples
  - Tool reference with JSON-RPC examples
  - Troubleshooting and performance tuning guides

## [1.2.0] - 2025-10-16

### Added
- **Multi-source, multi-range `blz get`** ([BLZ-199](https://linear.app/outfitter/issue/BLZ-199)): Dramatically improved ergonomics for retrieving documentation spans
  - **Multiple ranges from same source**: `blz get bun:120-142,200-210,300-350 --json` returns all spans in one call
  - **Multiple sources in one command**: `blz get bun:120-142 turbo:50-75 react:200-220 --json` for cross-library comparisons
  - **Matches search output**: Copy `alias:lines` directly from `blz search` JSON into `blz get` for seamless workflows
  - **Consistent JSON schema**: All responses use `requests[]` array structure, whether fetching one span or many sources
  - **Performance**: Single round-trip instead of multiple CLI invocations for agents and scripts

### Changed
- **`blz get` JSON schema** ([BLZ-199](https://linear.app/outfitter/issue/BLZ-199)): New structure optimized for multi-source, multi-range retrieval
  - **Top-level `requests[]` array**: Each entry represents one source with its spans
  - **Single span**: `snippet` + `lineStart`/`lineEnd` fields directly on request
  - **Multiple spans**: `ranges[]` array with separate snippets for each span
  - **Execution metadata**: `executionTimeMs` and `totalSources` at response root
  - **Migration**: Scripts should update from legacy `.content` field to `requests[0].snippet` or iterate `requests[0].ranges[]`
- **Snippet invariants** ([BLZ-163](https://linear.app/outfitter/issue/BLZ-163)): Enforced with `NonZeroUsize` line numbers and validated constructors
  - Guarantees `line_start <= line_end` at compile time
  - Eliminates impossible zero ranges and invalid spans
  - Foundation for future CLI enhancements
- **CLI help organization**: Commands and flags now appear in logical priority order for better discoverability
  - Core commands (add, search, get, list) appear first in help output
  - Related flags grouped together: context flags (30-34), format flags (40-44), pagination flags (50-55)
  - Deprecated flags hidden but still functional for backward compatibility

### Documentation & Prompts
- **Multi-range workflow guidance** ([BLZ-200](https://linear.app/outfitter/issue/BLZ-200), [BLZ-201](https://linear.app/outfitter/issue/BLZ-201), [BLZ-202](https://linear.app/outfitter/issue/BLZ-202)): Comprehensive updates for new `blz get` capabilities
  - **Agent prompts**: Examples showing `alias:lines` → `blz get` workflows with jq helpers for parsing `ranges[]`
  - **Shell integration**: Updated all examples (PowerShell, Elvish, Fish, Bash, Zsh, Alfred, Raycast) to use colon syntax
  - **CLI reference**: Documented colon syntax (`bun:120-142`) as preferred over legacy `--lines` flag
  - **Syntax standardization**: All docs now use short format flags (`--json`, `--text`) instead of verbose `--format json/text`
  - **Cross-source patterns**: Examples demonstrating how to fetch and compare spans from multiple libraries
- **Prompt consolidation**: Grep-style context flags (`-C`, `-A`, `-B`) consolidated in agent prompts for improved token efficiency
  - Removed `--block` references from prompts (still works as legacy alias for `--context all`)
- **History limit flag**: Documented the default history retention behavior added in 1.1

## [1.1.1] - 2025-10-13

### Fixed
- **Search shorthand context flags**: Inline `blz "<query>"` invocations now honor context-related flags like `--context`, `-C`, `-A`, and `-B`, including attached short-flag values (e.g., `-C5`), ensuring the preprocessor no longer misparses them.

### Documentation
- **README entry points**: Added a dedicated Docs section near the top of the README to surface the bundled documentation hub, quickstart, agent playbook, and architecture overview.

### Tests
- **Info metadata failures**: New regression test covers the error path when `blz info` encounters invalid `llms.json` metadata, verifying the user-facing diagnostics remain descriptive.

## [1.1.0] - 2025-10-11

### Added
- **Fuzzy-matched source warnings**: When searching with a non-existent source filter, `blz` now suggests similar source names
  - Shows top 3 closest matches sorted by similarity score
  - Warnings print to stderr only (preserves JSON output on stdout)
  - Respects quiet mode (`-q` flag) to suppress warnings
  - Exit code remains 0 for backward compatibility
- **Bundled documentation hub**: New `blz docs` command with subcommands for embedded documentation
  - `blz docs search`: Search the bundled blz-docs source without touching other aliases
  - `blz docs sync`: Sync or resync embedded documentation files and index
  - `blz docs overview`: Quick-start guide for humans and agents
  - `blz docs cat`: Print entire bundled llms-full.txt to stdout
  - `blz docs export`: Export CLI docs in markdown or JSON (replaces old `blz docs --format`)
- **Internal documentation source**: `blz-docs` alias (also `@blz`) ships with the binary
  - Hidden from default search with `internal` tag
  - Auto-syncs on first use or when version changes
  - Full CLI reference and user guide embedded in the binary
- **Linear integration rules**: Added `.agents/rules/LINEAR.md` for Linear project management workflow
- **Configurable snippet length** ([BLZ-117](https://linear.app/outfitter/issue/BLZ-117)): New `--max-chars` flag controls snippet length
  - Default: 200 characters (increased from ~100)
  - Range: 50-1000 characters with automatic clamping
  - Environment variable: `BLZ_MAX_CHARS`
  - Counts total characters including newlines, not per-line column width
- **Backward pagination** ([BLZ-137](https://linear.app/outfitter/issue/BLZ-137)): New `--previous` flag complements `--next` for pagination
  - Navigate backward through search results without repeating queries
  - Stateful pagination: `--next` (forward), `--previous` (backward), `--last` (jump to end)
  - Error handling: "Already on first page" when at page 1
  - Maintains query and source context automatically
- **Grep-style context flags** ([BLZ-132](https://linear.app/outfitter/issue/BLZ-132)): Industry-standard short options for context
  - `-C <N>`: Print N lines of context (both before and after)
  - `-A <N>`: Print N lines after each match
  - `-B <N>`: Print N lines before each match
  - Flags can be combined (e.g., `-C5 -A2` merges to max values)
  - Legacy `-c` flag maintained for backward compatibility
- **Read-only command enhancements and format shortcuts** ([BLZ-123](https://linear.app/outfitter/issue/BLZ-123)): Consistent, ergonomic output controls across commands
  - Format aliases: `--json`, `--jsonl`, `--text`, and `--raw` map to their respective `--format` values
  - `--limit` flag added to `list`, `stats`, `lookup`, and `anchor list`
  - All read-only commands now support the new format shortcuts
  - JSON output is pure (no mixed stderr/stdout) for clean piping
- **Language filtering** ([BLZ-111](https://linear.app/outfitter/issue/BLZ-111)): Automatic filtering of non-English documentation
  - URL-based locale detection (path markers: `/de/`, `/ja/`, subdomain patterns)
  - 60-90% bandwidth and storage reduction for multilingual sources
  - Opt-out with `--no-language-filter` flag
  - Zero dependencies, <1μs per URL performance
- **Section expansion improvements** ([BLZ-115](https://linear.app/outfitter/issue/BLZ-115)): `--context all` now consistent
  - Single line queries now expand to full heading blocks (previously only ranges worked)
  - Behavior matches search command expectations
  - Legacy `--block` flag maintained as alias
- **Prompt enhancements** ([BLZ-116](https://linear.app/outfitter/issue/BLZ-116)): New "Try this" section in search prompt
  - 5 practical examples with explanations
  - Emphasizes one-shot retrieval workflow with `--context all`
  - Shows optimal snippet sizing, pagination navigation, and noise reduction techniques

### Changed
- `blz docs` command now uses subcommands instead of single `--format` flag
  - Old `blz docs --format json` still works for backward compatibility
  - New preferred syntax: `blz docs export --format json`
- **Short flag consistency** ([BLZ-113](https://linear.app/outfitter/issue/BLZ-113)): Audited and fixed across all commands
  - `-s` for `--source` works universally where defined
  - `-f` for `--format` available on all commands
  - `-C/-c` for `--context` (uppercase is new standard, lowercase maintained for compatibility)
  - `-l` for `--lines` on get command
  - `-n` for `--limit` on commands with pagination
  - Help text consistently shows all available short flags

### Deprecated
- **`--snippet-lines` flag** ([BLZ-133](https://linear.app/outfitter/issue/BLZ-133)): Use `--max-chars` instead
  - Hidden from help output
  - Still functional for backward compatibility
  - Will be removed in future major version
  - `BLZ_SNIPPET_LINES` environment variable also deprecated

### Fixed
- **Context flag parsing**: `-C`, `-A`, and `-B` now parse correctly with concatenated values (e.g., `-C5`)
- **Single-line block expansion**: `blz get <source>:<line> --context all` now expands to full section

### Internal
- Added `DocsCommands` enum for `blz docs` subcommands
- Added `DocsSearchArgs` for bundled docs search functionality
- New `docs_bundle.rs` module for managing embedded documentation
- Added `ContextMode` enum with `All`, `Symmetric`, and `Asymmetric` variants
- Added `merge_context_flags` function for grep-style flag merging
- Comprehensive test suites for pagination (`--next`, `--previous`), context flags, and format shortcuts

## [1.0.0-beta.1] - 2025-10-03

### Breaking Changes
- Removed dual-flavor system (llms.txt vs llms-full.txt). BLZ now intelligently auto-prefers llms-full.txt when available.
- Removed backwards compatibility for v0.4.x cache format. Use `blz clear --force` to migrate from older versions.

### Added
- **One-line installation**: New install script with SHA-256 verification and platform detection
  - Download via: `curl -fsSL https://blz.run/install.sh | sh`
  - Support for macOS (x64, arm64) and Linux (x64)
  - SHA-256 checksum verification (use `--skip-check` to bypass)
  - Custom install directory with `--dir` flag
  - `--dry-run` mode for testing
- **Clipboard support**: Copy search results directly with `--copy` flag (OSC 52 escape sequences)
- **Search history**: New `blz history` command to view and manage persistent search history
  - History filtering by date, source, and query
  - Configurable retention (default: 1000 entries)
  - Clean command with date-based pruning
- **Source insights**: New commands for better visibility
  - `blz stats`: Cache statistics including source count, storage size, and index metrics
  - `blz info <source>`: Detailed source information with metadata
  - `blz validate`: Verify source integrity with URL accessibility, checksum validation, and staleness detection
  - `blz doctor`: Comprehensive health checks with auto-fix capability for cache and sources
- **Batch operations**: Add multiple sources via TOML manifest files
  - Template at `registry/templates/batch-manifest.example.toml`
  - Supports aliases, tags, npm/github mappings
  - Parallel indexing for faster setup
- **Rich metadata**: Source descriptors with name, description, and category
  - `blz list --details`: View extended source information
  - Auto-populated from registry or customizable
  - Persisted in `.blz/descriptor.toml` per source
- **Enhanced search**:
  - Multi-source filtering with `--source` flag (comma-separated)
  - Improved snippet extraction with configurable context lines
  - Search history integration with `.blz_history` replay

### Changed
- **URL intelligence**: Automatically prefers llms-full.txt when available (no manual configuration needed)
- **Simplified CLI**: Removed confusing `--flavor` flags from all commands
- **Better defaults**: Intelligent fallback to llms.txt if llms-full.txt unavailable
- **Descriptor defaults**: Sources added without explicit metadata get sensible auto-generated values

### Fixed
- **Exit codes**: Commands now properly return exit code 1 on errors for better scripting support
  - `blz get` with non-existent source now exits with code 1
  - `blz remove` with non-existent source now exits with code 1
  - `blz get` with out-of-range lines now exits with code 1 and provides helpful error message
- 40+ code quality improvements from strict clippy enforcement
- Redundant clones and inefficient Option handling eliminated
- Float precision warnings properly annotated
- All `.unwrap()` usage replaced with proper error handling
- Format string optimizations throughout CLI
- Documentation URL formatting fixed

### Performance
- Optimized format! string usage in hot paths
- Reduced unnecessary allocations in search results formatting
- Improved clipboard copy performance with write! macro

### Developer Experience
- All tests passing (224/224)
- Zero clippy warnings with strict configuration
- Clean release builds (~42s)
- Comprehensive v1.0-beta release checklist

## [0.5.0] - 2025-10-02

### Breaking Changes
- Removed backwards compatibility for v0.4.x cache format. Users upgrading from v0.4.x will need to clear their cache with `blz clear --force` and re-add sources. The CLI will detect old cache format and display helpful error message with migration instructions.

### Added
- New `blz clear` command to remove all cached sources and indices.
  - `--force` flag to skip confirmation prompt for non-interactive use.
  - Helpful error detection when old v0.4.x cache format is found.
- New `upgrade` command to migrate sources from llms.txt to llms-full.txt (#234).
- Automatic preference for llms-full.txt when available via `FORCE_PREFER_FULL` feature flag (#234).
- Comprehensive test suite for automatic llms-full preference behavior (5 new tests) (#234).
- CLI refactoring with testable seams for `clear`, `list`, `remove`, and `update` commands.

### Changed
- **XDG-compliant paths**: Both config and data now respect XDG Base Directory specification:
  - Config: `$XDG_CONFIG_HOME/blz/` (if set) or `~/.blz/` (fallback)
  - Data: `$XDG_DATA_HOME/blz/` (if set) or `~/.blz/` (fallback)
  - Environment overrides: `BLZ_GLOBAL_CONFIG_DIR` and `BLZ_DATA_DIR`
- **Reorganized data directory**: Source directories now organized under `sources/` subdirectory for cleaner structure.
- **Renamed state file**: `blz.json` renamed to `data.json` to distinguish runtime state from configuration files.
- Simplified flavor selection to automatically prefer llms-full.txt without user configuration (#234).
- Hidden `--flavor` flags across add, search, and update commands for cleaner user experience (#234).
- Updated `--yes` flag help text to be flavor-agnostic: "Skip confirmation prompts (non-interactive mode)" (#234).
- Removed `BLZ_PREFER_LLMS_FULL` environment variable (automatic preference replaces manual configuration) (#234).
- Removed custom LlmsJson deserializer for v0.4.x format (141 lines removed).

### Fixed
- Restored metadata alias propagation for update and add flows.
- Addressed security and portability issues identified in code review.
- Normalized heading counts with accurate recursive counting.
- Parser now filters out placeholder "404" pages.

### Documentation
- Updated 11 documentation files to reflect flavor simplification and automatic llms-full preference (#234).
- Added comprehensive `docs/cli/commands.md#upgrade` documentation (#234).
- Fixed 5 broken internal links in documentation index (#234).
- Added `SCRATCHPAD.md` for tracking session work and progress.

## [0.4.1] - 2025-09-29

### Added
- Search CLI pagination with history-aware `--next`/`--last`, improved JSON metadata, and stricter batch `get` span handling (#229).

### Changed
- JSON output now always includes both rounded `score` and `scorePercentage`, plus compatibility fields mirrored for downstream tooling (#229).
- Pagination flow now treats `--limit` as optional, enforces consistent page size when continuing with `--next`, and surfaces friendlier tips for text output (#229).
- Release automation can be manually dispatched without a full publish run (#228).

### Fixed
- Search history writes use fsync + atomic rename with advisory locking to avoid corruption when multiple CLI processes exit simultaneously (#229).

## [0.4.0] - 2025-09-26

### Changed
- Unified flavor resolution across `list`, `search`, and `get` so CLI commands respect stored preferences consistently (#227).
- Relaxed release coverage requirements to streamline the automated publish pipeline (#226).

## [0.3.3] - 2025-09-25

### Added
- Enhanced phrase search ergonomics, including `--source` flag migration, better highlighting, and improved snippet ordering (#224).

### Fixed
- Snippet extraction now handles quoted phrases without truncation (#225).

### CI
- Hardened the coverage cache cleanup to prevent flaky report uploads (#223).

## [0.3.2] - 2025-09-24

### Added
- SHA256 parameter support for the Homebrew workflow and expanded release automation documentation (#213, #217).

### Changed
- CLI shorthand parsing now dynamically discovers known subcommands and respects hidden entries (#215).
- Release workflows consolidated with parameterized modes and rewritten semver tooling in Rust for deterministic versioning (#218, #221).

### Fixed
- DotSlash generation and Homebrew publishing now retry transient errors to stabilize CI (#214, #212).

## [0.3.1] - 2025-09-24

### Added
- Linux binaries are now published alongside macOS and Windows in the Homebrew formula (#204).

### Fixed
- Search shorthand parsing correctly handles flags and hidden subcommands without misrouting queries (#203).

## [0.3.0] - 2025-09-21

### Added
- Dual-flavor ingestion for both `llms.txt` and `llms-full.txt`, including automatic
  detection, interactive selection, and flavor-aware indexing.
- CLI enhancements for the v0.3 release (refined help output, quiet mode polish,
  and centralized format flag handling).
- Release automation updates with coverage notes and BLZ stylization guidance for
  agent integrations.

### Changed
- Workspace crates bumped to version 0.3.0 to align with the release artifacts.
- Tests and documentation refreshed for the v0.3 feature set, including expanded
  integration coverage.

## [0.2.4] - 2025-09-18

### Fixed
- Added raw platform-specific binaries to GitHub release assets so npm postinstall can download them directly (was failing with 404s on v0.2.1).

### Changed
- Publish workflow now extracts archives while flattening artifacts to upload both compressed bundles and uncompressed binaries.

## [0.2.2] - 2025-09-17

### Changed
- Bumped workspace and npm packages to version 0.2.2 in preparation for the next patch release train.

### Fixed
- Hardened the publish workflow’s artifact flatten step by downloading into per-target directories, deep-searching for archives, and safely replacing existing files when identical assets already exist.

## [0.2.1] - 2025-09-17

### Changed
- Automated releases via label-driven workflows that build cross-platform artifacts, upload them, and publish npm/crates/Homebrew in sequence.
- Added asset readiness guards for the Homebrew job and tightened release undraft conditions to avoid incomplete releases.
- Cached `cargo-edit` in CI and documented local `act` rehearsals for release workflows.

### Fixed
- Windows npm postinstall now imports `package.json` via URL (no `ERR_UNSUPPORTED_ESM_URL_SCHEME`) and the package requires Node ≥ 18.20.0.

## [0.2.0] - 2025-09-15

### Added
- **`blz diff` command**: Compare current and archived versions of sources to see what's changed
- **`blz alias` command**: Manage source aliases with `add` and `rm` subcommands for better organization
- **`blz docs` command**: Generate CLI documentation in markdown or JSON format
- **Targeted cache invalidation**: Optimized search cache that invalidates only affected aliases on updates
- **Anchors support**: Parse and index anchor links from llms.txt files for better navigation
- **HEAD preflight checks**: Verify remote availability and size before downloads with retry logic
- **Windowed segmentation fallback**: Handle large documents that exceed indexing limits gracefully
- **Dynamic shell completions**: Enhanced completion support with metadata-aware suggestions
- **Flavor policy for updates**: Control update behavior with `--flavor` (auto, full, txt, current)

### Changed
- **JSON output improvements**: Consistent camelCase field names, added sourceUrl and checksum fields
- **CLI improvements**: Added `-s` as short alias for `--source`, improved error messages
- **Documentation restructure**: Split CLI docs into organized sections under `docs/cli/`
- **Performance**: Optimized search with granular cache invalidation per alias

### Fixed
- **JSON stability**: Proper stderr/stdout separation for clean JSON output
- **Panic handling**: Graceful handling of broken pipe errors (SIGPIPE)
- **Large document handling**: Fallback to windowed segmentation for documents exceeding limits

### Developer Experience
- **`blz --prompt` flag**: Emit JSON guidance for agents (replaces the old `blz instruct` output)
- **Improved logging**: All logs go to stderr, keeping stdout clean for JSON/JSONL output
- **Better error messages**: More actionable error messages with suggestions

## [0.1.7] - 2025-09-12

### Changed
- Bump workspace and npm versions to 0.1.7 for the next release train.

### CI
- Track Cargo.lock in release workflow and restore `--locked` enforcement.
- Finalize GitHub Release steps and tidy workflow titles.

## [0.1.6] - 2025-01-12

### Added
- Comprehensive CI/CD release workflows with GitHub Actions
- Support for automated releases to multiple platforms (macOS, Linux, Windows)
- Cargo.lock tracking for deterministic builds
- Draft release workflow with proper asset management
- Homebrew tap integration for macOS installations
- npm package publishing support
- Automated crates.io publishing with proper dependency ordering

### Fixed
- Security vulnerability RUSTSEC-2025-0055 in tracing-subscriber (updated to 0.3.20)
- CI/CD workflow robustness with proper error handling
- Draft release asset downloads using authenticated GitHub CLI
- Build reproducibility with --locked flag enforcement

### Changed
- Improved CI/CD workflows with reusable components
- Enhanced cache key strategy including Cargo.lock hash
- Standardized error message formats across workflows
- Better handling of annotated vs lightweight tags

### Security
- Updated tracing-subscriber from 0.3.19 to 0.3.20 to address log poisoning vulnerability

## [0.1.5] - 2025-01-05

### Added
- Initial public release of BLZ
- Fast local search for llms.txt documentation
- Support for multiple documentation sources
- Line-accurate search results with BM25 ranking
- ETag-based conditional fetching for efficiency
- Local filesystem storage with archive support

[Unreleased]: https://github.com/outfitter-dev/blz/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/outfitter-dev/blz/releases/tag/v1.4.0
[1.3.0]: https://github.com/outfitter-dev/blz/releases/tag/v1.3.0
[1.2.0]: https://github.com/outfitter-dev/blz/releases/tag/v1.2.0
[1.1.1]: https://github.com/outfitter-dev/blz/releases/tag/v1.1.1
[1.1.0]: https://github.com/outfitter-dev/blz/releases/tag/v1.1.0
[1.0.0-beta.1]: https://github.com/outfitter-dev/blz/releases/tag/v1.0.0-beta.1
[0.5.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.5.0
[0.4.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.4.1
[0.4.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.4.0
[0.3.3]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.3
[0.3.2]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.2
[0.3.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.1
[0.3.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.0
[0.2.4]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.4
[0.2.2]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.2
[0.2.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.1
[0.2.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.0
[0.1.7]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.7
[0.1.6]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.6
[0.1.5]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.5
