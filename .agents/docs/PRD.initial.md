<!-- note ::: @agents this was the original PRD and may no longer be accurate -->
# blz — Product Requirements Doc (PRD)

A local-first, line-accurate docs cache and MCP server for lightning-fast lookups of `llms.txt` ecosystems. Search in milliseconds, cite exact lines, keep diffs, and stay fresh via conditional fetches. Powered by Rust + Tantivy for speed and determinism; vectors are optional and **off by default**.

## Goals, Non-Goals, Success

**Goals**

- Local mirror of `llms.txt` sources arranged by tool (`bun/llms.txt`, etc.), plus any referenced markdown you elect to ingest.
- Blazing-fast **lexical** search across normalized docs, returning **precise line spans** with heading context.
- Deterministic JSON interfaces (CLI + MCP) designed for IDE/agent consumption.
- Durable parsing of imperfect `llms.txt`; always produce useful structure (headings, links, lines) even when inputs are sloppy.
- Built-in diff journal + optional archives per tool; track changes, timestamps, and changed sections.
- Fetch/sync from **registries you trust** and **Firecrawl’s llms.txt endpoint**; no arbitrary crawl by default. ([Firecrawl][1])

**Non-Goals (v1)**

- No default vector RAG or reranking (feature-flag later).
- No write-back to upstream docs.
- No remote HTTP API (MCP stdio first).

**Success criteria**

- P50 `search` end-to-end < 80 ms on a 10–50 MB corpus; P95 < 150 ms.
- `get_lines` returns exact `file#Lstart-Lend` slices with heading path + snippet.
- `update` uses conditional requests (ETag/If-None-Match) and produces a compact unified diff + “changed sections.” ([MDN Web Docs][2])

## User Stories

- **Agent author**: “Given ‘bun test concurrency’, return the three most relevant spans across `bun/llms.txt` and linked pages, with line ranges and headings, in <100 ms.”
- **Operator**: “Update all sources and tell me which sections changed since yesterday; show unified diffs and summarized change notes.”
- **IDE assistant**: “When a suggestion references a flag, paste only the exact lines and cite `file#Lstart-Lend`.”

## High-Level Requirements

**Functional**

1. `add <alias> <url>`: fetch `llms.txt` (or inline `<script type="text/llms.txt">`), normalize, index.
2. `search <query>`: hybrid lexical pipeline → JSON hits with `{file, headingPath, lines, snippet, score, sourceUrl, checksum}`.
3. `get <alias> --lines A-B`: exact span.
4. `update [alias|--all]`: conditional GET + reindex if changed; record diff.
5. `diff <alias> [--since TS]`: unified diff + changed sections.
6. `sources`: list state and metadata.

**Non-functional**

- Durable parsing; fallbacks for malformed Markdown/spec.
- Deterministic JSON; zero nondeterministic fields.
- Fast: in-process everything; minimal syscalls; parallelized IO.
- Safe defaults: whitelist-only fetch; bounded size/time.

## System Architecture

```
+---------------------------+
| CLI (blz)               |
|  - add / search / ...     |
+------------+--------------+
             | stdio
             v
+---------------------------+       +----------------------+
| MCP Server (rmcp)         |<----->| MCP Clients (IDE/AI) |
| Tools: search/get/diff    |       +----------------------+
+------------+--------------+
             |
             v
+---------------------------+      +-------------------------+
| Core Engine (Rust)        |----->| Tantivy Index           |
|  - Fetcher (reqwest, ETag)|      |  heading-block docs     |
|  - Normalizer (tree-sitter)|     +-------------------------+
|  - LineMap & TOC          |
|  - Search (fuzzy→lexical→rg)
|  - Diff (similar)         |
+------------+--------------+
             |
             v
+---------------------------+
| Storage (per tool)        |
|  llms.txt / llms.json     |
|  .index/ (tantivy)        |
|  .archive/ (snapshots)    |
|  diffs.log.jsonl          |
|  settings.toml            |
+---------------------------+
```

