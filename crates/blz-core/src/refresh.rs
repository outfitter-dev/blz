//! Refresh helpers shared by CLI and MCP consumers.

use std::path::PathBuf;

use crate::{
    FetchResult, Fetcher, HeadingFilterStats, LanguageFilter, MarkdownParser, ParseResult,
    PerformanceMetrics, Result, SearchIndex, Source, SourceType, Storage, TocEntry,
};

use crate::json_builder::build_llms_json;
use crate::url_resolver::resolve_best_url;

/// Abstraction over storage interactions used by refresh routines.
pub trait RefreshStorage {
    /// Load stored metadata for a source alias.
    fn load_metadata(&self, alias: &str) -> Result<Source>;
    /// Load alias list from the cached llms.json for a source.
    fn load_llms_aliases(&self, alias: &str) -> Result<Vec<String>>;
    /// Persist the latest llms.txt content.
    fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()>;
    /// Persist the computed llms.json metadata payload.
    fn save_llms_json(&self, alias: &str, data: &crate::LlmsJson) -> Result<()>;
    /// Persist updated source metadata.
    fn save_metadata(&self, alias: &str, metadata: &Source) -> Result<()>;
    /// Resolve the on-disk index path for a source.
    fn index_path(&self, alias: &str) -> Result<PathBuf>;
    /// Load cached llms.txt content for a source.
    fn load_llms_txt(&self, alias: &str) -> Result<String>;
}

impl RefreshStorage for Storage {
    fn load_metadata(&self, alias: &str) -> Result<Source> {
        Self::load_source_metadata(self, alias)?
            .ok_or_else(|| crate::Error::NotFound(format!("Missing metadata for {alias}")))
    }

    fn load_llms_aliases(&self, alias: &str) -> Result<Vec<String>> {
        match Self::load_llms_json(self, alias) {
            Ok(llms) => Ok(llms.metadata.aliases),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()> {
        Self::save_llms_txt(self, alias, content)
    }

    fn save_llms_json(&self, alias: &str, data: &crate::LlmsJson) -> Result<()> {
        Self::save_llms_json(self, alias, data)
    }

    fn save_metadata(&self, alias: &str, metadata: &Source) -> Result<()> {
        Self::save_source_metadata(self, alias, metadata)
    }

    fn index_path(&self, alias: &str) -> Result<PathBuf> {
        Self::index_dir(self, alias)
    }

    fn load_llms_txt(&self, alias: &str) -> Result<String> {
        Self::load_llms_txt(self, alias)
    }
}

/// Interface for indexing refreshed content.
pub trait RefreshIndexer {
    /// Index a set of heading blocks for the given alias.
    fn index(
        &self,
        alias: &str,
        index_path: &std::path::Path,
        metrics: PerformanceMetrics,
        blocks: &[crate::HeadingBlock],
    ) -> Result<()>;
}

/// Default indexer that writes to the Tantivy search index.
#[derive(Default)]
pub struct DefaultRefreshIndexer;

impl RefreshIndexer for DefaultRefreshIndexer {
    fn index(
        &self,
        alias: &str,
        index_path: &std::path::Path,
        metrics: PerformanceMetrics,
        blocks: &[crate::HeadingBlock],
    ) -> Result<()> {
        let index = SearchIndex::create_or_open(index_path)?.with_metrics(metrics);
        index.index_blocks(alias, blocks)
    }
}

/// Result summary for a refresh operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// The source content changed and was re-indexed.
    Refreshed {
        /// Canonical alias for the refreshed source.
        alias: String,
        /// Number of headings indexed.
        headings: usize,
        /// Total line count in the source.
        lines: usize,
    },
    /// The source content was unchanged.
    Unchanged {
        /// Canonical alias for the unchanged source.
        alias: String,
    },
}

/// Result summary for a reindex operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexOutcome {
    /// Canonical alias for the reindexed source.
    pub alias: String,
    /// Heading count before filtering.
    pub headings_before: usize,
    /// Heading count after filtering.
    pub headings_after: usize,
    /// Number of headings filtered out.
    pub filtered: usize,
}

/// Data describing remote changes.
#[derive(Debug, Clone)]
pub struct RefreshPayload {
    /// Refreshed llms.txt content.
    pub content: String,
    /// SHA256 hash of the refreshed content.
    pub sha256: String,
    /// `ETag` header value from the response.
    pub etag: Option<String>,
    /// Last-Modified header value from the response.
    pub last_modified: Option<String>,
}

