# v1.0.0-beta.1 Improvement Plan (2025-10-03)

This document started as external findings from Gemini and has been restructured into an actionable improvement plan for v1.0.0-beta.1. Both agent reviews (Gemini and Claude) agreed on high-value, low-effort wins.

## Prioritized Implementation Plan

### 🚀 P0: Ship for v1.0.0-beta.1

All three P0 items share the same serialization surface. Implement them in the order below so we only touch the formatter/tests once and the later flags can lean on the enriched structs.

#### 1. Enriched JSON Output (foundation)
**Status**: Approved for implementation
**Effort**: Low (4 new fields in `SearchHit`)
**Value**: High (better agent decision-making)

Add metadata to search results (and reuse in `get`/`list` so responses stay consistent):

```json
// Current (minimal)
{
  "alias": "react",
  "score": 15.3,
  "lines": "500-510",
  "content": "..."
}

// Enriched (agent-friendly)
{
  "alias": "react",
  "score": 15.3,
  "lines": "500-510",
  "content": "...",
  "source_url": "https://react.dev/llms-full.txt",
  "fetched_at": "2025-10-03T10:00:00Z",
  "is_stale": false,
  "heading_path": ["Hooks", "useEffect"]
}
```

**New fields:**
- `source_url`: Original llms.txt URL for provenance (pulled from existing descriptor metadata)
- `fetched_at`: When source was last updated (from metadata)
- `is_stale`: Boolean based on configurable TTL (default: >30 days)
- `heading_path`: Hierarchical heading path for context

**Implementation notes:**
- Wire these fields into the shared `SearchHit` struct first; `search`, `get`, and `list --json` can then share serializers.
- Update CLI help/JSON schema docs once so follow-on flags don't duplicate work.
- Persist `is_stale` alongside doctor output to keep health checks and search hits aligned.

#### 2. Context Flag for Search (default-friendly)
**Status**: Approved for implementation
**Effort**: Trivial (leverage existing `get` logic)
**Value**: High (50% reduction in API calls)

Add `--context` to `search` command:

```bash
# Currently requires two commands
blz search "useState" --json | jq -r '.[0] | "\(.alias):\(.lines)"'
blz get react:450-455 --context 5

# With this feature (default context = 5 when no value supplied)
blz search "useState" --context
```

**Implementation:**
- Reuse context retrieval logic from `get` command; default to 5 lines before/after when the flag is present without an explicit value.
- Return enriched results with surrounding lines and surface the default in CLI help so agents know what to expect.
- Keep the flag mutually exclusive with `--block` for a clear mental model.

#### 3. Block-Based Context Retrieval
**Status**: Approved for implementation
**Effort**: Low (infrastructure already exists)
**Value**: High (semantic completeness + fewer round-trips)

Add `--block` and `--max-lines` flags to both `search` and `get` commands:

```bash
# Return entire heading section containing match
blz search "useEffect cleanup" --block

# Cap block size for huge sections
blz search "API reference" --block --max-lines 100

# Also works with get
blz get react:450 --block
```

**Implementation details:**
- Built atop the enriched `SearchHit` serialization so blocks and context emit the same metadata.
- Mutually exclusive with `--context` (simpler mental model) and the CLI help/JSON format docs must call this out explicitly for agents parsing responses.
- Leverages existing `HeadingBlock` data structure
- Block boundaries already tracked (`start_line`, `end_line`, `path`)
- Optional `--max-lines` safety valve prevents massive returns

**Benefits:**
- Agents get semantically complete sections (entire concept, not fragments)
- Preserves heading hierarchy automatically
- One command instead of `search` → `get`
- Better for RAG: coherent chunks vs arbitrary line ranges

### ✅ Pre-Announcement Easy Wins

Before calling v1.0.0-beta.1 "public", land the quick polish items below so the new surface area is well-documented and battle-tested:

- **CLI help + docs refresh**: Update `--help`, README snippets, and the docs site examples for `--context`, `--block`, `--max-lines`, and the enriched JSON fields so agents and humans see the same contract the code ships.
- **Golden output snapshots**: Add/update unit or snapshot tests that cover enriched JSON, `--context` defaulting, and block responses. Cheap insurance against regressions while the schema settles.
- **Release notes & upgrade guidance**: Draft the beta.1 changelog summarizing the new flags plus a short "how to consume `source_url`/`is_stale`" blurb for downstream agents. Publish alongside the release tag.
- **Doctor alignment**: Ensure `blz doctor` surfaces the same staleness signals introduced in `SearchHit` so support/docs point to one canonical metric.