- **Tantivy** is the embedded search engine (Lucene-style) for sub-10ms doc hits. ([Docs.rs][3], [GitHub][4])
- **tree-sitter-markdown** for robust parsing and byte/line positions. ([GitHub][5], [Docs.rs][6])
- **ripgrep** as the exact line-finder for final spans/snippets (subprocess, optional). ([GitHub][7])
- **similar** for unified diffs and changed-section mapping. ([Docs.rs][8], [GitHub][9])
- **MCP** uses the **official Rust SDK (`rmcp`)** for maximum ecosystem support. ([GitHub][10], [Docs.rs][11])
- Fetch/sync favors conditional requests (ETag/If-None-Match) to save bandwidth and time. ([MDN Web Docs][2])
- Optional discovery via **Firecrawl’s `/llmstxt`** generator (off unless requested). ([Firecrawl][1])

## On-Disk Layout

```
~/.outfitter/blz/
  global.toml
  bun/
    llms.txt                 # latest upstream text
    llms.json                # parsed TOC + line map + metadata
    .index/                  # Tantivy index
    .archive/
      2025-08-22T12-01Z-llms.txt
      2025-08-22T12-01Z-llms.json
      2025-08-22T12-01Z.diff
    diffs.log.jsonl          # append-only journal
    settings.toml            # per-tool overrides
```

**`llms.json` minimal schema**

```json
{
  "alias": "bun",
  "source": {"url": "https://bun.sh/llms.txt", "etag": "…", "lastModified": "…", "fetchedAt": "…", "sha256": "…"},
  "toc": [{"headingPath": ["Install"], "lines": "120-168", "children": []}],
  "files": [{"path": "llms.txt", "sha256": "…"}],
  "lineIndex": {"totalLines": 1843, "byteOffsets": true},
  "diagnostics": [{"severity":"warn","message":"Missing H1","line":1}]
}
```

**`diffs.log.jsonl` entry**

```json
{"ts":"2025-08-22T12:01:03Z","alias":"bun","etagBefore":"…","etagAfter":"…",
 "shaBefore":"…","shaAfter":"…","unifiedDiffPath":".archive/2025-08-22T12-01Z.diff",
 "changedSections":[{"headingPath":["Install"],"lines":"120-142"}],
 "summary":"Install: adds --jit flag; fixes typos"}
```

Retention governed by `global.toml` and per-tool `settings.toml` (`max_archives`, `refresh_hours`, `fetch_enabled`, etc.).

## Parsing & Durability Strategy

1. **Raw load** (UTF-8), store metadata.
2. **tree-sitter-markdown** block parse → build TOC + heading ranges (line/byte). If parsing fails or input is sloppy, fall back to tolerant heuristics (`^#{1,6} ` headings, link lines, fenced code). ([Docs.rs][6])
3. **Spec aware**: treat top-level `llms.txt` as the root of trust; optionally expand only explicitly linked **.md** pages (allowlist by domain), controlled by config.
4. **Inline `<script type="text/llms.txt">`** scraping supported when encountered.
5. Always emit `llms.json` with `diagnostics[]` rather than failing hard.

## Search Pipeline (no vectors)

**Algorithm**

1. **Token prep / fuzzy prefilter** (in-proc fuzzy matcher) widens slightly misspelled terms.
2. **Tantivy** queries over **heading-sized blocks**: fields `{content, path, headingPath, line_start, line_end}` indexed for BM25.
3. For top-K candidates, run **ripgrep** to extract exact line spans and **tight snippets**.
4. Return ranked hits with deterministic scores and stable ordering.

**Hit JSON**

```json
{
  "alias":"bun",
  "file":"llms.txt",
  "headingPath":["CLI","Flags"],
  "lines":"311-339",
  "snippet":"--concurrency <N> ...",
  "score":12.47,
  "sourceUrl":"https://bun.sh/llms.txt#L311-L339",
  "checksum":"sha256:…"
}
```

Rationale: Tantivy for quick narrowing + BM25 relevance; ripgrep for precision at the line level. ([Docs.rs][3], [GitHub][7])

## MCP Surface (stdio)

We’ll use **`rmcp` (official Rust SDK)**. Server provides:

**Tools**

- `list_sources()` → array `{alias, path, fetchedAt, etag, size}`.
- `search({query, alias?, limit?})` → `hits[]` (JSON as above).
- `get_lines({alias, file, start, end})` → exact slice + MIME (`text/plain`).
- `update({alias?})` → `{updated:[], skipped:[], errors:[]}`.
- `diff({alias, since?})` → `{entries:[…], diffs:[{path, text}…]}`.

**Resources**

- `doc://<alias>/<file>#Lstart-Lend` resolves to cached content blob with position info (ideal for IDEs).