/// URL resolution details for refresh operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshUrlResolution {
    /// Final URL used for refresh.
    pub final_url: String,
    /// Variant chosen for the URL.
    pub variant: crate::SourceVariant,
    /// Whether the source was upgraded to llms-full.txt.
    pub upgraded: bool,
}

/// Context for refresh operations with preloaded metadata.
///
/// This struct bundles the context needed for `refresh_source_with_metadata`,
/// reducing parameter counts while maintaining clear ownership.
#[derive(Debug, Clone)]
pub struct RefreshContext {
    /// Existing source metadata.
    pub existing_metadata: Source,
    /// Existing aliases from llms.json.
    pub existing_aliases: Vec<String>,
    /// Resolved URL for refresh.
    pub resolution: RefreshUrlResolution,
}

impl RefreshContext {
    /// Create a new refresh context.
    #[must_use]
    pub const fn new(
        existing_metadata: Source,
        existing_aliases: Vec<String>,
        resolution: RefreshUrlResolution,
    ) -> Self {
        Self {
            existing_metadata,
            existing_aliases,
            resolution,
        }
    }
}

/// Resolve the best refresh URL (llms.txt → llms-full.txt) when available.
pub async fn resolve_refresh_url(
    fetcher: &Fetcher,
    metadata: &Source,
) -> Result<RefreshUrlResolution> {
    if metadata.variant != crate::SourceVariant::Llms {
        return Ok(RefreshUrlResolution {
            final_url: metadata.url.clone(),
            variant: metadata.variant.clone(),
            upgraded: false,
        });
    }

    match resolve_best_url(fetcher, &metadata.url).await {
        Ok(resolved) if resolved.variant == crate::SourceVariant::LlmsFull => {
            Ok(RefreshUrlResolution {
                final_url: resolved.final_url,
                variant: resolved.variant,
                upgraded: true,
            })
        },
        Ok(_) | Err(_) => Ok(RefreshUrlResolution {
            final_url: metadata.url.clone(),
            variant: metadata.variant.clone(),
            upgraded: false,
        }),
    }
}

/// Refresh a source using its current metadata.
///
/// # Errors
///
/// Returns an error if storage access, fetching, or indexing fails.
pub async fn refresh_source<S, I>(
    storage: &S,
    fetcher: &Fetcher,
    alias: &str,
    metrics: PerformanceMetrics,
    indexer: &I,
    filter_preference: bool,
) -> Result<RefreshOutcome>
where
    S: RefreshStorage + Sync,
    I: RefreshIndexer + Sync,
{
    let existing_metadata = storage.load_metadata(alias)?;
    let existing_aliases = storage.load_llms_aliases(alias)?;
    let resolution = resolve_refresh_url(fetcher, &existing_metadata).await?;

    let ctx = RefreshContext::new(existing_metadata, existing_aliases, resolution);

    refresh_source_with_metadata(
        storage,
        fetcher,
        alias,
        &ctx,
        metrics,
        indexer,
        filter_preference,
    )
    .await
}

/// Refresh a source using preloaded metadata and URL resolution.
///
/// # Errors
///
/// Returns an error if fetching, parsing, or indexing fails.
pub async fn refresh_source_with_metadata<S, I>(
    storage: &S,
    fetcher: &Fetcher,
    alias: &str,
    ctx: &RefreshContext,
    metrics: PerformanceMetrics,
    indexer: &I,
    filter_preference: bool,
) -> Result<RefreshOutcome>
where
    S: RefreshStorage + Sync,
    I: RefreshIndexer + Sync,
{
    let fetch_result = fetcher
        .fetch_with_cache(
            &ctx.resolution.final_url,
            ctx.existing_metadata.etag.as_deref(),
            ctx.existing_metadata.last_modified.as_deref(),
        )
        .await?;

    match fetch_result {
        FetchResult::NotModified { .. } => {
            if ctx.existing_metadata.filter_non_english.unwrap_or(true) != filter_preference {
                let mut updated_metadata = ctx.existing_metadata.clone();
                updated_metadata.filter_non_english = Some(filter_preference);
                storage.save_metadata(alias, &updated_metadata)?;
            }
            Ok(RefreshOutcome::Unchanged {
                alias: alias.to_string(),
            })
        },
        FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => {
            let payload = RefreshPayload {
                content,
                sha256,
                etag,
                last_modified,
            };

            let mut updated_metadata = ctx.existing_metadata.clone();
            updated_metadata.url.clone_from(&ctx.resolution.final_url);
            updated_metadata.variant = ctx.resolution.variant.clone();
            updated_metadata.filter_non_english = Some(filter_preference);

            let apply_params =
                ApplyRefreshParams::new(updated_metadata, ctx.existing_aliases.clone());
            apply_refresh(storage, alias, &apply_params, &payload, metrics, indexer)
        },
    }
}

