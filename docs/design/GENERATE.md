# Design: Integrated Generation via `blz add`

Generate llms-full.txt documentation from various sources, integrated into the `add` command workflow.

## Overview

Many documentation sites lack `llms-full.txt` files. Rather than a separate `generate` command, generation is integrated into the existing `add` workflow:

1. **Auto-discovery**: `blz add <domain>` probes for llms-full.txt, llms.txt, sitemap.xml
2. **Prompt for generation**: If only llms.txt exists, prompt to generate
3. **Page-level caching**: Store individual scraped pages with durable IDs for incremental updates
4. **Smart refresh**: Check for upstream llms-full.txt, otherwise regenerate from cache

This approach keeps the user flow simple (one `add` command) while enabling powerful generation capabilities.

## Commands

### Add with generation

```bash
# Auto-discover best source (probes domain + docs.* subdomain)
blz add hono hono.dev

# Explicit llms.txt (prompts to generate)
blz add hono https://hono.dev/llms.txt

# Force generation from llms.txt index
blz add hono https://hono.dev/llms.txt --generate

# Skip generation prompt (use index-only)
blz add hono https://hono.dev/llms.txt --no-generate
```

### Add options

```
--generate           Force generation from llms.txt index
--no-generate        Skip generation, use index-only mode
--concurrency <N>    Parallel scrapes (default: 3, max: 10)
--limit <N>          Limit pages to scrape (for testing)
--dry-run            Preview manifest without generating
-y, --yes            Non-interactive mode
```

### Job management

```bash
blz job                    # List jobs (shorthand for 'job list')
blz job list               # List all jobs
blz job list --active      # Only running jobs
blz job <id>               # Show job details + progress
blz job <id> --cancel      # Cancel running job
blz job <id> --retry       # Retry failed URLs
blz job <id> --logs        # Show detailed log
```

### Refresh behavior

```bash
blz refresh hono              # Smart refresh (see workflow below)
blz refresh hono --force      # Re-scrape everything
blz refresh --all             # All sources
```

For generated sources, refresh:

1. Checks if upstream llms-full.txt now exists (upgrade path)
2. If not, re-fetches llms.txt for new pages
3. Scrapes only new/changed pages using cached page data
4. Re-assembles llms-full.txt from cache

## Auto-Discovery Workflow

When given a domain without explicit URL:

```
blz add hono hono.dev
```

Discovery cascade:

1. Probe `https://hono.dev/llms-full.txt` (direct use if found)
2. Probe `https://hono.dev/llms.txt` (offer generation)
3. Probe `https://docs.hono.dev/llms-full.txt` (auto-check docs subdomain)
4. Probe `https://docs.hono.dev/llms.txt`
5. Probe sitemap.xml variants (future support)

```
$ blz add hono hono.dev

Discovering documentation sources...

Found:
  - https://hono.dev/llms.txt (index, 47 pages)

No llms-full.txt available. Generate from llms.txt index?
This will scrape 47 pages using Firecrawl CLI.

[Y] Generate  [n] Use index-only  [c] Cancel
```

## Durable Page IDs

Each scraped page gets a durable ID based on its source URL:

```
pg_<hash>
```

Where `<hash>` is the first 12 characters of SHA256(url).

Example:
- URL: `https://hono.dev/docs/getting-started`
- ID: `pg_a1b2c3d4e5f6`

Benefits:
- **Stable citations**: Line ranges can reference page IDs that survive re-generation
- **Incremental updates**: Only scrape pages that changed
- **Cache efficiency**: Identify cached pages by ID

## Page-Level Caching

Scraped pages are stored individually for efficient incremental updates:

```
$BLZ_DATA_DIR/sources/<alias>/
├── llms-full.txt          # Assembled output
├── llms.json              # Line map with page IDs
├── manifest.json          # Source metadata
└── .cache/
    └── pages/
        ├── pg_a1b2c3d4e5f6.json
        ├── pg_b2c3d4e5f6a1.json
        └── ...
```

Each page JSON:

```json
{
  "id": "pg_a1b2c3d4e5f6",
  "url": "https://hono.dev/docs/getting-started",
  "title": "Getting Started",
  "section": "Guides",
  "description": "Quick start guide for Hono",
  "fetchedAt": "2026-01-26T14:30:15Z",
  "markdown": "# Getting Started\n\n...",
  "firecrawl": {
    "sourceURL": "...",
    "title": "...",
    "description": "..."
  }
}
```

## Assembly Output

The assembled llms-full.txt includes section markers and source attribution:

```markdown
<!-- Section: Guides -->
<!-- Source: https://hono.dev/docs/getting-started | Page: pg_a1b2c3d4e5f6 -->
# Getting Started

Content here...

<!-- Source: https://hono.dev/docs/routing | Page: pg_b2c3d4e5f6a1 -->
# Routing

More content...
```