### 📋 P1: Post v1.0.0-beta.1

#### 4. Registry-Based Starter Packs
**Status**: Deferred (needs registry infrastructure)
**Effort**: Medium (requires registry repo changes)
**Value**: Medium (nice onboarding UX)

Enable community-maintained manifest bundles:

```bash
blz add --manifest registry:react-ecosystem
# Resolves to: https://raw.githubusercontent.com/outfitter-dev/blz-registry/main/packs/react-ecosystem.toml
```

**Prerequisites:**
- Create `/packs` directory in registry repo
- Define pack manifest format
- Implement registry:// URL resolver

**Execution notes:**
- Land on top of the manifest ingestion work already in progress so `blz add --manifest <local file>` and registry packs share the same plumbing.
- Document pack availability in CLI help once the registry publishes its first bundle to keep the agent story discoverable.

#### 5. Remote Manifest Support
**Status**: Deferred (security design needed)
**Effort**: Low (~20 LOC) but needs security review
**Value**: Medium (enables sharing curated lists)

Extend existing `--manifest` to accept URLs:

```bash
blz add --manifest https://gist.github.com/user/react-sources.toml
```

**Security requirements:**
- SHA-256 verification (like install script)
- `--allow-remote` flag required
- Warning on unsigned manifests
- No auto-execution without explicit consent

**Execution notes:**
- Ship after registry packs so we have a trusted namespace before opening arbitrary URLs.
- Reuse the install-script checksum UX; fail closed unless the hash or explicit `--allow-remote` override is supplied.

#### 6. Enhanced Registry Discovery
**Status**: Deferred (post-v1.0)
**Effort**: Medium
**Value**: Medium (better than web-crawling approach)

Make `blz lookup` smarter with fuzzy domain matching:

```bash
# Current: exact name matching
blz lookup tanstack

# Enhanced: fuzzy domain/description matching
blz lookup tanstack.com  # Finds tanstack/query, tanstack/router
blz lookup "react state" # Finds zustand, jotai, redux
```

**Implementation:**
- Registry-driven (no web crawling)
- Fuzzy matching on name, description, URL
- JSON output ready for piping to `blz add`

#### 7. Staleness-Driven Update Mode
**Status**: Proposed (post-v1.0.0-beta.1)
**Effort**: Low (builds on `is_stale` metadata)
**Value**: Medium (keeps caches fresh with fewer fetches)

Add targeted updates once `is_stale` ships alongside enriched search results:

```bash
# Only refresh sources older than the default TTL
blz update --only-stale

# Custom window for heavy users
blz update --only-stale --stale-since=7d
```

**Implementation:**
- Reuse the staleness metadata surfaced in P0 to avoid redundant timestamp logic.
- Integrate with `blz doctor` output so recommended fixes can auto-run `--only-stale`.
- Leave room for cron examples (instead of a daemon) once this flag exists.

### ❌ Won't Do (Rejected)

#### Background Daemon
**Reason**: Over-engineering, platform complexity, philosophy clash

Instead: Document cron/scheduled task examples in README:
```bash
# Update all sources every 6 hours (cron)
0 */6 * * * /usr/local/bin/blz update --all --quiet
```

#### Generic RAG Engine
**Reason**: Dilutes focus, premature expansion

Focus on owning the `llms.txt` niche first. Expansion can come after product-market fit.

#### Interactive fzf Modes
**Reason**: Against Unix philosophy (composability > integration)

Current approach works better:
```bash
blz list | fzf | xargs blz info
```

#### Discovery via Web Crawling
**Reason**: Fragile, scope creep, non-standard `<link>` tags

Use registry-based discovery instead (enhanced `lookup` command).

---

## Original Agent Reviews

### Gemini Findings (2025-10-03)

This section contains the original external review from Gemini, providing context for the improvement plan above.

#### Overall Impression & Core Strengths

