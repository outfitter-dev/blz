# Generate Feature Implementation Plan

> Revised after pathfinding session (2026-01-28)

## Executive Summary

Enable BLZ to generate `llms-full.txt` documentation from web sources when sites don't provide one natively. The feature is integrated into `blz add` for zero-friction onboarding while using Firecrawl CLI for battle-tested scraping.

**Core Goals:**
1. **Zero-friction onboarding**: `blz add hono.dev` → searchable docs
2. **Coverage expansion**: BLZ works for sites without native llms-full.txt
3. **Quality parity**: Generated content matches native search quality
4. **Self-healing**: System upgrades automatically when native support appears

**Estimated Scope:** 13-15 stacked PRs across 3-4 weeks

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                  blz-cli                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                            AddCommand                                    │ │
│  │  blz add hono.dev                                                        │ │
│  │    ↓                                                                     │ │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐  │ │
│  │  │  Discovery  │ →  │   Prompt    │ →  │    GenerateOrchestrator     │  │ │
│  │  │  - probe    │    │  - alias    │    │    - parallel scraping      │  │ │
│  │  │  - sitemap  │    │  - generate │    │    - progress display       │  │ │
│  │  └─────────────┘    └─────────────┘    │    - partial failure        │  │ │
│  │                                         └─────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │   SyncCommand   │  │  DoctorCommand  │  │      (Phase 2) GenCmd       │  │
│  │  - auto-retry   │  │  - health check │  │      - status, rebuild      │  │
│  │  - refresh      │  │  - upgrade hint │  │      - explicit retry       │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                                  blz-core                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │  FirecrawlCli   │  │   Discovery     │  │       PageCache             │  │
│  │  - detect()     │  │  - probe_url()  │  │  - save_page()              │  │
│  │  - version()    │  │  - parse_index()│  │  - load_page()              │  │
│  │  - scrape()     │  │  - sitemap()    │  │  - backup_to()              │  │
│  │  - is_auth()    │  │  - derive_alias │  │  - list_failed()            │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
│                                                                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │   Assembler     │  │  URLExtractor   │  │     GenerateManifest        │  │
│  │  - assemble()   │  │  - md_links()   │  │  - schemaVersion: "1.0.0"   │  │
│  │  - line_map()   │  │  - bare_urls()  │  │  - pages, stats, failures   │  │
│  │  - sections()   │  │  - sitemap()    │  │  - backup metadata          │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## User Flows

### Flow 1: Add with Generation

```
$ blz add hono.dev

Probing hono.dev...
  ✗ https://hono.dev/llms-full.txt (404)
  ✓ https://hono.dev/llms.txt (found, 47 URLs)
  ✓ https://hono.dev/sitemap.xml (found, +12 URLs)

Alias: hono  ← (press Enter to accept, or type new name)
> hono
✓ Alias 'hono' available

No llms-full.txt available. Generate from 59 discovered URLs?
[Y] Generate  [n] Index-only  [c] Cancel
> Y

Generating hono...
[████████████████████████████░░░] 56/59

✓ Generated 'hono' (56/59 pages, 12,450 lines)

⚠ 3 pages failed:
  • /docs/api/request   (timeout)
  • /docs/api/response  (403)
  • /docs/api/context   (timeout)

These will be retried on next sync.
```

### Flow 2: Sync with Auto-Retry

```
$ blz sync hono

Syncing hono (generated)...
  Checking for new pages in llms.txt... +2 new
  Retrying 3 failed pages...

[████████████████████████████████] 5/5

✓ hono updated (58/61 pages, 12,890 lines)

⚠ 1 page still failing:
  • /docs/api/response  (403 - may require auth)
```

### Flow 3: Doctor with Upgrade Detection

```
$ blz doctor

Sources:
  ✓ react       native     15,230 lines   fresh
  ✓ bun         native      8,120 lines   fresh
  ⚠ hono        generated  12,890 lines   1 failed page
                → Native llms-full.txt now available!
                  Run 'blz sync hono --upgrade' to switch.
  ✓ effect      generated   9,450 lines   healthy

Recommendations:
  • Upgrade 'hono' to native source (better quality, less maintenance)
  • Retry or skip failed page in 'hono': /docs/api/response
```

### Flow 4: Upgrade to Native

