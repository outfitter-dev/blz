# BLZ Documentation

**blz** /bleɪz/ *noun*

1. A local-first search cache for `llms.txt` documentation ecosystems
2. A CLI tool delivering millisecond-latency searches with exact line citations
3. A span-first retrieval model for coding agents

## 📚 Documentation Index

### Getting Started

- [**Quick Start**](QUICKSTART.md) - Installation and first steps
- [**CLI Overview**](cli/README.md) - CLI installation, overview, and index
- [**How-To Guide**](cli/howto.md) - Task-oriented "I want to..." solutions

### CLI Reference

- [**Commands**](cli/commands.md) - Complete command reference
- [**Search**](cli/search.md) - Search syntax, performance tips, and advanced queries
- [**Sources**](cli/sources.md) - Managing documentation sources
- [**Configuration**](cli/configuration.md) - Global config, per-source settings, env vars
- [**Shell Integration**](cli/shell_integration.md) - Setup for Bash, Zsh, Fish, PowerShell, Elvish

### Technical Details

- [**Architecture**](architecture/README.md) - How BLZ works under the hood
- [**Performance**](architecture/PERFORMANCE.md) - Benchmarks and optimization details
- [**Storage**](architecture/STORAGE.md) - Storage layout and data organization

### Development

- [**Contributing**](../CONTRIBUTING.md) - Development guidelines
- [**Development Setup**](development/README.md) - Local development environment
- [**CI/CD**](development/ci_cd.md) - Continuous integration and deployment
- [**Testing**](development/testing.md) - Testing strategies and tools

---

## What is llms.txt?

`llms.txt` is a standardized Markdown format for making documentation accessible to AI agents. Sites publish their docs at URLs like `https://bun.sh/llms.txt` (concise) or `https://bun.sh/llms-full.txt` (comprehensive).

**Don't have one?** Tools like Firecrawl can generate `llms.txt` from any site.

## Why BLZ?

Agents don't need pages—they need the *right lines*. BLZ is a local, line-exact retriever for the `llms.txt` ecosystem that delivers millisecond lookups and tiny, auditable snippets for coding agents.

### The status quo (how IDE agents use docs today)

Most coding tools fetch documentation (or search the web), then *paste large chunks into the model's context*. That inflates latency, explodes token usage, and makes reasoning brittle.

- **Cursor**
  - `@Docs` (official documentation) and `@Web` (live internet search) feed retrieved text into the LLM's prompt context
  - Cursor's own guidance: too little context leads to hallucination, but **too much irrelevant context "dilutes the signal"**
  - Supports **MCP** to pull internal docs into the model's context pipeline—powerful, but still "fetch then stuff"

- **Claude Code (Anthropic)**
  - First-class **MCP** integration (local/remote servers) for connecting data sources
  - Most MCP servers are fetch-first and return bulk content that ends up in the prompt
  - One-click **Desktop Extensions** improve MCP setup, but not the "paste big docs" pattern

- **Generic RAG stacks**
  - Popular SDKs/templates: retrieval → chunk → **append to prompt**
  - Network-bound and token-intensive unless you add disciplined re-ranking and span-slicing

**Bottom line:** today's doc flows are optimized for human reading or "page-level RAG," not for *agentic precision*. They pay a latency tax (network fetch) and a token tax (big blobs), and they often lack deterministic, line-level citations.

### A different retrieval model (span-first, local-first)

BLZ flips the pattern:

1. **Preload & index** `llms.txt`/`llms-full.txt` locally (ETag/If-Modified-Since for freshness)
2. **Search in ~6 ms** using Tantivy over *heading-sized blocks* (BM25)
3. **Return precise spans**: `alias:start-end` + heading path + tight snippet (dozens of tokens, not thousands)
4. **Audit & repeat**: the same query yields the same line-exact result; diffs are logged when upstream changes

This "span-first" model is agent-native: tiny, deterministic payloads that slot into prompts without blowing the budget.

```bash
$ blz bun "watch mode"
Bun > CLI > Test runner
  Lines 423-445: Run tests with --watch to re-run on file changes…
  src: https://bun.sh/llms.txt#L423-L445
# Typical end-to-end: ~6ms on a warm cache (see architecture/PERFORMANCE.md)
```

### Why this matters for agents (not just humans)

- **Latency**: local, index-backed search avoids 100s of ms per fetch; you get millisecond hits
- **Token economy**: agents operate on **line-level** facts; span outputs keep prompts lean
- **Determinism**: stable IDs (`alias:start-end`) → reproducible reasoning and easy audits
- **Freshness without spam**: conditional GETs + diff journal; only re-index when the ETag changes
- **Scope control**: repo-scoped preload means your agent only searches relevant tool docs

### How BLZ integrates with IDE agents

- **Direct CLI**: Agents run `blz` commands directly—no server needed. Simple `blz "query"` and `blz get alias -l123-145` commands return results in milliseconds
- **Context strategy**: instead of dumping pages, agents call `search → get` to stitch 2–5 *spans* into a prompt
- **MCP server** (coming soon): For deeper integration, an MCP server will expose tools like `search`, `get_lines`, `update`, `diff`, `list_sources` via stdio
- **Optional RAG**: if you need semantic retrieval, plug spans into your existing AI SDK RAG flow—BLZ still supplies the precise citations

### Comparison

| Concern | Fetch-and-stuff (typical) | BLZ span-first |
|---|---|---|
| **Latency** | 100-500ms network fetch | ~6ms local search |
| **Token usage** | 1000s (full pages/sections) | 10s-100s (exact spans) |
| **Determinism** | Content varies per fetch | Stable `alias:123-145` citations |
| **Offline** | Requires network | Fully local after initial cache |
| **Updates** | Re-fetch everything | Conditional GET + diff tracking |
| **Context precision** | "Here's the whole page about X" | "Lines 423-445: exactly about X" |

---

## Quick Example

```bash
# Add Bun's documentation
blz add bun https://bun.sh/llms.txt

# Search instantly (6ms)
blz "test concurrency" -s bun

# Get specific lines
blz get bun:304-324
```

## Performance at a Glance

| Operation | Time | Notes |
|-----------|------|-------|
| Search | 6ms | P50 on real docs |
| Add source | 373ms | Includes fetch + parse + index |
| Get lines | <1ms | Direct file access |

## Need Help?

- Check the [Quick Start](QUICKSTART.md)
- Browse the [How-To Guide](cli/howto.md)
- Read about [search patterns](cli/search.md#common-patterns)
- File an issue on [GitHub](https://github.com/outfitter-dev/blz)