First, the foundation is exceptionally strong.
1.  **Clear Vision:** The project knows exactly what it is: a fast, local-first, offline-capable search tool for `llms.txt`. This focus is a massive advantage.
2.  **Solid Tech Choices:** Rust + Tantivy for the core is perfect for performance and reliability. `clap` for the CLI provides a robust and extensible base.
3.  **Agent-Aware from Day One:** The extensive `AGENTS.md` files and CLI-specific guides show that you're already thinking about the agent as a primary user, which puts you ahead of 99% of CLI tools.
4.  **Excellent DX Foundation:** The use of `justfile`, comprehensive linting (`clippy`), formatting (`rustfmt`), and CI/CD automation is top-tier.

Now, let's explore the growth areas, framed from the agent's perspective.

#### 1. Agent Experience (AX) Enhancements: Closing the Loop

The core loop for an agent is: **Discover -> Acquire -> Search -> Retrieve**. `blz` is currently world-class at the middle two (Acquire/Search) and good at the last one (Retrieve). The biggest opportunity is in the first and last steps.

##### **A. The Discovery Problem: "Where is the `llms.txt`?"**

Right now, an agent must be *told* the URL for an `llms.txt` file. This is a manual, out-of-band step that breaks the automation chain.

**Suggestion: A `discover` command.**

```bash
# Agent wants to learn about TanStack Query
blz discover tanstack.com
```

**How it would work:**
1.  It receives a domain: `tanstack.com`.
2.  It intelligently checks for `llms.txt` files at common locations:
    *   `https://tanstack.com/llms.txt`
    *   `https://tanstack.com/llms-full.txt`
    *   `https://docs.tanstack.com/llms.txt` (and other common subdomains)
3.  It could even fetch the homepage (`https://tanstack.com`) and look for a `<link rel="llms" href="...">` tag in the `<head>`, which could become a best practice you champion.
4.  **Output for Agent:** It would return a JSON object with the discovered URL(s), ready to be piped into the `add` command.

```json
{
  "domain": "tanstack.com",
  "discovered": [
    {
      "url": "https://tanstack.com/query/latest/docs/llms-full.txt",
      "type": "full",
      "confidence": "high"
    }
  ]
}
```

This single feature would make `blz` dramatically more autonomous.

##### **B. The Onboarding Problem: "How do I get started?"**

An agent (or developer) setting up a new environment has to add sources one-by-one. The `blz-add-batch-manifest-spec.md` log shows you're already thinking about this, which is great. Let's expand on it.

**Suggestions:**
1.  **Remote Manifests:** Allow the `add --manifest` command to accept a URL. This enables curated, shareable lists.
    ```bash
    # Add a curated list of sources for a React project
    blz add --manifest https://gist.github.com/user/react-starter-pack.json
    ```
2.  **Starter Packs:** Create a concept of built-in "starter packs" that bundle common sources.
    ```bash
    # Interactively (for humans) or with --yes (for agents)
    blz init --pack=react-ecosystem
    ```
    This could add sources for React, Next.js, Tailwind, Zustand, etc., in one command. These packs could be defined in the `registry.json` file.

##### **C. The Maintenance Problem: "Are my docs stale?"**

Documentation rots. An agent relying on `blz` needs to trust its information is current.

**Suggestions:**
1.  **Smarter Updates:** A command to update only sources that haven't been checked in a while.
    ```bash
    blz update --all --stale-since=7d # Update sources not updated in 7 days
    ```
2.  **Background Service (Advanced):** For a truly best-in-class experience, `blz` could run an optional, lightweight background daemon.
    ```bash
    blz daemon start # Starts a background process
    blz daemon status # Checks status
    blz daemon stop  # Stops it
    ```
    This daemon would periodically run `blz update --all --stale-since=1d` in the background, ensuring the local cache is always fresh without agent intervention.

#### 2. Ergonomics & Best-in-Class CLI Features

These are features common in modern, best-in-class CLIs that would elevate the experience for both humans and agents.

*   **Interactive Modes for Humans:** While agents need non-interactive JSON, humans love interactive helpers.
    *   `blz search`: Without a query, could drop into an `fzf`-style fuzzy-finding UI.
    *   `blz add`: Without a URL, could present a list of known, popular sources from the registry for the user to select.
*   **Configuration Command:** Manually editing config files is a chore.
    ```bash
    blz config set default-limit 50
    blz config get editor
    ```