Version pinning follows MCP date-string spec (e.g., `2025-06-18`). ([Model Context Protocol][12], [spec.modelcontextprotocol.io][13])

## CLI (DX)

Binary name: `blz` .

```
blz add bun https://bun.sh/llms.txt          # fetch + index
blz search "test concurrency" --alias bun    # JSON hits
blz get bun --lines 120-142                  # span text
blz update --all
blz diff bun --since "2025-08-20T00:00:00Z"
blz sources
```

**Flags**

- `--format json|pretty` (default json)
- `--limit N` for search
- `--max-archive N`, `--refresh-hours H` (override per-tool)
- `--no-follow` / `--allow list=domain1,domain2`

## Update & Sync

- Conditional GET with `If-None-Match`/ETag and `If-Modified-Since`. `304` → skip reindex; `200` → snapshot + reindex + diff entry. ([MDN Web Docs][2])
- **Diffing** with `similar::TextDiff` (patience/Myers). Also compute `changedSections` by intersecting diff hunks with heading ranges. ([Docs.rs][8])
- **Discovery**: default resolvers = trusted registries you set + **Firecrawl `/llmstxt`** (opt-in). ([Firecrawl][1])

## Config (TOML)

**`global.toml`**

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party" # none|first_party|allowlist
allowlist = []

[paths]
root = "~/.outfitter/blz"
```

**`<alias>/settings.toml`**

```toml
[meta]
name = "Bun"
display_name = "Bun Runtime"
homepage = "https://bun.sh"
repo = "https://github.com/oven-sh/bun"

[fetch]
refresh_hours = 6
follow_links = "allowlist"
allowlist = ["bun.sh","github.com/oven-sh"]

[index]
max_heading_block_lines = 400
```

## Security & Privacy

- **Default-deny** remote fetch of non-listed domains.
- MCP tools are **read-only**; no shell escape; no arbitrary command execution.
- Snapshots and diffs are local; no telemetry unless explicitly enabled.

## Observability

- Structured logs (JSON) with timings for fetch, parse, index, search.
- `blz diag` dumps latest diagnostics and index stats.
- Optional OpenTelemetry traces behind a feature flag.

## Performance Targets & Benchmarks

- Index build: \~50–150 ms per 1 MB markdown (CPU-bound).
- Query: P50 < 80 ms end-to-end on laptop for 10–50 MB corpora.
- Update: conditional fetch round-trip + no-op reindex in < 30 ms.

(We’ll publish a `bench/` suite and capture results in CI.)

## Failure Modes & Recovery

- **Malformed `llms.txt`** → warnings in `diagnostics[]`, still produce `llms.json`; search remains operational on parsed regions.
- **Network failures** → keep last good snapshot; `update` surfaces errors but cache continues to serve.
- **Index corruption** → auto-rebuild from `llms.txt` snapshot.

## Agent Guide (guardrails)

- Prefer MCP tools `search` → `get_lines` over scraping files.
- Always cite exact `file#Lstart-Lend` spans.
- Only trigger `update` when freshness matters; otherwise use existing cache.
- To seed new tools: check known registries; if missing, try **Firecrawl `/llmstxt`** once and cache results. ([Firecrawl][1])

## Code Scaffolds (Rust)

> Intentionally compact to prime agents; not a full build.

**`Cargo.toml` (core)**

```toml
[package]
name = "outfitter-blz"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["gzip", "brotli", "json", "stream"] }
sha2 = "0.10"
base64 = "0.22"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "json"] }

# Parsing & indexing
tree-sitter = "0.22"
tree-sitter-md = "0.3"
tantivy = "0.22"
similar = "2"

# MCP server
rmcp = "0.4"  # official Rust MCP SDK

# optional (enable via feature)
# fuzzy-matcher = "0.3" # SkimMatcherV2
```

**ETag fetch (cond. GET)**

```rust
async fn fetch_with_blz(url: &str, etag: Option<&str>, last_mod: Option<&str>) -> anyhow::Result<reqwest::Response> {
    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if let Some(tag) = etag { req = req.header("If-None-Match", tag); }
    if let Some(lm)  = last_mod { req = req.header("If-Modified-Since", lm); }
    let resp = req.send().await?;
    Ok(resp)
}
```

(Use `If-None-Match`/ETag for lightweight updates.) ([MDN Web Docs][2])