/// Re-parse and re-index a source using cached content.
///
/// # Errors
///
/// Returns an error if cached content cannot be parsed or indexed.
pub fn reindex_source<S, I>(
    storage: &S,
    alias: &str,
    metrics: PerformanceMetrics,
    indexer: &I,
    filter_preference: bool,
) -> Result<ReindexOutcome>
where
    S: RefreshStorage,
    I: RefreshIndexer,
{
    let content = storage.load_llms_txt(alias)?;
    let mut parser = MarkdownParser::new()?;
    let mut parse_result = parser.parse(&content)?;

    let before_count = parse_result.heading_blocks.len();
    apply_language_filter(&mut parse_result, filter_preference);
    let after_count = parse_result.heading_blocks.len();

    let index_path = storage.index_path(alias)?;
    indexer.index(
        alias,
        index_path.as_path(),
        metrics,
        &parse_result.heading_blocks,
    )?;

    Ok(ReindexOutcome {
        alias: alias.to_string(),
        headings_before: before_count,
        headings_after: after_count,
        filtered: before_count.saturating_sub(after_count),
    })
}

/// Merge aliases from existing metadata with any already-known aliases.
fn merge_aliases(existing_aliases: Vec<String>, metadata_aliases: &[String]) -> Vec<String> {
    let mut merged = existing_aliases;
    for alias_value in metadata_aliases {
        if !merged.contains(alias_value) {
            merged.push(alias_value.clone());
        }
    }
    merged.sort();
    merged.dedup();
    merged
}

/// Copy preserved fields from existing metadata into the new `llms_json`.
fn copy_preserved_metadata_fields(llms_json: &mut crate::LlmsJson, existing: &Source) {
    llms_json.metadata.tags.clone_from(&existing.tags);
    llms_json
        .metadata
        .description
        .clone_from(&existing.description);
    llms_json.metadata.category.clone_from(&existing.category);
    llms_json
        .metadata
        .npm_aliases
        .clone_from(&existing.npm_aliases);
    llms_json
        .metadata
        .github_aliases
        .clone_from(&existing.github_aliases);
    llms_json.metadata.variant = existing.variant.clone();
}

/// Resolve the source origin based on existing metadata.
fn resolve_origin(existing: &Source) -> crate::SourceOrigin {
    let mut origin = existing.origin.clone();
    origin.source_type = match (&origin.source_type, &existing.origin.source_type) {
        (Some(SourceType::Remote { .. }), _) | (None, None) => Some(SourceType::Remote {
            url: existing.url.clone(),
        }),
        (Some(SourceType::LocalFile { path }), _) => {
            Some(SourceType::LocalFile { path: path.clone() })
        },
        (None, Some(existing_type)) => Some(existing_type.clone()),
    };
    origin
}

/// Build the updated Source metadata for a refresh.
fn build_refresh_metadata(
    existing: Source,
    payload: &RefreshPayload,
    origin: crate::SourceOrigin,
) -> Source {
    Source {
        url: existing.url,
        etag: payload.etag.clone(),
        last_modified: payload.last_modified.clone(),
        fetched_at: chrono::Utc::now(),
        sha256: payload.sha256.clone(),
        variant: existing.variant,
        aliases: existing.aliases,
        tags: existing.tags,
        description: existing.description,
        category: existing.category,
        npm_aliases: existing.npm_aliases,
        github_aliases: existing.github_aliases,
        origin,
        filter_non_english: existing.filter_non_english,
    }
}

/// Parameters for applying a refresh operation.
#[derive(Debug, Clone)]
pub struct ApplyRefreshParams {
    /// Updated source metadata (with new URL, variant, filter settings).
    pub metadata: Source,
    /// Existing aliases from the source.
    pub existing_aliases: Vec<String>,
}

impl ApplyRefreshParams {
    /// Create new apply refresh parameters.
    #[must_use]
    pub const fn new(metadata: Source, existing_aliases: Vec<String>) -> Self {
        Self {
            metadata,
            existing_aliases,
        }
    }
}