*   **Health Check Command:** A `blz doctor` or `blz status` command would be invaluable for debugging. It could check:
    *   Are all indices readable?
    *   Are any sources very stale?
    *   Is the config file valid?
    *   Disk space used by caches.
*   **Shell Integration:** You have completions, which is step 1.
    *   **Dynamic Completions:** The shell could autocomplete source aliases after `-s` or `--source`.
    *   **`blz cd`:** A helper to quickly `cd` into the directory where a source is cached for inspection. `eval "$(blz init -)"` could install this helper function.

#### 3. Output & Formatting for Agents

An agent's "eyes" are parsers. The more structured and metadata-rich the output, the better.

**Suggestions:**
1.  **JSONL (Newline-Delimited JSON):** For search results, offer a `--format jsonl` option. This is often easier for agents to stream and process line-by-line than a single large JSON array.
2.  **Richer Metadata in Search Results:** The agent needs signals beyond just the text. Every JSON object in a search result should be rich.
    ```json
    // Current is good, but could be better
    {
      "alias": "react",
      "score": 15.3,
      "lines": "500-510",
      "content": "..."
    }

    // Enriched for an agent
    {
      "alias": "react",
      "score": 15.3,
      "line_range": { "start": 500, "end": 510 },
      "content": "...",
      "source_url": "https://react.dev/llms-full.txt",
      "retrieved_at": "2025-10-03T10:00:00Z",
      "is_stale": false, // Based on a configurable TTL
      "document_title": "Hooks API Reference" // If available from parsing
    }
    ```
3.  **Reduce Round-Trips:** The `search` -> `get` loop is logical but requires two steps. Consider adding a `--context <lines>` flag to the `search` command itself. This would include the surrounding context directly in the search results, saving the agent a follow-up call for many use cases.

#### 4. Conceptual & Strategic Ideas

*   **`blz` as a Generic Local RAG Engine:** Right now, `blz` is for `llms.txt`. But the core engine (fetch, parse, index, search) is generic. You could position `blz` as a pluggable tool for building local knowledge bases from *any* text source (local markdown files, GitHub repos, etc.). This would dramatically expand the tool's TAM (Total Addressable Market).
*   **The "Alias" vs. "Source" Terminology:** I noticed the log file `alias-terminology-audit.md`. This is a minor but important point. "Alias" is what it is, but "Source" is what it represents. Commands like `blz source list` or `blz source remove` might be slightly more intuitive than `blz list` and `blz remove`. Since you've already audited this, I trust you've settled on what's best, but it's worth a final thought before a v1.0 release.

#### Summary of Top Recommendations

1.  **Implement `blz discover <domain>`:** This is the highest-leverage feature for agent autonomy.
2.  **Enrich JSON Output:** Add more metadata (`retrieved_at`, `is_stale`, `source_url`) to search results to give agents better signals.
3.  **Build Out Manifests/Starter Packs:** Make onboarding trivial with `blz add --manifest <url>` and `blz init --pack <name>`.
4.  **Add a Health/Status Command:** `blz doctor` for simple, robust self-diagnostics.
5.  **Introduce a `--context` flag to `search`:** Reduce agent round-trips by providing context directly in search results.

---

### Claude's Assessment (2025-10-03)

Having just implemented `blz validate` and `blz doctor`, and having reviewed the current codebase extensively, here's my honest assessment of Gemini's findings:

#### Strong Agreement (High Priority)

1. **`blz doctor` (Already Implemented!)** - This was in the top recommendations and we just completed it. The implementation includes:
   - Cache/config directory writability checks
   - Disk usage warnings
   - Source integrity validation
   - Search index verification
   - Auto-fix capability with `--fix`

   This validates the recommendation's value—it fills a real operational need.

2. **Enriched JSON Output** - Completely agree. The current search results are functional but minimal. Adding:
   - `retrieved_at` timestamp
   - `is_stale` boolean (based on configurable TTL)
   - `source_url` for context
   - `heading_path` from parsed structure

   This would be straightforward to implement and dramatically improve agent decision-making. **High value, low effort.**

3. **`--context` flag for search** - Brilliant suggestion. The current `search` → `get` pattern requires two commands. Adding `--context N` to search would:
   - Reduce API calls by 50% for common agent workflows
   - Be trivial to implement (leverage existing context logic from `get`)
   - Maintain backward compatibility

   **Should be in v1.0.0.**