**Markdown → heading blocks (line-mapped)**

```rust
use tree_sitter::{Parser, Node, Range};
use tree_sitter_md::LANGUAGE;

fn headings_with_ranges(text: &str) -> Vec<(Vec<String>, usize, usize)> {
    let mut parser = Parser::new();
    parser.set_language(LANGUAGE).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    // Walk blocks, collect ATX headings and their subsequent range until next heading.
    // Return: (["CLI","Flags"], start_line, end_line)
    // ...omitted: tree-sitter cursor walk, compute ranges by node.range().start_point.row
    vec![]
}
```

(tree-sitter gives row/column → exact line spans.) ([Docs.rs][6])

**Tantivy schema (heading-block docs)**

```rust
use tantivy::{schema::*, Index};

fn build_index(path: &std::path::Path) -> anyhow::Result<(Index, Schema, Field, Field, Field, Field)> {
    let mut schema_builder = Schema::builder();
    let f_content = schema_builder.add_text_field("content", TEXT | STORED);
    let f_path    = schema_builder.add_text_field("path", STRING | STORED);
    let f_hpath   = schema_builder.add_text_field("heading_path", TEXT | STORED);
    let f_lines   = schema_builder.add_text_field("lines", STRING | STORED); // "A-B"
    let schema = schema_builder.build();
    let index = Index::create_in_dir(path, schema.clone())?;
    Ok((index, schema, f_content, f_path, f_hpath, f_lines))
}
```

(Tantivy = Lucene-like speed/quality, embedded.) ([Docs.rs][3])

**Unified diff (changed sections)**

```rust
use similar::{TextDiff, ChangeTag};

fn unified_diff(a: &str, b: &str) -> String {
    TextDiff::from_lines(a, b).unified_diff().to_string()
}
```

([Docs.rs][8])

**MCP server (rmcp) tool sketch**

```rust
use rmcp::{Server, Result as McpResult};

#[derive(serde::Deserialize)]
struct SearchArgs { query: String, alias: Option<String>, limit: Option<usize> }

fn main() -> anyhow::Result<()> {
    let mut server = Server::stdio("outfitter.blz");
    server.tool("search", |args: SearchArgs| async move {
        // call core::search(args.query, args.alias, args.limit)
        // return serde_json::json!({ "hits": hits })
            .pipe_ok()
    });
    server.run()
}
```

(Official SDK; stdio transport keeps everything local and fast.) ([Docs.rs][11])

## JSON Schemas (Draft-07)

**Search response**

```json
{
  "$schema":"http://json-schema.org/draft-07/schema#",
  "title":"BlzSearchResponse",
  "type":"object",
  "properties":{
    "query":{"type":"string"},
    "alias":{"type":["string","null"]},
    "hits":{
      "type":"array",
      "items":{
        "type":"object",
        "required":["file","headingPath","lines","snippet","score"],
        "properties":{
          "file":{"type":"string"},
          "headingPath":{"type":"array","items":{"type":"string"}},
          "lines":{"type":"string","pattern":"^\\d+-\\d+$"},
          "snippet":{"type":"string"},
          "score":{"type":"number"},
          "sourceUrl":{"type":"string"},
          "checksum":{"type":"string"}
        }
      }
    }
  },
  "required":["query","hits"]
}
```

**Diff entry**

```json
{
  "$schema":"http://json-schema.org/draft-07/schema#",
  "title":"BlzDiffEntry",
  "type":"object",
  "required":["ts","alias","unifiedDiffPath","changedSections"],
  "properties":{
    "ts":{"type":"string","format":"date-time"},
    "alias":{"type":"string"},
    "unifiedDiffPath":{"type":"string"},
    "changedSections":{"type":"array","items":{
      "type":"object",
      "required":["headingPath","lines"],
      "properties":{
        "headingPath":{"type":"array","items":{"type":"string"}},
        "lines":{"type":"string","pattern":"^\\d+-\\d+$"}
      }
    }},
    "summary":{"type":"string"}
  }
}
```

## Risks & Mitigations

- **Upstream format drift** → resilient parser + diagnostics + raw archive.
- **Index bloat** → heading-block docs keep postings compact; rotate archives.
- **Ecosystem drift in MCP** → target spec `2025-06-18`, pin `rmcp` minor; integration tests. ([Model Context Protocol][12])

## Roadmap & Milestones

