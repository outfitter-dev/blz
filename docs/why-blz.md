# docs/why-blz.md

## Why `blz`?

Agents don’t need pages—they need the *right lines*. `blz` is a local, line-exact retriever for the `llms.txt` ecosystem that delivers millisecond lookups and tiny, auditable snippets for coding agents.

## The status quo (how IDE agents use docs today)

Most coding tools fetch documentation (or search the web), then *paste large chunks into the model’s context*. That inflates latency, explodes token usage, and makes reasoning brittle.

- **Cursor**  
  - Public docs: `@Docs` (connects to official documentation) and `@Web` (live internet search). Both routes feed retrieved text into the LLM's prompt context.  
  - Cursor's own guidance: too little context leads to hallucination, but **too much irrelevant context "dilutes the signal."** That's exactly what happens when you shovel long passages instead of tight spans.  
  - Cursor also supports **MCP** to pull internal docs into the model's context pipeline. Powerful—still fundamentally "fetch then stuff."

- **Claude Code (Anthropic)**  
  - First-class **MCP** integration (local/remote servers). Great for connecting data sources, but most servers are fetch-first and return bulk content that ends up in the prompt.  
  - Anthropic recently launched one-click **Desktop Extensions** to make MCP installs easier—improves setup, not the "paste big docs" pattern.

- **Generic RAG stacks**  
  - Popular SDKs/templates encourage retrieval → chunk → **append to prompt**. Useful, but network-bound and token-intensive unless you add a disciplined re-ranking and span-slicing layer.

**Bottom line:** today's doc flows are optimized for human reading or "page-level RAG," not for *agentic precision*. They pay a latency tax (network fetch) and a token tax (big blobs), and they often lack deterministic, line-level citations. Cursor even calls out the risk of over-stuffing context.

## A different retrieval model (span-first, local-first)

`blz` flips the pattern:

1) **Preload & index** `llms.txt` / `llms-full.txt` locally (ETag/If-Modified-Since for freshness).  
2) **Search in ~6 ms** using Tantivy over *heading-sized blocks* (BM25).  
3) **Return precise spans**: `file#Lstart-Lend` + heading path + tight snippet (dozens of tokens, not thousands).  

This “span-first” model is agent-native: tiny, deterministic payloads that slot into prompts without blowing the budget.

```bash
$ blz bun "watch mode"
Bun > CLI > Test runner
  Lines 423-445: Run tests with --watch to re-run on file changes…
  src: https://bun.sh/llms.txt#L423-L445
# Typical end-to-end: ~6ms on a warm cache (see PERFORMANCE.md)
```

## Where `llms.txt` fits

- **`/llms.txt`** is a simple Markdown standard to expose the right docs for LLMs; many sites also publish **`/llms-full.txt`** as a single expanded bundle.  
- **Example**: Bun ships a large `llms-full.txt` (excellent stress test).  
- **Don't have one?** Firecrawl can **generate** `llms.txt`/`llms-full.txt` from any site (UI + API).

## Why this matters for agents (not just humans)

- **Latency**: local, index-backed search avoids 100s of ms per fetch; you get millisecond hits.  
- **Token economy**: agents operate on **line-level** facts; span outputs keep prompts lean.  
- **Determinism**: stable IDs (`file#Lstart-Lend`) → reproducible reasoning and easy audits.  
- **Scope control**: repo-scoped preload means your agent only searches relevant tool docs.

## How blz integrates with IDE agents

- **Direct CLI**: Agents can run `blz` commands directly—no server needed. Simple `blz search "query"` and `blz get alias --lines 123-145` commands return results in milliseconds.
- **Context strategy**: instead of dumping pages, agents call `search → get` to stitch 2–5 *spans* into a prompt.  
- **Optional RAG**: if you need semantic retrieval, plug spans into your existing AI SDK RAG flow—`blz` still supplies the precise citations.

## Comparison

| Concern | Fetch-and-stuff (typical) | `blz` span-first |
|---|---|---|
| **Latency** | 100-500ms network fetch | ~6ms local search |
| **Token usage** | 1000s (full pages/sections) | 10s-100s (exact spans) |
| **Determinism** | Content varies per fetch | Stable `file#L123-L145` citations |
| **Offline** | Requires network | Fully local after initial cache |
| **Updates** | Re-fetch everything | Conditional GET |
| **Context precision** | "Here's the whole page about X" | "Lines 423-445: exactly about X" |
