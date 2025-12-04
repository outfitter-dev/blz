# Language Filtering Migration Guide

BLZ automatically filters non-English content from multilingual documentation sources, reducing storage by 60-90% while preserving all English documentation.

## What is Language Filtering?

Many popular documentation sites (like Anthropic, LangChain, Supabase) publish multilingual `llms.txt` files that include complete documentation in 10+ languages. While this is great for international users, it creates significant overhead:

- **Storage bloat**: Sources grow from 2-3MB to 15-30MB+ with duplicated content
- **Search noise**: Non-English results appear alongside English matches
- **Slower indexing**: Processing 10x more content takes proportionally longer

Language filtering automatically detects and removes non-English content during indexing, keeping only the English documentation you need.

## Default Behavior

As of BLZ v0.5.0, language filtering is **enabled by default** for all new sources:

```bash
# These commands automatically enable filtering
blz add anthropic https://docs.anthropic.com/llms-full.txt
blz add langchain https://python.langchain.com/llms-full.txt

# Result: Only English content is indexed
✓ Added anthropic (2,341 headings, 52,847 lines) in 1.2s
  ℹ Filtered 437,293 lines (89%) of non-English content
```

## Checking Filter Status

Use `blz info <alias>` to see if filtering is enabled for a source:

```bash
blz info anthropic --json | jq '.filters'
```

Example output:

```json
{
  "lang": {
    "enabled": true,
    "reason": "default",
    "linesFiltered": 437293,
    "percentFiltered": 89.2
  }
}
```

Or check all sources at once:

```bash
blz list --status --json | jq '.[] | {alias, filters}'
```

## Migrating Existing Sources

If you added sources before language filtering was available, they contain all languages. Here's how to migrate them:

### Single Source Migration

Refresh with the `--reindex` and `--filter` flags:

```bash
blz refresh anthropic --reindex --filter
```

This will:
1. Re-download the latest `llms.txt` file (respects ETags, only downloads if changed)
2. Apply language filtering during parsing
3. Rebuild the search index with only English content

**Before:**
```
anthropic: 589,140 lines indexed (30.4 MB)
```

**After:**
```
anthropic: 52,847 lines indexed (2.8 MB)
  ℹ Filtered 437,293 lines (89%) of non-English content
```

### Batch Migration

To migrate multiple sources at once:

```bash
# Migrate all sources
blz refresh --all --reindex --filter

# Or migrate specific sources
blz refresh anthropic langchain supabase --reindex --filter
```

### Expected Results

Here are typical space savings for common multilingual sources:

| Source | Before | After | Savings |
|--------|--------|-------|---------|
| Anthropic | 30.4 MB | 2.8 MB | 89% |
| LangChain | 28.7 MB | 3.1 MB | 87% |
| Supabase | 18.2 MB | 2.4 MB | 85% |

Sources that are already English-only (like Bun, Next.js, React) won't see significant changes.

## Disabling Filtering

If you need multilingual documentation, you can disable filtering:

### For New Sources

```bash
blz add anthropic https://docs.anthropic.com/llms-full.txt --no-language-filter
```

### For Existing Sources

```bash
# Re-download and reindex without filtering
blz refresh anthropic --reindex --no-filter
```

### Permanently Disable (Advanced)

To disable filtering for all sources by default, set the environment variable:

```bash
export BLZ_LANGUAGE_FILTER=false
```

Or configure it per-source in `<data_dir>/sources/<alias>/settings.toml`:

> **Tip:** `<data_dir>` defaults to `~/.blz` when `XDG_DATA_HOME` is unset. If you have `XDG_DATA_HOME` configured, use `$XDG_DATA_HOME/blz` instead.

```toml
[filters]
language = false
```

## How It Works

Language filtering uses a combination of techniques to identify non-English content:

1. **Unicode Script Detection**: Identifies CJK characters (Chinese, Japanese, Korean), Arabic, Cyrillic, etc.
2. **URL Path Analysis**: Detects language-specific URL segments (`/ja/`, `/zh-CN/`, `/es/`, etc.)
3. **Common Word Patterns**: Recognizes non-English function words and articles

The filter runs during parsing (before indexing) and:
- Removes entire sections written in non-English languages
- Preserves code examples (even if they contain non-ASCII characters)
- Preserves English headings that link to multilingual content
- Never modifies the original `llms.txt` file

## Troubleshooting

### "I'm still seeing non-English results"

1. Verify filtering is enabled:
   ```bash
   blz info <alias> --json | jq '.filters.lang.enabled'
   ```

2. If it shows `false`, re-enable with:
   ```bash
   blz refresh <alias> --reindex --filter
   ```

### "I need one non-English language"

Language filtering is currently all-or-nothing. If you need specific non-English languages:

1. Disable filtering: `blz refresh <alias> --reindex --no-filter`
2. Search will return all languages; filter results client-side

Future versions may support per-language filtering.

### "The filter removed too much"

If legitimate English content was filtered (false positive):

1. Please [report an issue](https://github.com/outfitter-dev/blz/issues) with:
   - The source alias and URL
   - The specific content that was incorrectly filtered
   - The section heading or line range

2. As a workaround, disable filtering for that source:
   ```bash
   blz refresh <alias> --reindex --no-filter
   ```

## Migration Checklist

- [ ] Check which sources have multilingual content: `blz list --status --json`
- [ ] Identify sources with large storage footprints (>10MB)
- [ ] Run migration: `blz refresh <alias> --reindex --filter`
- [ ] Verify results: `blz info <alias>` to see lines filtered
- [ ] Test searches to confirm quality hasn't degraded
- [ ] Repeat for remaining multilingual sources

## See Also

- [Configuration Guide](../cli/configuration.md) - Per-source settings and environment variables
- [Command Reference](../cli/commands.md) - Complete `refresh` and `add` flag documentation
- [Performance](../architecture/PERFORMANCE.md) - Impact of filtering on indexing speed