**MVP (Week 1)**

- CLI: `add`, `search`, `get`, `sources`.
- Parser + line maps + Tantivy index.
- Ripgrep-powered span extraction (optional dep).
- `llms.json` and basic diagnostics.

**v0.2 (Week 2)**

- `update` (ETag/If-Modified-Since) + `diff` (similar) + `.archive/` + `diffs.log.jsonl`.
- Per-tool `settings.toml`; global config.

**v0.3 (Week 3)**

- MCP server (`rmcp`): `list_sources`, `search`, `get_lines`, `update`, `diff`.
- Benchmarks + `blz diag`.

**v0.4+**

- Optional fuzzy prefilter feature flag.
- Optional vector extension (SQLite-vec/LanceDB) + Vercel AI SDK retriever (off by default). ([AI SDK][14])

## Reference Links

- **Tantivy** (docs & repo). Fast, embedded search in Rust. ([Docs.rs][3], [GitHub][4])
- **tree-sitter-markdown** grammar & crate. Line-accurate Markdown parsing. ([GitHub][5], [Docs.rs][6])
- **ripgrep**. Ridiculously fast line searches. ([GitHub][7])
- **similar** diff crate. Unified diffs, patience/Myers. ([Docs.rs][8])
- **HTTP ETag / If-None-Match** (MDN). Conditional requests. ([MDN Web Docs][15])
- **Model Context Protocol**: spec & official Rust SDK (`rmcp`). ([Model Context Protocol][12], [GitHub][16], [Docs.rs][11])
- **Firecrawl `/llmstxt`** generator (opt-in discovery). ([Firecrawl][1])
- **Vercel AI SDK: embeddings** (for later vector feature). ([AI SDK][14])

## Next Steps

1. Create repo `outfitter-dev/blz` with crates: `blz-cli`, `blz-core`, `blz-mcp`.
2. Implement MVP path: fetch → parse → index → search → get.
3. Wire conditional updates + diff journal and archives.
4. Stand up MCP tools on `stdio` using `rmcp` and add integration tests against a couple of real sources.
5. Publish a short **AGENT\_GUIDE.md** and **JSON schemas** so Claude Code/Cursor agents can plug in cleanly.

When you’re ready, I’ll scaffold the repo layout, paste initial Rust modules, and add a couple of end-to-end tests you can run to validate performance on your machine.

[1]: https://docs.firecrawl.dev/features/alpha/llmstxt?utm_source=chatgpt.com "LLMs.txt Generator"
[2]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/If-None-Match?utm_source=chatgpt.com "If-None-Match header - MDN"
[3]: https://docs.rs/tantivy/?utm_source=chatgpt.com "tantivy - Rust"
[4]: https://github.com/quickwit-oss/tantivy?utm_source=chatgpt.com "Tantivy is a full-text search engine library inspired ..."
[5]: https://github.com/tree-sitter-grammars/tree-sitter-markdown?utm_source=chatgpt.com "Markdown grammar for tree-sitter"
[6]: https://docs.rs/tree-sitter-md?utm_source=chatgpt.com "tree_sitter_md - Rust"
[7]: https://github.com/BurntSushi/ripgrep?utm_source=chatgpt.com "ripgrep recursively searches directories for a regex pattern while ..."
[8]: https://docs.rs/similar?utm_source=chatgpt.com "similar - Rust"
[9]: https://github.com/mitsuhiko/similar?utm_source=chatgpt.com "mitsuhiko/similar: A high level diffing library for rust based ..."
[10]: https://github.com/modelcontextprotocol/rust-sdk?utm_source=chatgpt.com "The official Rust SDK for the Model Context Protocol"
[11]: https://docs.rs/rmcp?utm_source=chatgpt.com "rmcp - Rust"
[12]: https://modelcontextprotocol.io/specification/2025-06-18?utm_source=chatgpt.com "Specification"
[13]: https://spec.modelcontextprotocol.io/?utm_source=chatgpt.com "Model Context Protocol: Versioning"
[14]: https://ai-sdk.dev/docs/ai-sdk-core/embeddings?utm_source=chatgpt.com "AI SDK Core: Embeddings"
[15]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/ETag?utm_source=chatgpt.com "ETag header - MDN - Mozilla"
[16]: https://github.com/modelcontextprotocol?utm_source=chatgpt.com "Model Context Protocol"