4. **`--block` flag for search/get** - Building on the context idea, returning entire heading sections is even better:
   - Semantic completeness (whole concepts, not arbitrary ranges)
   - Infrastructure already exists (`HeadingBlock` data structure)
   - Optional `--max-lines` safety valve
   - Mutually exclusive with `--context` for simpler mental model

#### Good Ideas (Medium Priority)

5. **`blz discover <domain>`** - Clever concept, but I see challenges:
   - **Scope creep**: This shifts blz from "search tool" to "discovery tool"
   - **Reliability**: DNS/subdomain guessing is fragile
   - **Maintenance**: The `<link rel="llms">` tag doesn't exist as a standard

   **Alternative approach**: Enhance the existing `lookup` command (which queries the registry) to be smarter about fuzzy matching domains. This keeps discovery registry-driven rather than web-crawling.

6. **Remote Manifests** - Good idea but needs guardrails:
   - Security implications of fetching and executing remote configs
   - Need SHA-256 verification (like the install script)
   - Could be gated behind `--allow-remote` flag

   **Implementation note**: The batch manifest feature already exists via `blz add --manifest <path>`. Extending it to URLs is ~20 lines of code but needs careful security design.

7. **Starter Packs** - Love the UX, but:
   - Maintenance burden: Who curates these? How often updated?
   - **Better approach**: Create community-maintained manifests in the registry repo that users can reference by name:
     ```bash
     blz add --manifest registry:react-ecosystem
     # resolves to: https://raw.githubusercontent.com/outfitter-dev/blz-registry/main/packs/react-ecosystem.toml
     ```

   This keeps the CLI lean and delegates curation to the community.

#### Disagree or Lower Priority

8. **Background Daemon** - Strong disagree for v1.0:
   - **Complexity explosion**: Process management, IPC, startup scripts
   - **Platform divergence**: systemd vs launchd vs Windows services
   - **Over-engineering**: `cron` or user-level schedulers already solve this
   - **Philosophy clash**: blz is a tool, not a service

   **Simpler alternative**: Document a cron example in README:
   ```cron
   0 */6 * * * /usr/local/bin/blz update --all --quiet
   ```

9. **Interactive Modes (fzf integration)** - Nice-to-have, not need-to-have:
   - Adds dependency on external tools (fzf)
   - CLI already works great with pipes: `blz list | fzf`
   - Agent-first philosophy means non-interactive should be primary

   Keep this as a **future enhancement**, not v1.0 blocker.

10. **Config Command** - Currently exists via `blz alias add/rm` and direct TOML editing. Adding `blz config set` would be nice but:
    - Low urgency (manual editing works fine)
    - Config surface area is small
    - Could be post-v1.0

11. **JSONL Format** - Already implemented! Both `--format jsonl` and `--json` shortcuts exist for all commands.

#### Strategic Observations

**On "blz as Generic RAG Engine"**: Intriguing but wrong direction for v1.0. The focus on `llms.txt` is the product's strength, not a limitation. Once blz owns that niche, *then* consider expansion. Trying to be generic from day one dilutes the value proposition.

**On Terminology (Alias vs Source)**: The current model is:
- **Source**: The canonical identifier (e.g., `react`)
- **Alias**: Additional names pointing to a source (e.g., `reactjs` → `react`)

The confusion comes from overloading "alias" in `blz add <alias> <url>`. That first argument is actually the *source name*, not an alias. This is a CLI design flaw that should be addressed:

```bash
# Current (confusing)
blz add react https://react.dev/llms.txt

# Clearer
blz add --name react https://react.dev/llms.txt
# or
blz source add react https://react.dev/llms.txt
```

This would reserve `blz alias` exclusively for managing alternate names.

#### Final Thought

These findings demonstrate the agent actually *used* blz in anger, which is validating. The high-value suggestions (`doctor`, enriched JSON, `--context`, `--block`) align perfectly with the "agent-first" philosophy. The lower-priority items (daemon, interactive modes) show a human-UX bias creeping in—which is understandable but not the primary use case.

The fact that we *just* implemented `doctor` and `validate` (two of the top recommendations) suggests the project instincts are sound. Trust the current trajectory.