```
$ blz sync hono --upgrade

Upgrading hono to native llms-full.txt...
  ✓ Fetched native content (14,200 lines)

Generated cache backed up to .cache/pages.bak/
  58 pages preserved (index: .cache/pages.bak/index.json)

Delete backup? [y/N]: n
Keeping backup. Run 'blz cache clean hono' to remove later.

✓ hono upgraded to native (14,200 lines)
```

---

## Data Structures

### GenerateManifest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateManifest {
    /// Schema version for future migrations
    pub schema_version: String,  // "1.0.0"

    /// Source type
    #[serde(rename = "type")]
    pub source_type: SourceType,  // Generated | Native

    /// When generation completed
    pub generated_at: DateTime<Utc>,

    /// Discovery method used
    pub discovery: DiscoveryInfo,

    /// Individual page metadata
    pub pages: Vec<PageMeta>,

    /// Pages that failed and need retry
    pub failed_pages: Vec<FailedPage>,

    /// Aggregate statistics
    pub stats: GenerateStats,

    /// Firecrawl CLI version used
    pub firecrawl_version: String,

    /// Backup info if upgraded from generated
    pub backup: Option<BackupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryInfo {
    /// Original input (e.g., "hono.dev")
    pub input: String,
    /// Resolved llms.txt URL
    pub index_url: Option<String>,
    /// Resolved sitemap URL
    pub sitemap_url: Option<String>,
    /// URLs found from each source
    pub url_sources: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedPage {
    pub url: String,
    pub error: String,
    pub attempts: u32,
    pub last_attempt: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub backed_up_at: DateTime<Utc>,
    pub reason: String,  // "upgraded_to_native"
    pub page_count: usize,
    pub path: String,  // ".cache/pages.bak"
}
```

### PageCacheEntry

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCacheEntry {
    /// Durable page ID: pg_<sha256_prefix_12>
    pub id: String,

    /// Source URL
    pub url: String,

    /// Extracted or Firecrawl-provided title
    pub title: Option<String>,

    /// Section from llms.txt structure
    pub section: Option<String>,

    /// When fetched
    pub fetched_at: DateTime<Utc>,

    /// Markdown content
    pub markdown: String,

    /// Line count (for stats)
    pub line_count: usize,
}
```

---

## MVP Scope

### In Scope (Phase 1)

| Component | Description |
|-----------|-------------|
| **Discovery** | Probe domain for llms-full.txt, llms.txt, sitemap.xml |
| **Alias derivation** | Strip docs/api/www, base domain, editable prompt, collision detection |
| **URL extraction** | Markdown links + bare URLs + sitemap, filter to domain/subdomains |
| **Firecrawl wrapper** | Detect, version check, auth check, scrape |
| **Page caching** | Durable IDs, save/load/list, backup on upgrade |
| **Assembly** | Concatenate pages, section markers, line map with page IDs |
| **Progress display** | Progress bar, failure reporting |
| **Partial failure** | Complete with gaps, store failures for retry |
| **Auto-retry** | Sync retries failed pages automatically |
| **Upgrade detection** | Doctor checks for native llms-full.txt |
| **Schema versioning** | manifest.json has schemaVersion: "1.0.0" |

### Out of Scope (Phase 2+)

| Component | Rationale |
|-----------|-----------|
| `blz gen` namespace | Management commands - MVP uses sync + doctor |
| Background jobs | Synchronous with progress is sufficient for MVP |
| ContentGenerator trait | Abstract when we have 2+ generators |
| GitHub repo generator | Web-first, GitHub is separate use case |
| Local directory generator | Ditto |
| Incremental change detection | Full re-scrape is acceptable for MVP |
| Firecrawl changeTracking | Nice optimization, not required |

---

## Implementation Phases

### Phase 1: Firecrawl Foundation (BLZ-344)

**PR 1.1: Firecrawl CLI detection** (~80 LOC)
```
crates/blz-core/src/firecrawl/mod.rs
crates/blz-core/src/firecrawl/detect.rs
```
- Detect Firecrawl in PATH
- Parse version, check >= 1.1.0
- Check authentication status
- Helpful error messages

**PR 1.2: Firecrawl scrape** (~120 LOC)
```
crates/blz-core/src/firecrawl/scrape.rs
```
- `scrape()` method invoking CLI
- Parse JSON response
- Handle timeouts, errors

**PR 1.3: Error types** (~60 LOC)
```
crates/blz-core/src/error.rs (extend)
```
- `FirecrawlNotInstalled`
- `FirecrawlVersionTooOld`
- `FirecrawlNotAuthenticated`
- `ScrapeError { url, reason }`

### Phase 2: Discovery (NEW - was Phase 2, now MVP)

**PR 2.1: URL probing** (~100 LOC)
```
crates/blz-core/src/discovery/probe.rs
```
- Probe URLs: /, /llms-full.txt, /llms.txt, /sitemap.xml
- Handle redirects, 404s gracefully
- Support docs.* subdomain probing

**PR 2.2: Sitemap parsing with lastmod** (~120 LOC)
```
crates/blz-core/src/discovery/sitemap.rs
```
- Parse sitemap.xml (we do this ourselves, not Firecrawl)
- Extract `<lastmod>` dates for each URL (critical for FREE change detection)
- Filter to domain/subdomains
- Handle sitemap index files (recursive parsing)
- Return `SitemapEntry { url, lastmod, changefreq, priority }`

**PR 2.3: Alias derivation** (~100 LOC)
```
crates/blz-core/src/discovery/alias.rs
```
- Strip common prefixes (docs, api, www)
- Extract base domain
- Validate against existing sources (collision detection)

**PR 2.4: URL extraction and filtering** (~160 LOC)
```
crates/blz-core/src/discovery/extract.rs
crates/blz-core/src/discovery/filter.rs
```
- Parse markdown links `[text](url)`
- Find bare URLs
- Filter to domain/subdomains
- Deduplicate with sitemap URLs
- **Doc path heuristics**: `is_likely_docs_path()` to exclude /blog/, /careers/, /pricing/, etc.
- Merge URLs from llms.txt + sitemap, preserving lastmod from sitemap

### Phase 3: Page Caching

**PR 3.1: Page cache types** (~100 LOC)
```
crates/blz-core/src/page_cache/types.rs
```
- `PageCacheEntry` struct with `sitemap_lastmod: Option<DateTime<Utc>>`
- Durable ID generation: `pg_<sha256_12>`
- Serialization tests
- Track both `fetched_at` (when we scraped) and `sitemap_lastmod` (from sitemap)

**PR 3.2: Page cache storage** (~120 LOC)
```
crates/blz-core/src/page_cache/storage.rs
```
- Save/load/list pages
- Backup to .bak with index
- Failed pages tracking

### Phase 4: Assembly

**PR 4.1: Manifest types** (~100 LOC)
```
crates/blz-core/src/generate/manifest.rs
```
- `GenerateManifest` with schemaVersion
- `DiscoveryInfo`, `FailedPage`, `BackupInfo`

**PR 4.2: Content assembler** (~150 LOC)
```
crates/blz-core/src/generate/assembler.rs
```
- Concatenate pages with section markers
- Build line map with page IDs
- Generate stats

### Phase 5: CLI Integration

**PR 5.1: Add command discovery** (~150 LOC)
```
crates/blz-cli/src/commands/add.rs (extend)
crates/blz-cli/src/prompt/alias.rs
```
- Domain-only input parsing
- Discovery cascade
- Alias prompt with validation

**PR 5.2: Generate orchestrator** (~200 LOC)
```
crates/blz-cli/src/generate/orchestrator.rs
```
- Parallel scraping with semaphore
- Progress bar display
- Partial failure handling

**PR 5.3: Add command generate flow** (~150 LOC)
```
crates/blz-cli/src/commands/add.rs (extend)
```
- Generate prompt
- Orchestrator integration
- Success/failure summary

**PR 5.4: Sync with lastmod optimization** (~150 LOC)
```
crates/blz-cli/src/commands/sync.rs (extend)
```
- Detect generated sources
- Re-fetch sitemap.xml, compare lastmod dates
- **Skip unchanged pages** (where sitemap lastmod <= cached lastmod) - FREE!
- Only scrape new/changed pages (massive credit savings)
- Retry failed pages from previous run
- Re-assemble on success

**PR 5.5: Doctor generation health** (~120 LOC)
```
crates/blz-cli/src/commands/doctor.rs (extend)
```
- Show generated source status
- Failed page warnings
- Upgrade availability check

---

## Testing Strategy

### Unit Tests (Mock Firecrawl)

```rust
// firecrawl/detect.rs
#[test]
fn detects_firecrawl_in_path() { ... }

#[test]
fn rejects_old_version() { ... }

// discovery/alias.rs
#[test]
fn strips_common_prefixes() {
    assert_eq!(derive_alias("docs.hono.dev"), "hono");
    assert_eq!(derive_alias("api.stripe.com"), "stripe");
}

#[test]
fn detects_collision() { ... }

// discovery/extract.rs
#[test]
fn extracts_markdown_links() { ... }

#[test]
fn filters_to_domain() { ... }
```

### Integration Tests (Feature-Gated)

```rust
#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_real_firecrawl_scrape() {
    let cli = FirecrawlCli::detect().expect("Firecrawl required");
    let result = cli.scrape("https://example.com").await;
    assert!(result.is_ok());
}
```

### E2E Tests

```bash
# Dry-run generation
blz add --dry-run hono.dev

# Full generation (requires Firecrawl auth)
BLZ_TEST_FIRECRAWL=1 blz add hono.dev --yes
```

---

## Linear Issue Mapping

| Issue | PRs | Status |
|-------|-----|--------|
| BLZ-344 | PR 1.1, 1.2, 1.3 | Firecrawl wrapper |
| BLZ-375 | PR 2.1, 2.2, 2.3, 2.4 | Discovery ✓ Created |
| BLZ-345 | PR 3.1, 3.2, 4.1, 4.2, 5.1-5.5 | Generation + CLI |
| BLZ-349 | Phase 2 | Tiered sync (moved to Backlog) |
| BLZ-342 | All | Parent epic |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Firecrawl CLI changes | Medium | High | Pin min version, integration tests |
| Rate limiting | High | Medium | Configurable concurrency, auto-retry |
| Sitemap format variations | Medium | Low | Graceful fallback to llms.txt only |
| Large sites (500+ pages) | Low | Medium | Progress indication, --limit flag |
| llms.txt format variations | High | Low | Multi-strategy extraction |

---

## Resolved Design Decisions

### Section Marker Syntax

Follow **Bun's llms-full.txt pattern** (cleanest, most widely adopted):

```markdown
# Getting Started
Source: https://hono.dev/docs/getting-started

Content here...

# Installation
Source: https://hono.dev/docs/installation

More content...
```

Optionally include page ID as HTML comment for traceability:
```markdown
# Getting Started
Source: https://hono.dev/docs/getting-started
<!-- Page: pg_a1b2c3d4e5f6 -->
```

### Adaptive Concurrency

Start high and back off based on failures:

```
Initial: 5 parallel requests
  ↓
On rate limit (429) or timeout cluster (3+ in window):
  ↓
Back off: 5 → 3 → 2 → 1
  ↓
Track success rate at each level
  ↓
Find stable level, use as session default
  ↓
Store learned default in source config for next run
```

User overrides via `--concurrency <N>` (max 10).

Config option:
```toml
[generate]
concurrency = 5          # starting point
adaptive = true          # enable backoff
min_concurrency = 1      # floor
```

### Cost Optimization: Sitemap Lastmod Strategy

**Key insight:** We parse sitemaps ourselves (not via Firecrawl) to get `<lastmod>` dates for FREE change detection.

**Sync flow:**
```
1. Fetch sitemap.xml (FREE - direct HTTP)
2. Parse <lastmod> for each URL
3. Compare against cached page's sitemap_lastmod
4. Skip pages where lastmod unchanged (FREE!)
5. Only scrape new/changed pages via Firecrawl (costs credits)
```

**Cost comparison (200-page site, 10 changed):**

| Approach | Credits |
|----------|---------|
| Full re-scrape | 200 |
| With sitemap lastmod | ~10-20 |
| Savings | **90-95%** |

**Fallback:** If sitemap has no `<lastmod>`, fall back to re-scraping (or Phase 2: HEAD request ETag).

**Firecrawl CLI flags to use:**
- `--only-main-content` for cleaner output
- `-f markdown` for the format we need
- Adaptive concurrency (start 5, back off on rate limits)

### Cache TTL

For MVP: No automatic expiration. Pages are considered fresh until:
- Sitemap `<lastmod>` changes
- User runs `blz sync --force`
- llms.txt shows new/removed URLs
- `blz doctor` recommends refresh

Phase 2 consideration: Add `max_age` config for automatic staleness.

---

## Next Steps

1. ~~Create Linear issue for Discovery~~ → BLZ-375 created
2. Update BLZ-375 with sitemap lastmod details
3. Begin PR 1.1 (Firecrawl CLI detection)