Section markers are HTML comments to preserve heading hierarchy (no H1s that nest H2s).

## Line Map with Page IDs

The `llms.json` line map includes page IDs for citation stability:

```json
[
  {
    "id": "pg_a1b2c3d4e5f6",
    "title": "Getting Started",
    "url": "https://hono.dev/docs/getting-started",
    "section": "Guides",
    "lines": "3-245"
  }
]
```

## Source Metadata

Generated sources store metadata in `manifest.json`:

```json
{
  "alias": "hono",
  "type": "generated",
  "generatedAt": "2026-01-26T14:30:00Z",
  "generationMethod": "index-guided",
  "sourceUrl": "https://hono.dev/llms.txt",
  "firecrawlVersion": "1.1.1",
  "pages": [
    {
      "id": "pg_a1b2c3d4e5f6",
      "url": "https://hono.dev/docs/getting-started",
      "title": "Getting Started",
      "section": "Guides",
      "fetchedAt": "2026-01-26T14:30:15Z",
      "lines": "3-245",
      "lineCount": 243
    }
  ],
  "stats": {
    "totalPages": 47,
    "totalLines": 12450,
    "totalChars": 523000
  },
  "lastRefresh": null,
  "upstreamCheck": null
}
```

## Firecrawl CLI Integration

### Prerequisites

- Firecrawl CLI installed (`firecrawl --version`)
- Authenticated (`firecrawl login` or `FIRECRAWL_API_KEY`)
- Minimum version: 1.1.0

BLZ checks these on first use and provides helpful error messages if not met.

### Invocation

For each URL in manifest:

```bash
firecrawl scrape <url> \
  --format markdown \
  --only-main-content \
  --json \
  -o /tmp/blz-scrape-<job-id>/<page-id>.json
```

### Concurrency

- Default: 3 parallel scrapes
- Max: 10 (respects Firecrawl rate limits)
- User's plan limits apply

## Refresh Workflow

```
blz refresh hono
```

1. **Check for upstream llms-full.txt**
   - If source was generated from llms.txt, check if llms-full.txt now exists
   - If found, switch to direct source (no more generation needed)

2. **Re-fetch llms.txt index**
   - Parse for new/removed pages
   - Compare against cached page list

3. **Incremental scrape**
   - New pages: scrape and cache
   - Existing pages: check ETag/Last-Modified (future: Firecrawl changeTracking)
   - Removed pages: mark as stale in cache

4. **Re-assemble**
   - Concatenate all cached pages
   - Update line map and metadata

## Error Handling

### Firecrawl not installed

```
Error: Firecrawl CLI not found

Generation requires the Firecrawl CLI for web scraping.

Install: npm install -g firecrawl
Docs: https://docs.firecrawl.dev/cli

Use --no-generate to add the source without generation.
```

### Firecrawl not authenticated

```
Error: Firecrawl not authenticated

Run 'firecrawl login' to authenticate, or set FIRECRAWL_API_KEY.
```

### Partial failure

```
Warning: 3 of 47 pages failed to scrape

Completed: 44 pages
Failed:
  - https://hono.dev/docs/api/request (timeout)
  - https://hono.dev/docs/api/response (403)
  - https://hono.dev/docs/api/context (timeout)

The source has been indexed with available content.
Run 'blz job <id> --retry' to retry failed pages.
```

## Configuration

In `config.toml`:

```toml
[generate]
# Default concurrency for web scraping
concurrency = 3

# Auto-generate when only llms.txt found (skip prompt)
auto_generate = false

[generate.firecrawl]
# Minimum CLI version required
min_version = "1.1.0"
```

## Exit Codes

- `0` - Success
- `1` - Partial success (some URLs failed)
- `2` - Usage error
- `3` - Firecrawl not available (and --generate requested)
- `4` - Job cancelled
- `5` - All URLs failed

## Future Extensibility

### Additional source types

```rust
#[async_trait]
pub trait ContentGenerator {
    async fn generate(&self, config: &GenerateConfig) -> Result<GeneratedContent>;
    fn source_type(&self) -> &'static str;
}

// Current
struct FirecrawlGenerator;      // Web via Firecrawl CLI

// Future
struct GitHubGenerator;         // GitHub repos (clone + process markdown)
struct LocalGenerator;          // Local directories
struct SitemapGenerator;        // Direct sitemap parsing
```

### GitHub repo support (future)

```bash
blz add effect gh:Effect-TS/effect --path docs/ --generate
```

### Local directory support (future)

```bash
blz add mylib ./packages/mylib/docs --generate
```

## Security Considerations

- Firecrawl CLI handles authentication via user's existing login
- No API keys stored in BLZ
- Generated content is stored locally only
- Page cache respects source permissions