/// Apply a refresh: persist content and re-index the source.
///
/// # Errors
///
/// Returns an error if parsing, persistence, or indexing fails.
pub fn apply_refresh<S, I>(
    storage: &S,
    alias: &str,
    params: &ApplyRefreshParams,
    payload: &RefreshPayload,
    metrics: PerformanceMetrics,
    indexer: &I,
) -> Result<RefreshOutcome>
where
    S: RefreshStorage,
    I: RefreshIndexer,
{
    let mut parser = MarkdownParser::new()?;
    let mut parse_result = parser.parse(&payload.content)?;

    let filter_enabled = params.metadata.filter_non_english.unwrap_or(true);
    let filter_stats = Some(apply_language_filter(&mut parse_result, filter_enabled));

    storage.save_llms_txt(alias, &payload.content)?;

    let mut llms_json = build_llms_json(
        alias,
        &params.metadata.url,
        "llms.txt",
        payload.sha256.clone(),
        payload.etag.clone(),
        payload.last_modified.clone(),
        &parse_result,
    );

    llms_json.metadata.aliases =
        merge_aliases(params.existing_aliases.clone(), &params.metadata.aliases);
    copy_preserved_metadata_fields(&mut llms_json, &params.metadata);
    llms_json.filter_stats = filter_stats;

    storage.save_llms_json(alias, &llms_json)?;

    let origin = resolve_origin(&params.metadata);
    llms_json.metadata.origin = origin.clone();

    let metadata = build_refresh_metadata(params.metadata.clone(), payload, origin);
    storage.save_metadata(alias, &metadata)?;

    let index_path = storage.index_path(alias)?;
    indexer.index(
        alias,
        index_path.as_path(),
        metrics,
        &parse_result.heading_blocks,
    )?;

    Ok(RefreshOutcome::Refreshed {
        alias: alias.to_string(),
        headings: count_headings(&llms_json.toc),
        lines: llms_json.line_index.total_lines,
    })
}

fn apply_language_filter(
    parse_result: &mut ParseResult,
    filter_enabled: bool,
) -> HeadingFilterStats {
    let original_count = parse_result.heading_blocks.len();
    if filter_enabled {
        let mut language_filter = LanguageFilter::new(true);
        parse_result.heading_blocks.retain(|block| {
            let urls_in_content = extract_urls_from_content(&block.content);
            let url_check = urls_in_content.is_empty()
                || urls_in_content
                    .iter()
                    .all(|url| language_filter.is_english_url(url));

            let heading_check = language_filter.is_english_heading_path(&block.path);

            url_check && heading_check
        });
    }

    let accepted = parse_result.heading_blocks.len();
    let filtered_count = original_count.saturating_sub(accepted);
    HeadingFilterStats {
        enabled: filter_enabled,
        headings_total: original_count,
        headings_accepted: accepted,
        headings_rejected: filtered_count,
        reason: if filter_enabled {
            "non-English content removed".to_string()
        } else {
            "filtering disabled".to_string()
        },
    }
}

fn count_headings(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + count_headings(&entry.children))
        .sum()
}

fn extract_urls_from_content(content: &str) -> Vec<String> {
    let mut urls = Vec::new();

    let mut search_start = 0;
    while let Some(rel) = content[search_start..].find('[') {
        let open_idx = search_start + rel;
        if let Some(close_rel) = content[open_idx + 1..].find(']') {
            let close_idx = open_idx + 1 + close_rel;
            let after_bracket = content.get(close_idx + 1..).unwrap_or("");
            if let Some(rest) = after_bracket.strip_prefix('(') {
                if let Some(paren_rel) = rest.find(')') {
                    if let Some(cleaned) = clean_url_slice(&rest[..paren_rel]) {
                        urls.push(cleaned.to_string());
                    }
                }
            }
        }
        search_start = open_idx + 1;
    }

    urls
}

fn clean_url_slice(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let trimmed = trimmed
        .strip_prefix('"')
        .or_else(|| trimmed.strip_prefix('\''))
        .unwrap_or(trimmed);

    let trimmed = trimmed
        .strip_suffix('"')
        .or_else(|| trimmed.strip_suffix('\''))
        .unwrap_or(trimmed);

    let mut end = trimmed.len();
    for (idx, ch) in trimmed.char_indices().rev() {
        if trailing_punctuation(ch) {
            end = idx;
        } else {
            break;
        }
    }

    if end == 0 {
        None
    } else {
        Some(&trimmed[..end])
    }
}

