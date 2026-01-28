# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Release-please will manage this file after the migration lands. Archived
pre-cutover notes live in `docs/release/next-release-notes.md`.

## [3.0.0-beta.1](https://github.com/outfitter-dev/blz/compare/v2.0.0-beta.1...v3.0.0-beta.1) (2026-01-28)


### ⚠ BREAKING CHANGES

* **cli:** MCP server command renamed from `blz mcp` to `blz mcp-server`

### Features

* **#33:** Implement update command with ETag/Last-Modified and archive ([#43](https://github.com/outfitter-dev/blz/issues/43)) ([6dc695e](https://github.com/outfitter-dev/blz/commit/6dc695e45d8c0b22ede232ae7a46d94c9cfd0ecd)), closes [#33](https://github.com/outfitter-dev/blz/issues/33)
* add Claude AI workflows and security tooling ([38fbb9e](https://github.com/outfitter-dev/blz/commit/38fbb9e5432ef7c0af02130d12566b3d067dc055))
* add Fish shell support with dynamic completions ([1c6474a](https://github.com/outfitter-dev/blz/commit/1c6474a582328dcb20bec389fc9abcd40af8c049))
* add missing doc sources to registry (BLZ-112) ([#239](https://github.com/outfitter-dev/blz/issues/239)) ([83355a6](https://github.com/outfitter-dev/blz/commit/83355a664cfe77a9e5567dab691d2086a8f43fcd))
* add registry lookup command for discovering documentation sources ([af17fb6](https://github.com/outfitter-dev/blz/commit/af17fb664bc27d2396d5df03f502a47acfbe7fd7))
* add smart llms.txt flavor detection with interactive selection ([80c4c6f](https://github.com/outfitter-dev/blz/commit/80c4c6f4db7c0ee46d099bdf36afe4e9676e3c9f))
* blz v0.1.0 - first release with core search, CLI, and full CI/CD pipeline ([#129](https://github.com/outfitter-dev/blz/issues/129)) ([415d017](https://github.com/outfitter-dev/blz/commit/415d01755a99e167ebf6250ec4bab0c08af3fd69))
* **ci:** add comprehensive CI workflow with nextest configuration ([1aa5245](https://github.com/outfitter-dev/blz/commit/1aa5245439418bbf69f9d9d53c69241fbf78e69d))
* **ci:** add GitHub Actions workflow for Rust CI ([a7ed323](https://github.com/outfitter-dev/blz/commit/a7ed3231562a8b4a16e47cbb096ab43ae620cd79))
* **ci:** add Linux support to Homebrew formula ([#204](https://github.com/outfitter-dev/blz/issues/204)) ([5d3bac3](https://github.com/outfitter-dev/blz/commit/5d3bac328791aaf650eff3172244f8839df91842))
* **ci:** add SHA256 input parameters to Homebrew workflow ([#213](https://github.com/outfitter-dev/blz/issues/213)) ([2ead596](https://github.com/outfitter-dev/blz/commit/2ead596f31f230b217be9da175a83c71fb916fbf))
* **ci:** automate release pipeline ([#176](https://github.com/outfitter-dev/blz/issues/176)) ([f028aba](https://github.com/outfitter-dev/blz/commit/f028aba1c5393c60836654211a332401b65d6c7d))
* **claude:** add implement-issue command ([#242](https://github.com/outfitter-dev/blz/issues/242)) ([2a38de5](https://github.com/outfitter-dev/blz/commit/2a38de5eda82e2f82ab9bc7bffb89f04ad45c000))
* **cli:** add --context all as primary interface for section expansion [BLZ-116] ([#244](https://github.com/outfitter-dev/blz/issues/244)) ([5a82f03](https://github.com/outfitter-dev/blz/commit/5a82f0314561dcb2c95c921c1fbef3a4772f1309))
* **cli:** add --reindex flag to refresh command [BLZ-265] ([#329](https://github.com/outfitter-dev/blz/issues/329)) ([2739ce7](https://github.com/outfitter-dev/blz/commit/2739ce75580b6676fee64d12a7315d0bd9ea0064))
* **cli:** add --timing flag for performance profiling ([#489](https://github.com/outfitter-dev/blz/issues/489)) ([ceaaede](https://github.com/outfitter-dev/blz/commit/ceaaede5782d2044cc749e4b89df18ea95ba88ff))
* **cli:** add backpressure-aware streaming output ([#488](https://github.com/outfitter-dev/blz/issues/488)) ([529a082](https://github.com/outfitter-dev/blz/commit/529a082cc5b3665e33a0ac7279a8afad39debcc6))
* **cli:** add boolean filtering to toc ([#320](https://github.com/outfitter-dev/blz/issues/320)) ([a48c2cb](https://github.com/outfitter-dev/blz/commit/a48c2cb7a62d26c53e151d13a535a00ae6f2da0a))
* **cli:** add claude-plugin install helper ([#459](https://github.com/outfitter-dev/blz/issues/459)) ([0802008](https://github.com/outfitter-dev/blz/commit/0802008af01340ded57fbf58c8677d27687f3f92))
* **cli:** add grep-style context flags (-A, -B, -C) for search and get commands ([#255](https://github.com/outfitter-dev/blz/issues/255)) ([a573f78](https://github.com/outfitter-dev/blz/commit/a573f78185cbb1adb490c0214060833f6691edea))
* **cli:** add grouped help sections for better command discoverability ([#482](https://github.com/outfitter-dev/blz/issues/482)) ([6fbfc97](https://github.com/outfitter-dev/blz/commit/6fbfc97abf3fa577e7f3e9118f62d6fa3d449eb7))
* **cli:** add language filtering for non-English content ([#249](https://github.com/outfitter-dev/blz/issues/249)) ([d8cabdf](https://github.com/outfitter-dev/blz/commit/d8cabdf119b36d890815f9c754e6589472cc1130))
* **cli:** add limit option to list, lookup, stats and anchors commands ([#253](https://github.com/outfitter-dev/blz/issues/253)) ([fb997b0](https://github.com/outfitter-dev/blz/commit/fb997b095bd1183b84d97d0cc0fec2ea7f8f9aa4))
* **cli:** add mcp subcommand [BLZ-213] ([#291](https://github.com/outfitter-dev/blz/issues/291)) ([db9b058](https://github.com/outfitter-dev/blz/commit/db9b0586cbf3d619878ef53a5f9ea05fe496f35e))
* **cli:** add pagination state to toc command [BLZ-249] ([#325](https://github.com/outfitter-dev/blz/issues/325)) ([437b93b](https://github.com/outfitter-dev/blz/commit/437b93bb36462593fbf0a7e7b75d577744096c3f))
* **cli:** add pagination state to toc command [BLZ-250] ([#324](https://github.com/outfitter-dev/blz/issues/324)) ([f357e6f](https://github.com/outfitter-dev/blz/commit/f357e6f548beafc5a66d6e1997dcd9bc8638ffef))
* **cli:** add search pagination, score display options, and batch get support ([#229](https://github.com/outfitter-dev/blz/issues/229)) ([b101252](https://github.com/outfitter-dev/blz/commit/b1012529ec405d0c92d2d32bfff36b04b4bae00e))
* **cli:** add semantic error categories with exit codes ([#476](https://github.com/outfitter-dev/blz/issues/476)) ([630b801](https://github.com/outfitter-dev/blz/commit/630b801be79fbeb95984b99d13ef562fa02288e9))
* **cli:** add shape-based output types ([#474](https://github.com/outfitter-dev/blz/issues/474)) ([e0aa22f](https://github.com/outfitter-dev/blz/commit/e0aa22ff5247c53d60714e14e4c318f2c16ab825))
* **cli:** add shared argument groups for CLI commands ([#473](https://github.com/outfitter-dev/blz/issues/473)) ([b5222ab](https://github.com/outfitter-dev/blz/commit/b5222abf77a853f04fbd32f3261a47e814760d70))
* **cli:** add toc limit and depth controls ([#318](https://github.com/outfitter-dev/blz/issues/318)) ([04b4729](https://github.com/outfitter-dev/blz/commit/04b47290344004abf435529afbab76b99961de85))
* **cli:** add TTY detection utilities and unify OutputFormat ([#475](https://github.com/outfitter-dev/blz/issues/475)) ([1adc4f6](https://github.com/outfitter-dev/blz/commit/1adc4f6d9f9805412a00c54dd310c63521c2aecb))
* **cli:** add unified find command with pattern-based dispatch ([#339](https://github.com/outfitter-dev/blz/issues/339)) ([bcb94c6](https://github.com/outfitter-dev/blz/commit/bcb94c689a7151bb363df598830ce7eab0d08dab))
* **cli:** deprecate --snippet-lines flag in favor of --max-chars ([#254](https://github.com/outfitter-dev/blz/issues/254)) ([20f72c5](https://github.com/outfitter-dev/blz/commit/20f72c5440835d4523adfc06ed99e8d098219449))
* **cli:** enhance toc with heading level operators and tree view [BLZ-256] ([#323](https://github.com/outfitter-dev/blz/issues/323)) ([207245e](https://github.com/outfitter-dev/blz/commit/207245e70db60d11181e38e3da8742910086b53a))
* **cli:** implement core v0.3 functionality and commands ([#199](https://github.com/outfitter-dev/blz/issues/199)) ([e9faf4b](https://github.com/outfitter-dev/blz/commit/e9faf4b980cef7f6d44befda88939195a7d1bce3))
* **cli:** implement multi-source get JSON contract [BLZ-199] ([#277](https://github.com/outfitter-dev/blz/issues/277)) ([81b5b3b](https://github.com/outfitter-dev/blz/commit/81b5b3b873be176ef67966d04fe27470b3b3f751))
* **cli:** make --filter flag extensible for future filters [BLZ-266] ([#330](https://github.com/outfitter-dev/blz/issues/330)) ([6c77420](https://github.com/outfitter-dev/blz/commit/6c7742008950c93436e99168f4dcb2ba7c6ca093))
* **cli:** rename anchors command to toc ([#317](https://github.com/outfitter-dev/blz/issues/317)) ([b814b45](https://github.com/outfitter-dev/blz/commit/b814b459aeee41ffe5175bde28c64712b577544d))
* **cli:** scaffold get JSON contract [BLZ-163] ([#276](https://github.com/outfitter-dev/blz/issues/276)) ([e4c3e18](https://github.com/outfitter-dev/blz/commit/e4c3e18c31722f218def9a1f20fd0464ba5edb79))
* **cli:** show filter status and reason in info command [BLZ-267] ([#331](https://github.com/outfitter-dev/blz/issues/331)) ([55218af](https://github.com/outfitter-dev/blz/commit/55218af5ed6e48bc764f9422101a63bccd878adb))
* **cli:** standardize short flags and migrate to FormatArg (BLZ-113) ([#251](https://github.com/outfitter-dev/blz/issues/251)) ([79ccd41](https://github.com/outfitter-dev/blz/commit/79ccd41aa51779f3e5155c98c1aa9a0364b23ad5))
* **cli:** support boolean toc filters ([#343](https://github.com/outfitter-dev/blz/issues/343)) ([3beb640](https://github.com/outfitter-dev/blz/commit/3beb640b34ebda76fd09ab0ff57099b8c2ecc2b9))
* comprehensive CLI ergonomics improvements ([e9c20fb](https://github.com/outfitter-dev/blz/commit/e9c20fb1efd81736570f3471f0f5f504698aaa31))
* comprehensive overhaul - rename to blz, add registry lookup, and feature flags ([#6](https://github.com/outfitter-dev/blz/issues/6)) ([82f4124](https://github.com/outfitter-dev/blz/commit/82f4124af65554663984346bd76f9887ea0a969a))
* **core:** add HeadingLevel and Verbosity types ([#472](https://github.com/outfitter-dev/blz/issues/472)) ([4067e5c](https://github.com/outfitter-dev/blz/commit/4067e5cb92fb59aab5fecdf6d2c15fb4cd583fe0))
* **core:** normalize heading display and search aliases [BLZ-243] ([#314](https://github.com/outfitter-dev/blz/issues/314)) ([7980d4a](https://github.com/outfitter-dev/blz/commit/7980d4ad36152cc2fd4f4e902e7fb32fb056d130))
* **core:** persist language filter preference per-source [BLZ-263] ([#327](https://github.com/outfitter-dev/blz/issues/327)) ([ca12f56](https://github.com/outfitter-dev/blz/commit/ca12f56a477795e8fa6415ee52ef3bb73453a87c))
* **docs:** add bundled documentation hub and Linear integration rules ([#250](https://github.com/outfitter-dev/blz/issues/250)) ([9fc111d](https://github.com/outfitter-dev/blz/commit/9fc111dea998f6427c47634f28ab63a02ced8275))
* finalize rename to Blaze (blz) ([db6411e](https://github.com/outfitter-dev/blz/commit/db6411ed960852a8e2f55276b8a2aee2a5954af0))
* implement MVP for @outfitter/cache ([474d1a2](https://github.com/outfitter-dev/blz/commit/474d1a255effb2086ace8f2fbfbf0b076d6c88bc))
* initial README for @outfitter/cache ([13385f1](https://github.com/outfitter-dev/blz/commit/13385f1f890b9600a8053630be3493460df8e75d))
* **mcp:** establish blz-mcp foundation with tests and clean CI [BLZ-207] ([#284](https://github.com/outfitter-dev/blz/issues/284)) ([d0c8b46](https://github.com/outfitter-dev/blz/commit/d0c8b46b509a66aa86c1a8903bca2143feef46b4))
* **mcp:** expose resources for sources and registry [BLZ-211] ([#288](https://github.com/outfitter-dev/blz/issues/288)) ([5da65da](https://github.com/outfitter-dev/blz/commit/5da65da38dacf51336a35d58d41d0f469065971d))
* **mcp:** implement auxiliary tools (run-command, learn-blz) [BLZ-210] ([#287](https://github.com/outfitter-dev/blz/issues/287)) ([bed667f](https://github.com/outfitter-dev/blz/commit/bed667f86dd20333c584a0d121fbf0b0346795f5))
* **mcp:** implement discover-docs prompt [BLZ-212] ([#290](https://github.com/outfitter-dev/blz/issues/290)) ([56406b7](https://github.com/outfitter-dev/blz/commit/56406b7cb607696e81ccb1dd014aae2718e5ddad))
* **mcp:** implement find tool for search and snippet retrieval [BLZ-208] ([#285](https://github.com/outfitter-dev/blz/issues/285)) ([5b14c36](https://github.com/outfitter-dev/blz/commit/5b14c36b67decf2b73412030988cbe9917992752))
* **mcp:** implement list-sources and source-add tools [BLZ-209] ([#286](https://github.com/outfitter-dev/blz/issues/286)) ([022d405](https://github.com/outfitter-dev/blz/commit/022d405105da416724450b4493f17451ef8cd470))
* **mcp:** optimize tool names and add response format ([#295](https://github.com/outfitter-dev/blz/issues/295)) ([bea5f2a](https://github.com/outfitter-dev/blz/commit/bea5f2ac6d3f826c4bf80c86265e90f003e50fe9))
* **mcp:** simplify claude plugin to single command and agent ([#338](https://github.com/outfitter-dev/blz/issues/338)) ([8ebd32b](https://github.com/outfitter-dev/blz/commit/8ebd32bf66fd779e12e187bbe69367f261399571))
* **mcp:** support cross-source search with "all" or array of sources ([#297](https://github.com/outfitter-dev/blz/issues/297)) ([383251d](https://github.com/outfitter-dev/blz/commit/383251d1a52d2608b298742acf19af2470d87b19))
* rename project to blzr (Blazer) ([ecddcc2](https://github.com/outfitter-dev/blz/commit/ecddcc2eb260b44c45b9307d0546cbae07218867))
* **search:** add --max-chars option to control snippet length ([#252](https://github.com/outfitter-dev/blz/issues/252)) ([e3eef32](https://github.com/outfitter-dev/blz/commit/e3eef32a53bf126b38c67d78faf3e670ee542350))
* **search:** add --previous flag for navigating to previous page ([#256](https://github.com/outfitter-dev/blz/issues/256)) ([84fcdfe](https://github.com/outfitter-dev/blz/commit/84fcdfe63794f3c2fab70a0825b3055bb8a256af))
* **search:** add fuzzy-matched source warnings for non-existent sources [BLZ-154] ([#259](https://github.com/outfitter-dev/blz/issues/259)) ([ee70910](https://github.com/outfitter-dev/blz/commit/ee70910c60fbba41219bb7b7b15af0bb356a2f20))
* **search:** add headings-only flag [BLZ-228] ([#316](https://github.com/outfitter-dev/blz/issues/316)) ([c9fb032](https://github.com/outfitter-dev/blz/commit/c9fb0327e53649a6a91818ae1219086e006dd25e))
* **search:** boost heading matches with # prefix ([#315](https://github.com/outfitter-dev/blz/issues/315)) ([7f9152b](https://github.com/outfitter-dev/blz/commit/7f9152b012e49ff7cb3e9bc95301f11a79c4561d))
* **search:** improve phrase search ergonomics and migrate to --source flag ([#224](https://github.com/outfitter-dev/blz/issues/224)) ([b367c8b](https://github.com/outfitter-dev/blz/commit/b367c8bf3509e948100380a12629642eedd04fe0))
* **storage:** add dual-flavor support for llms.txt and llms-full.txt ([#198](https://github.com/outfitter-dev/blz/issues/198)) ([65c50e0](https://github.com/outfitter-dev/blz/commit/65c50e0c6974b359f0fddf9ce8e2e6aec5f37efa))
* **v0.2:** targeted cache invalidation, diff command, flavor policy, and enhanced DX ([#164](https://github.com/outfitter-dev/blz/issues/164)) ([c049962](https://github.com/outfitter-dev/blz/commit/c049962427d95b60e61b0de1bfb0047816f225d1))


### Bug Fixes

* **#34:** Reduce over-fetching and add parallel multi-source search ([#41](https://github.com/outfitter-dev/blz/issues/41)) ([2e254ca](https://github.com/outfitter-dev/blz/commit/2e254ca8fd3de17e9e759e440771d07783a919e4))
* **#36:** tighten lints, hide diff command, improve error messages ([#40](https://github.com/outfitter-dev/blz/issues/40)) ([45c34f8](https://github.com/outfitter-dev/blz/commit/45c34f80bd0f85e42c0f3bda2a3900a096f847e8)), closes [#36](https://github.com/outfitter-dev/blz/issues/36)
* **add:** improve llms.txt resolution guidance ([#387](https://github.com/outfitter-dev/blz/issues/387)) ([9c2a0ff](https://github.com/outfitter-dev/blz/commit/9c2a0ff60702544666cecb60c2f1fead04b52e37))
* address CodeRabbit review feedback for PR [#14](https://github.com/outfitter-dev/blz/issues/14) ([31a3caf](https://github.com/outfitter-dev/blz/commit/31a3caf746b184ceb7c50622ad0c303c539c2744))
* **build:** add ring workarounds for macOS ARM CPU feature detection issues ([#145](https://github.com/outfitter-dev/blz/issues/145)) ([656760d](https://github.com/outfitter-dev/blz/commit/656760dabfb405e578415aea50f63ed7b2b7fcce))
* change remaining 'blzr' reference to 'blz' in MCP server ([4ddf009](https://github.com/outfitter-dev/blz/commit/4ddf009e1471d8cb76c0b51173e33fffeee580b9))
* **ci:** add missing npm auth token for publish workflow ([#174](https://github.com/outfitter-dev/blz/issues/174)) ([6b674a9](https://github.com/outfitter-dev/blz/commit/6b674a95180af9a64ab599653e55b051a7de4831))
* **ci:** build linux on ubuntu-22.04 ([#417](https://github.com/outfitter-dev/blz/issues/417)) ([84ad65f](https://github.com/outfitter-dev/blz/commit/84ad65f1145667e1859a75dbbeafbd754c0c455a))
* **ci:** correct rust-toolchain.toml syntax error ([#115](https://github.com/outfitter-dev/blz/issues/115)) ([f2f3aae](https://github.com/outfitter-dev/blz/commit/f2f3aaee61945d1a6a737a505b7ba4bc50950f77))
* **ci:** harden release automation ([#179](https://github.com/outfitter-dev/blz/issues/179)) ([c0f8283](https://github.com/outfitter-dev/blz/commit/c0f828312eb72f558cc37d5536f3457153c06981))
* **ci:** improve DotSlash generation workflow reliability ([#214](https://github.com/outfitter-dev/blz/issues/214)) ([b39549c](https://github.com/outfitter-dev/blz/commit/b39549c3ed01cf9900dee50268f2f6bdcae2329d))
* **ci:** install NASM for Windows release builds ([#171](https://github.com/outfitter-dev/blz/issues/171)) ([5cd48cf](https://github.com/outfitter-dev/blz/commit/5cd48cf96d69247100e7820c43c623888c40c114))
* **ci:** remove --locked flag from release workflow ([83ff8a2](https://github.com/outfitter-dev/blz/commit/83ff8a2e2971523aea83894702471503deb274e6))
* **ci:** resolve clippy and release-please token ([#391](https://github.com/outfitter-dev/blz/issues/391)) ([fd79c53](https://github.com/outfitter-dev/blz/commit/fd79c53b67510eb0e00d4ce603f3455abb577e9c))
* **ci:** skip claude review for bot-created PRs ([#367](https://github.com/outfitter-dev/blz/issues/367)) ([57e942a](https://github.com/outfitter-dev/blz/commit/57e942a513997e12cd915f14f83d0cd26436817c))
* **ci:** use macos-14 for darwin builds ([#410](https://github.com/outfitter-dev/blz/issues/410)) ([196eb8b](https://github.com/outfitter-dev/blz/commit/196eb8b85d294616899d0c3e8950b9a138d480d8))
* **cli:** --context all now expands single lines to full blocks in get command [BLZ-115] ([#245](https://github.com/outfitter-dev/blz/issues/245)) ([9ccf0dd](https://github.com/outfitter-dev/blz/commit/9ccf0dd6cab218a124f70e09110f80cd8508dc88))
* **cli:** allow multiple inputs to find ([#369](https://github.com/outfitter-dev/blz/issues/369)) ([6569a57](https://github.com/outfitter-dev/blz/commit/6569a5743c77557a8abce003f680b326e756388e))
* **cli:** apply language filtering in refresh command [BLZ-264] ([#328](https://github.com/outfitter-dev/blz/issues/328)) ([eb474f0](https://github.com/outfitter-dev/blz/commit/eb474f0a814eb80b214a58a716114b3df6cbfb2e))
* **cli:** embed bundled docs for publishing ([#272](https://github.com/outfitter-dev/blz/issues/272)) ([3cffcc6](https://github.com/outfitter-dev/blz/commit/3cffcc6e8f245d263f405453e9471b6fad6a2b8a))
* **cli:** handle flags correctly in shorthand search syntax ([#203](https://github.com/outfitter-dev/blz/issues/203)) ([a75dc8c](https://github.com/outfitter-dev/blz/commit/a75dc8cc60a943817c3a36ee9113d40e3d3ed507))
* **cli:** improve bundled docs search error messaging and format handling [BLZ-151] ([#258](https://github.com/outfitter-dev/blz/issues/258)) ([e9e9ff1](https://github.com/outfitter-dev/blz/commit/e9e9ff1d09dba217c111e445f2a9cd69f4b8b45b))
* **cli:** improve shell completions and docs ([#388](https://github.com/outfitter-dev/blz/issues/388)) ([6efee25](https://github.com/outfitter-dev/blz/commit/6efee25ae0058c07e9e9782bc4d99f24cad4d035))
* **cli:** include headings count in info output [BLZ-152] ([#273](https://github.com/outfitter-dev/blz/issues/273)) ([80870dd](https://github.com/outfitter-dev/blz/commit/80870dd72bd9c51cc58829627f2521109246367e))
* **cli:** recognize context flags in shorthand search syntax ([#264](https://github.com/outfitter-dev/blz/issues/264)) ([08f4c60](https://github.com/outfitter-dev/blz/commit/08f4c60e360cee97fc0f9bd21e58d51ea3c4369e))
* **cli:** rename mcp command to mcp-server, allow mcp as source alias [BLZ-258] ([#333](https://github.com/outfitter-dev/blz/issues/333)) ([9997dfd](https://github.com/outfitter-dev/blz/commit/9997dfd03b587706c619178a4bcd121711380c31))
* **cli:** respect hidden subcommands in shorthand mode ([d227c5c](https://github.com/outfitter-dev/blz/commit/d227c5c9a32e014a3d4f0fd3404cba3d9a4ffe3d))
* **cli:** unify flavor resolution across list, search, and get commands ([#227](https://github.com/outfitter-dev/blz/issues/227)) ([1070207](https://github.com/outfitter-dev/blz/commit/1070207780f92f826ea17af9ddf4c29f9694c7b3))
* comprehensive code improvements for production quality ([358238d](https://github.com/outfitter-dev/blz/commit/358238db459a444bd2e01ea08b750ac3282b4de0))
* comprehensive code improvements for production quality ([#13](https://github.com/outfitter-dev/blz/issues/13)) ([f4e9e0c](https://github.com/outfitter-dev/blz/commit/f4e9e0c2b56ce2d30e74b053332cc4df7ebb7153))
* **core:** improve language filtering with hybrid url+text detection [BLZ-236] ([#302](https://github.com/outfitter-dev/blz/issues/302)) ([93ee123](https://github.com/outfitter-dev/blz/commit/93ee123e3b14d3cbb6a996a7d137b1464237c2a6))
* **core:** refactor url resolver error message ([#390](https://github.com/outfitter-dev/blz/issues/390)) ([dbe585f](https://github.com/outfitter-dev/blz/commit/dbe585f5d3563cff82e2b47fe4a1fedf68c043ab))
* correct heading-block extraction with exact line slices (Issue [#31](https://github.com/outfitter-dev/blz/issues/31)) ([#38](https://github.com/outfitter-dev/blz/issues/38)) ([e888785](https://github.com/outfitter-dev/blz/commit/e888785c6596202377de70e76ce885cc2c7419dc))
* **error-handling:** improve error handling and Unicode safety (Issue [#9](https://github.com/outfitter-dev/blz/issues/9)) ([#16](https://github.com/outfitter-dev/blz/issues/16)) ([8fab665](https://github.com/outfitter-dev/blz/commit/8fab6658a22ade068d507a798d7980c024db456b))
* **homebrew:** correct formula component order and livecheck URL ([e7fba3b](https://github.com/outfitter-dev/blz/commit/e7fba3b0bb31ccbc85ebc09027ead008d9972617))
* hook syntax error and skill MCP documentation ([#365](https://github.com/outfitter-dev/blz/issues/365)) ([4c888b6](https://github.com/outfitter-dev/blz/commit/4c888b6b86091730477b0f40317c332959f50088))
* **mcp:** add crates.io description ([#415](https://github.com/outfitter-dev/blz/issues/415)) ([f13422c](https://github.com/outfitter-dev/blz/commit/f13422ccc4d81578808e4f0a1aa7c03eefd5f3d3))
* **mcp:** default prompts capability fields ([#411](https://github.com/outfitter-dev/blz/issues/411)) ([7b8094c](https://github.com/outfitter-dev/blz/commit/7b8094c1ea170c70f47957088c85b8bb02e53989))
* **mcp:** expand context all to deepest section boundary ([#296](https://github.com/outfitter-dev/blz/issues/296)) ([e0a6b92](https://github.com/outfitter-dev/blz/commit/e0a6b92ef9c0cb8a6665b91870a3d5079359fa88))
* **net:** use native-tls on Windows ([#172](https://github.com/outfitter-dev/blz/issues/172)) ([b75437e](https://github.com/outfitter-dev/blz/commit/b75437ea4b0ebeb6bcb8509c3ab72b04e0b293af)), closes [#168](https://github.com/outfitter-dev/blz/issues/168)
* **npm:** update import syntax and decouple publish jobs ([#175](https://github.com/outfitter-dev/blz/issues/175)) ([b49105b](https://github.com/outfitter-dev/blz/commit/b49105ba91105a309f577917514891cba13c2ab4))
* **quality:** code quality improvements and maintenance (Issue [#11](https://github.com/outfitter-dev/blz/issues/11)) ([#15](https://github.com/outfitter-dev/blz/issues/15)) ([ceff689](https://github.com/outfitter-dev/blz/commit/ceff6891fa404373956e7b21f3a5e027739d5517))
* **release:** auto-label homebrew tap PRs ([#408](https://github.com/outfitter-dev/blz/issues/408)) ([e20abdf](https://github.com/outfitter-dev/blz/commit/e20abdf444fd83d931987ebeda25fa3adba00872))
* **release:** disable draft releases ([#403](https://github.com/outfitter-dev/blz/issues/403)) ([d9dc1a2](https://github.com/outfitter-dev/blz/commit/d9dc1a227a418f2d111a64e31e757ff5d6a70d2c))
* **release:** enable cargo-workspace plugin ([#399](https://github.com/outfitter-dev/blz/issues/399)) ([b3d4df8](https://github.com/outfitter-dev/blz/commit/b3d4df871beedc79522c418a6e9cbaa088f08775))
* **release:** gate homebrew linux arm fallback ([#267](https://github.com/outfitter-dev/blz/issues/267)) ([d73a894](https://github.com/outfitter-dev/blz/commit/d73a8949fcfd37d77fb7e31e7811b3460e31c6b5))
* **release:** rewrite semver tooling in Rust ([#221](https://github.com/outfitter-dev/blz/issues/221)) ([ee81e11](https://github.com/outfitter-dev/blz/commit/ee81e11df8fd53342ffe98f2ccdf7825d842e09f))
* **release:** switch release-please to node strategy ([#400](https://github.com/outfitter-dev/blz/issues/400)) ([4aa010a](https://github.com/outfitter-dev/blz/commit/4aa010a578b8a8519d1f88dada2e900c0c7f09c0))
* **release:** sync Cargo.lock on release PRs ([#406](https://github.com/outfitter-dev/blz/issues/406)) ([7cf94f0](https://github.com/outfitter-dev/blz/commit/7cf94f071ebf8bbf4a2a78a74ba061838640064b))
* **release:** wait on crates.io index + pass homebrew shas ([#412](https://github.com/outfitter-dev/blz/issues/412)) ([1f14b50](https://github.com/outfitter-dev/blz/commit/1f14b503abf13cf98297b702f326b3f8404bd0a5))
* remove unused pretty_assertions dependency and switch to rustls ([31209d4](https://github.com/outfitter-dev/blz/commit/31209d4cdbb4cb0d15625c2c25ca867c8bbcdc7f)), closes [#30](https://github.com/outfitter-dev/blz/issues/30)
* resolve all compilation warnings and add missing dependency ([ae5f4c1](https://github.com/outfitter-dev/blz/commit/ae5f4c138c09d62b7e908fdfe0f38606114c70e8))
* resolve clippy warnings for Rust 1.90.0 ([6a00326](https://github.com/outfitter-dev/blz/commit/6a0032677d563ccc257e454692ef32872b01e2e4))
* resolve final MCP server warnings ([4411c05](https://github.com/outfitter-dev/blz/commit/4411c0503505e08970189cd9dd4ac6f9ceacd257))
* **search:** improve snippet extraction for quoted phrases ([#225](https://github.com/outfitter-dev/blz/issues/225)) ([b407934](https://github.com/outfitter-dev/blz/commit/b4079347cee919db8fbdf01580c935c1652cc6ea))
* **tests:** improve test error handling with panic instead of assert(false) ([#370](https://github.com/outfitter-dev/blz/issues/370)) ([7361c61](https://github.com/outfitter-dev/blz/commit/7361c61bc2b675d5894c9ca78b018eae7f7237af))
* unblock v1.3 crate publishing ([#298](https://github.com/outfitter-dev/blz/issues/298)) ([fd64be7](https://github.com/outfitter-dev/blz/commit/fd64be70c85cc6647addf43405b0cf1e3f2ea884))


### Performance

* **dx:** optimize build and test performance [BLZ-237] ([#303](https://github.com/outfitter-dev/blz/issues/303)) ([e97dc67](https://github.com/outfitter-dev/blz/commit/e97dc67017dbebb74e060df0932b59a3662a5691))
* **dx:** Optimize git hooks for faster development workflow [BLZ-235] ([#301](https://github.com/outfitter-dev/blz/issues/301)) ([a9bf0f3](https://github.com/outfitter-dev/blz/commit/a9bf0f3d294934f52bbe0f090cd36485cef5ca9e))
* **hooks:** defer fetcher network tests to CI ([#382](https://github.com/outfitter-dev/blz/issues/382)) ([b1646a6](https://github.com/outfitter-dev/blz/commit/b1646a6b5a42b9b58c66a99b9a36de69b4d0d694))


### Refactoring

* **cli:** add ExecutionConfig for bundled execution parameters ([#478](https://github.com/outfitter-dev/blz/issues/478)) ([5c1bb54](https://github.com/outfitter-dev/blz/commit/5c1bb54c6d4f151722ca6b59793c36c310104801))
* **cli:** decompose add and lib functions ([#480](https://github.com/outfitter-dev/blz/issues/480)) ([95ce771](https://github.com/outfitter-dev/blz/commit/95ce771d51ec23502747cdf35f5fe64a095c85d4))
* **cli:** decompose search and get command functions ([#479](https://github.com/outfitter-dev/blz/issues/479)) ([b01eb3b](https://github.com/outfitter-dev/blz/commit/b01eb3b997684c2fdaf85ef8fb42b265540f58b1))
* **cli:** dynamically generate known subcommands list ([#215](https://github.com/outfitter-dev/blz/issues/215)) ([4307b87](https://github.com/outfitter-dev/blz/commit/4307b877b00b8cdedb27f8bd83cea7d9928709b7)), closes [#209](https://github.com/outfitter-dev/blz/issues/209)
* **cli:** migrate prompts from dialoguer to inquire [BLZ-240] ([#311](https://github.com/outfitter-dev/blz/issues/311)) ([6edc3f8](https://github.com/outfitter-dev/blz/commit/6edc3f81dd7f530fa4a404116dec3d5328e46072))
* **clippy-02-core-surgical-fixes:** registry API, index signature, storage error propagation ([7de4b77](https://github.com/outfitter-dev/blz/commit/7de4b7758d2578b821d9b50d58a702cacfb3addb))
* **cli:** rename --mappings to --anchors in toc command ([#322](https://github.com/outfitter-dev/blz/issues/322)) ([830e78b](https://github.com/outfitter-dev/blz/commit/830e78b305295fa9e2c570fe6bdd6b89e2a9508a))
* **cli:** rename update to refresh command [BLZ-262] ([#326](https://github.com/outfitter-dev/blz/issues/326)) ([f0ad6bb](https://github.com/outfitter-dev/blz/commit/f0ad6bb0d1c8b5f8f2cebbe03afdcecb5a5ca1a6))
* **cli:** replace Storage:: with Self:: in trait implementations ([#484](https://github.com/outfitter-dev/blz/issues/484)) ([3b1b0df](https://github.com/outfitter-dev/blz/commit/3b1b0dfced7cc3bab7673987a835658bc4a770fd))
* **cli:** restructure commands for better separation of concerns ([#471](https://github.com/outfitter-dev/blz/issues/471)) ([0dd40b3](https://github.com/outfitter-dev/blz/commit/0dd40b35526d9ee0e17e75c3515b36481a3e8012))
* **core:** add safe numeric conversion helpers ([#483](https://github.com/outfitter-dev/blz/issues/483)) ([e347a98](https://github.com/outfitter-dev/blz/commit/e347a98140c5ab3442720ba352669539d9b3d593))
* **core:** extract refresh helpers for MCP reuse ([#374](https://github.com/outfitter-dev/blz/issues/374)) ([3107eee](https://github.com/outfitter-dev/blz/commit/3107eee30b029c6f5835608a19f4af32989eefdf))
* eliminate script redundancy with shared common.sh ([f3c861b](https://github.com/outfitter-dev/blz/commit/f3c861b9bd8bd35c0ec7adfcab4ae6bb64549725))
* **mcp,core:** decompose long functions in server, sources, index, refresh ([#481](https://github.com/outfitter-dev/blz/issues/481)) ([b7705e4](https://github.com/outfitter-dev/blz/commit/b7705e42ddfcc3867d69b0bdeb1d6e9445994d4b))
* **mcp:** add action-based find tool ([#375](https://github.com/outfitter-dev/blz/issues/375)) ([8acf93e](https://github.com/outfitter-dev/blz/commit/8acf93e123f303e525eb5ccf2852dc2c79043a88))
* **mcp:** add blz tool for source actions ([#377](https://github.com/outfitter-dev/blz/issues/377)) ([b0bb471](https://github.com/outfitter-dev/blz/commit/b0bb4711b8e40f677eef4c36df4bfe5d158e903f))
* refine branding to use 'blz' as primary name ([4bd2b1c](https://github.com/outfitter-dev/blz/commit/4bd2b1cacd66c8f91f220085ffd9ce0d193d5d8b))


### Documentation

* **.agents:** add use-branchwork guide; update README examples to use `just` form; add `branchwork log` subcommand ([#64](https://github.com/outfitter-dev/blz/issues/64)) ([48de01b](https://github.com/outfitter-dev/blz/commit/48de01beecd8921f7c3ec5033e8f74d4ba07e878))
* add CHANGELOG.md for v0.1.6 release ([d567000](https://github.com/outfitter-dev/blz/commit/d567000f9e0efe83b5da5d3b8feade068bb0b1da))
* add comprehensive agent handoff document ([ee8e619](https://github.com/outfitter-dev/blz/commit/ee8e6190a3b2a2f2b26845d0ec6fb519cd51cb38))
* add comprehensive code improvements handoff document ([9c00363](https://github.com/outfitter-dev/blz/commit/9c00363ed0b5692c61b38262c886b275c54e9357))
* add comprehensive documentation directory ([4506004](https://github.com/outfitter-dev/blz/commit/450600436c6068dfb19b9f7b2f962296b765d1bb))
* add comprehensive release flow migration plan ([#344](https://github.com/outfitter-dev/blz/issues/344)) ([d8b2888](https://github.com/outfitter-dev/blz/commit/d8b2888756bcd9ec18deef89084d7b23948fb3be))
* add error sections to result fns ([#442](https://github.com/outfitter-dev/blz/issues/442)) ([#455](https://github.com/outfitter-dev/blz/issues/455)) ([a620cab](https://github.com/outfitter-dev/blz/commit/a620cab24ebfb4f67fe20692b976b7cbce5b54ed))
* add language filtering migration guide [BLZ-268] ([#332](https://github.com/outfitter-dev/blz/issues/332)) ([073f42d](https://github.com/outfitter-dev/blz/commit/073f42dd08a276165c945d836df0ad1e603c963f))
* add nextest installation instructions to README ([#120](https://github.com/outfitter-dev/blz/issues/120)) ([5d5fbde](https://github.com/outfitter-dev/blz/commit/5d5fbdebb7f98b6ab7bc1b3a84b45600cab43bfb))
* add performance benchmarks showing 6ms search latency ([42db13c](https://github.com/outfitter-dev/blz/commit/42db13c53cb75c3b881a8adade3bdd0d3aad9d71))
* add style guide for honest, humble writing tone ([8e4cb2c](https://github.com/outfitter-dev/blz/commit/8e4cb2c17949e12cea0ecf42f0b300beac518991))
* archive scratchpad and clarify agent logging ([4671d7e](https://github.com/outfitter-dev/blz/commit/4671d7eaf618d8cf4d302aac2e0d795eeabbda74))
* archive scratchpad and clarify agent logging ([#247](https://github.com/outfitter-dev/blz/issues/247)) ([4671d7e](https://github.com/outfitter-dev/blz/commit/4671d7eaf618d8cf4d302aac2e0d795eeabbda74))
* **changelog:** add unified find command [BLZ-229] ([8e63a6d](https://github.com/outfitter-dev/blz/commit/8e63a6de6f60783d3c24c588fe1fe717069e82ef))
* **cli:** align help text and prompts with v1.0.1 patterns [BLZ-150] ([#261](https://github.com/outfitter-dev/blz/issues/261)) ([ea7085d](https://github.com/outfitter-dev/blz/commit/ea7085dacfa7c05dfacc2921ec6b556299cce2be))
* **cli:** document add command types ([#437](https://github.com/outfitter-dev/blz/issues/437)) add rustdoc for add request, descriptor input, and flow options closes [#437](https://github.com/outfitter-dev/blz/issues/437) ([#450](https://github.com/outfitter-dev/blz/issues/450)) ([12f924e](https://github.com/outfitter-dev/blz/commit/12f924ea61a14bed6187db9010d4c5bc7ee2df1a))
* **cli:** document command output types ([#436](https://github.com/outfitter-dev/blz/issues/436)) ([34837bd](https://github.com/outfitter-dev/blz/commit/34837bd6a4fbcbb7bb2ab56921c398bf864ca33a))
* **cli:** document command output types ([#436](https://github.com/outfitter-dev/blz/issues/436)) add rustdoc for doctor, list, remove, validate, update, and clear output types closes [#436](https://github.com/outfitter-dev/blz/issues/436) ([#449](https://github.com/outfitter-dev/blz/issues/449)) ([34837bd](https://github.com/outfitter-dev/blz/commit/34837bd6a4fbcbb7bb2ab56921c398bf864ca33a))
* **cli:** document default history limit [BLZ-153] ([#274](https://github.com/outfitter-dev/blz/issues/274)) ([162f6a7](https://github.com/outfitter-dev/blz/commit/162f6a7dd04331797199a8e6e06695b8c07f3b79))
* **cli:** document enum variants ([#441](https://github.com/outfitter-dev/blz/issues/441)) add docs for docs format and prompt channel variants; output format variants already documented closes [#441](https://github.com/outfitter-dev/blz/issues/441) ([#452](https://github.com/outfitter-dev/blz/issues/452)) ([01c13ae](https://github.com/outfitter-dev/blz/commit/01c13aeb212c0ca7953ef83941a57120089f2cbb))
* **cli:** document json contract types ([#435](https://github.com/outfitter-dev/blz/issues/435)) add rustdoc for get json contract types, payload variants, and response metadata closes [#435](https://github.com/outfitter-dev/blz/issues/435) ([#448](https://github.com/outfitter-dev/blz/issues/448)) ([51d1fe3](https://github.com/outfitter-dev/blz/commit/51d1fe3f0adbefac7808426ebc050fae12620487))
* **cli:** document output formatters ([#438](https://github.com/outfitter-dev/blz/issues/438)) add rustdoc for format params fields and formatter structs across output modules closes [#438](https://github.com/outfitter-dev/blz/issues/438) ([#451](https://github.com/outfitter-dev/blz/issues/451)) ([1427e0c](https://github.com/outfitter-dev/blz/commit/1427e0cd61888f0245dbc5da737b6276a095c7c9))
* **cli:** document toc filtering ([#321](https://github.com/outfitter-dev/blz/issues/321)) ([9708486](https://github.com/outfitter-dev/blz/commit/97084860f89142418a45ce6ccebe9eaede58cd4c))
* **cli:** document utility functions ([#443](https://github.com/outfitter-dev/blz/issues/443)) add rustdoc for preference helpers, history log utilities, search helpers, and staleness functions closes [#443](https://github.com/outfitter-dev/blz/issues/443) ([#454](https://github.com/outfitter-dev/blz/issues/454)) ([35dea46](https://github.com/outfitter-dev/blz/commit/35dea46428e458618c519a36f124f6a895bb227a))
* **cli:** document utility structs ([#439](https://github.com/outfitter-dev/blz/issues/439)) add rustdoc for preferences, store, and toc utility structs and fields closes [#439](https://github.com/outfitter-dev/blz/issues/439) ([#453](https://github.com/outfitter-dev/blz/issues/453)) ([00e6cba](https://github.com/outfitter-dev/blz/commit/00e6cbab88457a89425cec96a14729868e898086))
* **cli:** enhance search pagination docs and add try_this examples [BLZ-116] ([#257](https://github.com/outfitter-dev/blz/issues/257)) ([ff58771](https://github.com/outfitter-dev/blz/commit/ff587713ef18179a8da61e95084b738c0bf9c874))
* **cli:** fix doc markdown warnings ([#436](https://github.com/outfitter-dev/blz/issues/436)) ([34837bd](https://github.com/outfitter-dev/blz/commit/34837bd6a4fbcbb7bb2ab56921c398bf864ca33a))
* complete v1.0.1 documentation polish for consistency and accuracy [BLZ-149] ([#260](https://github.com/outfitter-dev/blz/issues/260)) ([cec756d](https://github.com/outfitter-dev/blz/commit/cec756d404f5ce5a38715706a08d3b1e24eac6d8))
* comprehensive documentation update ([dd8c180](https://github.com/outfitter-dev/blz/commit/dd8c180106faa7ee55aea18f3a93aeb284bf96f2))
* **core:** add module-level rustdoc ([#434](https://github.com/outfitter-dev/blz/issues/434)) add module overviews for cache, async i/o, optimized index, memory pool, string pool, and search index internals closes [#434](https://github.com/outfitter-dev/blz/issues/434) ([#446](https://github.com/outfitter-dev/blz/issues/446)) ([bcd5da9](https://github.com/outfitter-dev/blz/commit/bcd5da9fd16642c96e3d673e4454413a37cbcbd5))
* **core:** document stats and summaries ([#440](https://github.com/outfitter-dev/blz/issues/440)) document stats and summary structs across async io, cache, memory pool, string pool, and optimized index closes [#440](https://github.com/outfitter-dev/blz/issues/440) ([#447](https://github.com/outfitter-dev/blz/issues/447)) ([ab42011](https://github.com/outfitter-dev/blz/commit/ab420116cbfe8b54a6bf2f835ca184ab5f79c98f))
* **devtools-01:** link to local hooks + nextest docs (BLZ-44) ([2ec34bd](https://github.com/outfitter-dev/blz/commit/2ec34bd2ef37f196456fb9105653184bebec0204))
* document get JSON migration coordination [BLZ-202] ([#280](https://github.com/outfitter-dev/blz/issues/280)) ([6f75d32](https://github.com/outfitter-dev/blz/commit/6f75d322421b069b80e2b3f6c69fcde98237806f))
* enhance Claude AI integration and development tooling ([4398408](https://github.com/outfitter-dev/blz/commit/43984088a8c838218dab0e51de3ecf495ad3c7c5))
* **factory:** add Factory command templates [BLZ-138] ([#275](https://github.com/outfitter-dev/blz/issues/275)) ([0842935](https://github.com/outfitter-dev/blz/commit/0842935eabd9985fd23835c394eedf40db7cc24d))
* fix inconsistent 'blz sources' references to 'blz list' ([869a6b1](https://github.com/outfitter-dev/blz/commit/869a6b19e28b9436af11ac4ff3cd7e30b3eadfcb))
* improve MCP documentation ([#299](https://github.com/outfitter-dev/blz/issues/299)) ([2195a92](https://github.com/outfitter-dev/blz/commit/2195a9219337085f529c1ca8f2bcbd87235a4fa1))
* **mcp:** add mcp server rustdoc ([#444](https://github.com/outfitter-dev/blz/issues/444)) ([#456](https://github.com/outfitter-dev/blz/issues/456)) ([4760e93](https://github.com/outfitter-dev/blz/commit/4760e93fb7ced82cb7ba9f01b57d8eacb36e040b))
* **mcp:** document the MCP server [BLZ-215] ([#292](https://github.com/outfitter-dev/blz/issues/292)) ([727dc88](https://github.com/outfitter-dev/blz/commit/727dc889dfdbd7666a40e3091acca2c23e80968c))
* prioritize direct CLI usage over MCP server ([adfe8bc](https://github.com/outfitter-dev/blz/commit/adfe8bca36f97761845af254f406214e837c0072))
* **prompts:** update get JSON guidance [BLZ-200] ([#278](https://github.com/outfitter-dev/blz/issues/278)) ([c999925](https://github.com/outfitter-dev/blz/commit/c99992528b4a49b810cf22ea5d151e715b1b6a15))
* refresh CLI docs for new get JSON [BLZ-201] ([#279](https://github.com/outfitter-dev/blz/issues/279)) ([b918bfd](https://github.com/outfitter-dev/blz/commit/b918bfd6566e22d3e9131ebd4deed2e7df76a64b))
* **release:** align docs with release-please ([#350](https://github.com/outfitter-dev/blz/issues/350)) ([0d8a9b9](https://github.com/outfitter-dev/blz/commit/0d8a9b91cea2462a372f5b9e734de3e15ede7f3d))
* **release:** align release-please docs ([#381](https://github.com/outfitter-dev/blz/issues/381)) ([e9dc394](https://github.com/outfitter-dev/blz/commit/e9dc394451edede34dbf6274f21e65f43f737792))
* **release:** archive prerelease notes ([#345](https://github.com/outfitter-dev/blz/issues/345)) ([b0b704f](https://github.com/outfitter-dev/blz/commit/b0b704ff82ef2d3d9f0bbde9da3ab98b20c0fcde))
* **release:** document release-please flow and archive legacy workflows ([#354](https://github.com/outfitter-dev/blz/issues/354)) ([f82c177](https://github.com/outfitter-dev/blz/commit/f82c1770031541942d1864374484a0006660d884))
* **release:** draft 1.4.0 changelog ([#378](https://github.com/outfitter-dev/blz/issues/378)) ([d207ef8](https://github.com/outfitter-dev/blz/commit/d207ef8778f21aca42945e263e5b9509ab0fd06e))
* reorganize and expand documentation structure ([b668515](https://github.com/outfitter-dev/blz/commit/b6685153e34fcc4e8a49587000f19a7f77724afa))
* replace deprecated 'blz get' with 'blz find' in documentation ([#364](https://github.com/outfitter-dev/blz/issues/364)) ([a6023a3](https://github.com/outfitter-dev/blz/commit/a6023a3890aeb5caf33cfd785320744713950d75))
* standardize terminology and fix examples consistency ([#7](https://github.com/outfitter-dev/blz/issues/7)) ([b368f51](https://github.com/outfitter-dev/blz/commit/b368f51373251e0e3a78743df381d8612571f05b))
* standardize terminology and fix examples consistency ([#7](https://github.com/outfitter-dev/blz/issues/7)) ([#14](https://github.com/outfitter-dev/blz/issues/14)) ([488de36](https://github.com/outfitter-dev/blz/commit/488de364a9ce02b299b22d4cf2eba49f7254a2e5))
* steer scratch logging to Linear ([4671d7e](https://github.com/outfitter-dev/blz/commit/4671d7eaf618d8cf4d302aac2e0d795eeabbda74))
* streamline blazer.md and use-blz.md for agents ([#366](https://github.com/outfitter-dev/blz/issues/366)) ([6a49f3c](https://github.com/outfitter-dev/blz/commit/6a49f3c377b3c955b07f4492af89611f8703e5fc))
* tighten get prompt guidance [BLZ-218] ([#281](https://github.com/outfitter-dev/blz/issues/281)) ([23fb48c](https://github.com/outfitter-dev/blz/commit/23fb48c47d35359cfc340654b41d7dc4194a4c67))
* update changelog for upcoming 0.4.1 release ([#231](https://github.com/outfitter-dev/blz/issues/231)) ([f20a69b](https://github.com/outfitter-dev/blz/commit/f20a69bfc0b7b4dd32dcac0df8b68ec50c6c4862))
* update commands.md for find/search/get deprecation and XDG fallback ([#485](https://github.com/outfitter-dev/blz/issues/485)) ([9ca5ecf](https://github.com/outfitter-dev/blz/commit/9ca5ecf09db1d5e3c3c1848b1da4be73bba94aa5))
* update documentation and add release automation for v0.3 ([#201](https://github.com/outfitter-dev/blz/issues/201)) ([b0351fa](https://github.com/outfitter-dev/blz/commit/b0351fac57b922f99be298062b3f50458cd4b50b))
* update README with refined definition and accurate commands ([95cd742](https://github.com/outfitter-dev/blz/commit/95cd7421c5284aae9e810e273005e54abb84f606))
* update shell_integration.md for modern find command ([#486](https://github.com/outfitter-dev/blz/issues/486)) ([5027117](https://github.com/outfitter-dev/blz/commit/5027117d7a214581813ad4981eb73e4907dfbaea))
* **workflow:** refresh release checklist ([#414](https://github.com/outfitter-dev/blz/issues/414)) ([896d277](https://github.com/outfitter-dev/blz/commit/896d27702331821103016ca935bf90988b64028e))
* **workflows:** align version management with release-please ([#380](https://github.com/outfitter-dev/blz/issues/380)) ([7b097b9](https://github.com/outfitter-dev/blz/commit/7b097b94312bc621cc7d2b76f8d74638b4deb8e8))
* **workflows:** expand CI/CD documentation with comprehensive pipeline details ([#217](https://github.com/outfitter-dev/blz/issues/217)) ([22a4fe0](https://github.com/outfitter-dev/blz/commit/22a4fe0f1482003b23fa0027ce745642db73d9a8)), closes [#211](https://github.com/outfitter-dev/blz/issues/211)

## [2.0.0-beta.1] - 2026-01-27

### ⚠️ Breaking Changes

- **CLI Restructuring**: Purpose-specific commands replace "smart" unified commands
- **New Primary Commands**:
  - `query` - Full-text search (replaces `search`)
  - `get` - Retrieve by citation (promoted from hidden)
  - `map` - Browse documentation structure (replaces `toc`)
  - `sync` - Fetch latest documentation (replaces `refresh`)
  - `check` - Validate source integrity (replaces `validate`)
  - `rm` - Remove source (replaces `remove`)
- **Deprecated Commands**: `find`, `toc`, `refresh`, `validate`, `remove` show deprecation warnings
  - Suppress with `BLZ_SUPPRESS_DEPRECATIONS=1`

### Features

- Grouped `--help` output with logical command categories
- Shell completions improvements

### Refactoring

- CLI command module restructuring for better maintainability
- Codebase cleanup with improved clippy compliance

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