const fn trailing_punctuation(c: char) -> bool {
    matches!(c, ',' | '.' | ';' | ':' | '!' | '?')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    use anyhow::Result;

    #[derive(Default)]
    struct MockStorage {
        metadata: HashMap<String, Source>,
        saved_txt: RefCell<Vec<String>>,
        saved_json: RefCell<Vec<String>>,
        saved_metadata: RefCell<Vec<Source>>,
        index_paths: HashMap<String, PathBuf>,
        cached_txt: HashMap<String, String>,
    }

    impl RefreshStorage for MockStorage {
        fn load_metadata(&self, alias: &str) -> crate::Result<Source> {
            self.metadata
                .get(alias)
                .cloned()
                .ok_or_else(|| crate::Error::NotFound("missing metadata".to_string()))
        }

        fn load_llms_aliases(&self, _alias: &str) -> crate::Result<Vec<String>> {
            Ok(Vec::new())
        }

        fn save_llms_txt(&self, alias: &str, _content: &str) -> crate::Result<()> {
            self.saved_txt.borrow_mut().push(alias.to_string());
            Ok(())
        }

        fn save_llms_json(&self, alias: &str, _data: &crate::LlmsJson) -> crate::Result<()> {
            self.saved_json.borrow_mut().push(alias.to_string());
            Ok(())
        }

        fn save_metadata(&self, _alias: &str, metadata: &Source) -> crate::Result<()> {
            self.saved_metadata.borrow_mut().push(metadata.clone());
            Ok(())
        }

        fn index_path(&self, alias: &str) -> crate::Result<PathBuf> {
            self.index_paths
                .get(alias)
                .cloned()
                .ok_or_else(|| crate::Error::NotFound("missing index path".to_string()))
        }

        fn load_llms_txt(&self, alias: &str) -> crate::Result<String> {
            self.cached_txt
                .get(alias)
                .cloned()
                .ok_or_else(|| crate::Error::NotFound(format!("missing llms.txt for {alias}")))
        }
    }

    #[derive(Default)]
    struct MockIndexer {
        indexed: RefCell<Vec<String>>,
    }

    impl RefreshIndexer for MockIndexer {
        fn index(
            &self,
            alias: &str,
            _index_path: &std::path::Path,
            _metrics: PerformanceMetrics,
            _blocks: &[crate::HeadingBlock],
        ) -> crate::Result<()> {
            self.indexed.borrow_mut().push(alias.to_string());
            Ok(())
        }
    }

    fn sample_source() -> Source {
        Source {
            url: "https://example.com/llms.txt".to_string(),
            etag: None,
            last_modified: None,
            fetched_at: chrono::Utc::now(),
            sha256: "abc123".to_string(),
            variant: crate::SourceVariant::Llms,
            aliases: Vec::new(),
            tags: Vec::new(),
            description: None,
            category: None,
            npm_aliases: Vec::new(),
            github_aliases: Vec::new(),
            origin: crate::SourceOrigin {
                manifest: None,
                source_type: Some(SourceType::Remote {
                    url: "https://example.com/llms.txt".to_string(),
                }),
            },
            filter_non_english: Some(true),
        }
    }

    fn sample_payload() -> RefreshPayload {
        RefreshPayload {
            content: "# Title\n\nSome content.\n".to_string(),
            sha256: "abc123".to_string(),
            etag: None,
            last_modified: None,
        }
    }

    #[test]
    fn apply_refresh_persists_changes() -> Result<()> {
        let mut storage = MockStorage::default();
        storage.metadata.insert("test".to_string(), sample_source());
        storage
            .index_paths
            .insert("test".to_string(), PathBuf::from("index"));

        let indexer = MockIndexer::default();
        let params = ApplyRefreshParams::new(sample_source(), Vec::new());
        let outcome = apply_refresh(
            &storage,
            "test",
            &params,
            &sample_payload(),
            PerformanceMetrics::default(),
            &indexer,
        )?;

        assert!(matches!(outcome, RefreshOutcome::Refreshed { .. }));
        assert_eq!(storage.saved_txt.borrow().len(), 1);
        assert_eq!(storage.saved_json.borrow().len(), 1);
        assert_eq!(storage.saved_metadata.borrow().len(), 1);
        assert_eq!(indexer.indexed.borrow().len(), 1);
        Ok(())
    }
}
