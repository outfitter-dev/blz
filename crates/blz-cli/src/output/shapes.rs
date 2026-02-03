// TODO(BLZ-339): Remove dead_code allow once commands adopt these shapes.
#![allow(dead_code)]
//! Shape-based output types for CLI commands.
//!
//! This module provides typed output shapes that commands can return,
//! separating data production from formatting decisions. Commands produce
//! structured data; the output system handles rendering.
//!
//! # Design
//!
//! Each shape encapsulates a specific type of CLI output:
//! - [`SearchOutput`] - Search results with metadata
//! - [`RetrieveOutput`] - Retrieved content snippets
//! - [`TocOutput`] - Table of contents / document structure
//! - [`SourceListOutput`] - List of configured sources
//! - [`SourceInfoOutput`] - Detailed source information
//! - [`CheckOutput`] - Validation results
//! - [`GenericOutput`] - Generic key-value data
//!
//! # Examples
//!
//! ```ignore
//! use blz_cli::output::shapes::SearchOutput;
//!
//! // Command produces structured data
//! let output = SearchOutput {
//!     query: "useEffect".to_string(),
//!     results: hits,
//!     total_results: 156,
//!     // ...
//! };
//!
//! // Output system handles rendering
//! output.print(OutputFormat::Json)?;
//! ```

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Context information for results with expanded line ranges.
///
/// This provides a unified representation of context across both search
/// and retrieve operations, addressing the inconsistency documented in BLZ-221.
///
/// # JSON Representation
///
/// ```json
/// {
///   "contextApplied": 5,
///   "lines": "19134-19144",
///   "lineNumbers": [19134, 19135, 19136, ...]
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextInfo {
    /// Number of context lines applied (e.g., 5 means ±5 lines from match).
    pub context_applied: usize,
    /// Expanded line range after context applied (e.g., "19134-19144").
    pub lines: String,
    /// Individual line numbers in the expanded range.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub line_numbers: Vec<usize>,
    /// The expanded content with context lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Whether the context was truncated by a `--max-lines` limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

impl ContextInfo {
    /// Create context info with the essential fields.
    #[must_use]
    pub fn new(context_applied: usize, lines: impl Into<String>) -> Self {
        Self {
            context_applied,
            lines: lines.into(),
            line_numbers: Vec::new(),
            content: None,
            truncated: None,
        }
    }

    /// Add line numbers to the context info.
    #[must_use]
    pub fn with_line_numbers(mut self, line_numbers: Vec<usize>) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    /// Add content to the context info.
    #[must_use]
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Mark the context as truncated.
    #[must_use]
    pub const fn with_truncated(mut self, truncated: bool) -> Self {
        self.truncated = Some(truncated);
        self
    }
}

/// Output shape for search results.
///
/// Contains search hits along with query metadata, pagination info,
/// and performance metrics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchOutput {
    /// Original search query.
    pub query: String,
    /// Search results as serializable values.
    pub results: Vec<SearchHitOutput>,
    /// Total number of matching results.
    pub total_results: usize,
    /// Total lines searched across all sources.
    pub total_lines_searched: usize,
    /// Search execution time in milliseconds.
    pub search_time_ms: u64,
    /// Source aliases included in the search.
    pub sources: Vec<String>,
    /// Current page number (1-based).
    pub page: usize,
    /// Results per page.
    pub page_size: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Optional fuzzy suggestions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

impl SearchOutput {
    /// Create a new search output builder.
    ///
    /// Use the builder pattern to construct a `SearchOutput`:
    /// ```ignore
    /// let output = SearchOutput::builder("query", results)
    ///     .total_results(100)
    ///     .search_time(Duration::from_millis(5))
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(query: impl Into<String>, results: Vec<SearchHitOutput>) -> SearchOutputBuilder {
        SearchOutputBuilder::new(query, results)
    }

    /// Add fuzzy suggestions to the output.
    #[must_use]
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }
}

/// Builder for `SearchOutput`.
#[derive(Debug, Clone)]
pub struct SearchOutputBuilder {
    query: String,
    results: Vec<SearchHitOutput>,
    total_results: usize,
    total_lines_searched: usize,
    search_time_ms: u64,
    sources: Vec<String>,
    page: usize,
    page_size: usize,
    total_pages: usize,
    suggestions: Option<Vec<String>>,
}

impl SearchOutputBuilder {
    /// Create a new builder with required fields.
    #[must_use]
    pub fn new(query: impl Into<String>, results: Vec<SearchHitOutput>) -> Self {
        let total_results = results.len();
        Self {
            query: query.into(),
            results,
            total_results,
            total_lines_searched: 0,
            search_time_ms: 0,
            sources: Vec::new(),
            page: 1,
            page_size: 10,
            total_pages: 1,
            suggestions: None,
        }
    }

    /// Set total results count.
    #[must_use]
    pub const fn total_results(mut self, count: usize) -> Self {
        self.total_results = count;
        self
    }

    /// Set total lines searched.
    #[must_use]
    pub const fn total_lines_searched(mut self, count: usize) -> Self {
        self.total_lines_searched = count;
        self
    }

    /// Set search execution time.
    #[must_use]
    pub fn search_time(mut self, duration: Duration) -> Self {
        self.search_time_ms = duration.as_millis().try_into().unwrap_or(u64::MAX);
        self
    }

    /// Set source aliases.
    #[must_use]
    pub fn sources(mut self, sources: Vec<String>) -> Self {
        self.sources = sources;
        self
    }

    /// Set pagination: page number (1-based).
    #[must_use]
    pub const fn page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set pagination: results per page.
    #[must_use]
    pub const fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    /// Set pagination: total pages.
    #[must_use]
    pub const fn total_pages(mut self, pages: usize) -> Self {
        self.total_pages = pages;
        self
    }

    /// Set fuzzy suggestions.
    #[must_use]
    pub fn suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    /// Build the `SearchOutput`.
    #[must_use]
    pub fn build(self) -> SearchOutput {
        SearchOutput {
            query: self.query,
            results: self.results,
            total_results: self.total_results,
            total_lines_searched: self.total_lines_searched,
            search_time_ms: self.search_time_ms,
            sources: self.sources,
            page: self.page,
            page_size: self.page_size,
            total_pages: self.total_pages,
            suggestions: self.suggestions,
        }
    }
}

/// A single search hit in the output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHitOutput {
    /// Source alias.
    pub alias: String,
    /// Line range (e.g., "12-15").
    pub lines: String,
    /// Content snippet.
    pub snippet: String,
    /// Relevance score (0-100).
    pub score: u8,
    /// Raw score value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_score: Option<f32>,
    /// Heading path/breadcrumbs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
    /// Optional anchor link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// Source URL if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Context information when `-C` or `--context` is applied.
    ///
    /// This field provides unified context representation across search and
    /// retrieve operations, replacing the previous inconsistent patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextInfo>,
}

/// Output shape for retrieved content.
///
/// Contains one or more retrieved snippets with context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrieveOutput {
    /// Retrieved content requests.
    pub requests: Vec<RetrievedContent>,
    /// Whether all requests succeeded.
    pub success: bool,
    /// Number of successful retrievals.
    pub retrieved_count: usize,
    /// Number of failed retrievals.
    pub failed_count: usize,
}

impl RetrieveOutput {
    /// Create output from a list of retrieved content.
    #[must_use]
    pub fn new(requests: Vec<RetrievedContent>) -> Self {
        let success = requests.iter().all(|r| r.error.is_none());
        let retrieved_count = requests.iter().filter(|r| r.error.is_none()).count();
        let failed_count = requests.len() - retrieved_count;
        Self {
            requests,
            success,
            retrieved_count,
            failed_count,
        }
    }
}

/// A single retrieved content block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrievedContent {
    /// Source alias.
    pub alias: String,
    /// Requested line range.
    pub lines: String,
    /// Retrieved content snippet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    /// Content with context lines if requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_with_context: Option<String>,
    /// Heading path for context.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
    /// Error message if retrieval failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Context information when `-C` or `--context` is applied.
    ///
    /// This field provides unified context representation across search and
    /// retrieve operations, replacing the inconsistent `contextApplied` patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextInfo>,
}

/// Output shape for table of contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocOutput {
    /// Source alias.
    pub alias: String,
    /// TOC entries.
    pub entries: Vec<TocEntry>,
    /// Total number of entries.
    pub total_entries: usize,
    /// Maximum heading depth in the TOC.
    pub max_depth: u8,
}

impl TocOutput {
    /// Create a new TOC output.
    #[must_use]
    pub fn new(alias: impl Into<String>, entries: Vec<TocEntry>) -> Self {
        let total_entries = Self::count_entries_recursive(&entries);
        let max_depth = Self::max_depth_recursive(&entries);
        Self {
            alias: alias.into(),
            entries,
            total_entries,
            max_depth,
        }
    }

    /// Recursively count all entries including nested children.
    fn count_entries_recursive(entries: &[TocEntry]) -> usize {
        entries
            .iter()
            .map(|e| 1 + Self::count_entries_recursive(&e.children))
            .sum()
    }

    /// Recursively find the maximum heading depth.
    fn max_depth_recursive(entries: &[TocEntry]) -> u8 {
        entries
            .iter()
            .map(|e| {
                let child_max = Self::max_depth_recursive(&e.children);
                e.level.max(child_max)
            })
            .max()
            .unwrap_or(0)
    }
}

/// A single TOC entry for tree view.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    /// Heading level (1-6).
    pub level: u8,
    /// Heading text.
    pub title: String,
    /// Line range in source (e.g., "12-15").
    pub lines: String,
    /// Anchor link if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// Breadcrumb path to this heading.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
    /// Child entries for tree view.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
}

/// Output shape for paginated TOC (flat list across sources).
///
/// This is the format used for JSON/JSONL output and paginated text output.
/// Uses `snake_case` for JSON serialization to match existing API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TocPaginatedOutput {
    /// TOC entries for the current page.
    pub entries: Vec<TocPaginatedEntry>,
    /// Current page number (1-based).
    pub page: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Total number of entries across all pages.
    pub total_results: usize,
    /// Page size (entries per page), if pagination is active.
    pub page_size: Option<usize>,
}

impl TocPaginatedOutput {
    /// Create a new paginated TOC output.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to Vec parameter
    pub fn new(
        entries: Vec<TocPaginatedEntry>,
        page: usize,
        total_pages: usize,
        total_results: usize,
        page_size: Option<usize>,
    ) -> Self {
        Self {
            entries,
            page,
            total_pages,
            total_results,
            page_size,
        }
    }
}

/// A single entry in paginated TOC output.
///
/// This is a flat entry format that includes source information,
/// used when displaying entries across multiple sources.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocPaginatedEntry {
    /// Source alias.
    pub alias: String,
    /// Canonical source name.
    pub source: String,
    /// Display heading path (breadcrumbs).
    pub heading_path: Vec<String>,
    /// Raw heading path.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub raw_heading_path: Vec<String>,
    /// Normalized heading path.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heading_path_normalized: Vec<String>,
    /// Heading level (1-6).
    pub heading_level: u8,
    /// Line range (e.g., "12-15").
    pub lines: String,
    /// Anchor link if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

/// Output shape for multi-source tree TOC.
///
/// This is used when displaying tree/hierarchical TOC for multiple sources.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocMultiOutput {
    /// TOC entries per source.
    pub sources: Vec<TocOutput>,
    /// Total entries across all sources.
    pub total_entries: usize,
}

impl TocMultiOutput {
    /// Create a new multi-source TOC output.
    #[must_use]
    pub fn new(sources: Vec<TocOutput>) -> Self {
        let total_entries = sources.iter().map(|s| s.total_entries).sum();
        Self {
            sources,
            total_entries,
        }
    }
}

/// Render options for TOC output.
///
/// These options control how the TOC is rendered in text format.
#[derive(Clone, Debug, Default)]
pub struct TocRenderOptions {
    /// Display as hierarchical tree with box-drawing characters.
    pub tree_mode: bool,
    /// Show anchor slugs in output.
    pub show_anchors: bool,
}

/// Output shape for source list.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceListOutput {
    /// List of sources.
    pub sources: Vec<SourceSummary>,
    /// Total number of sources.
    pub total: usize,
}

impl SourceListOutput {
    /// Create a new source list output.
    #[must_use]
    pub fn new(sources: Vec<SourceSummary>) -> Self {
        let total = sources.len();
        Self { sources, total }
    }
}

/// Summary information for a source in list view.
///
/// This struct contains all fields needed for both text and JSON output formats.
/// Fields are designed to be backward compatible - new fields are optional.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    /// Source alias (primary identifier).
    pub alias: String,
    /// Source URL.
    pub url: String,
    /// Source status indicator.
    #[serde(default)]
    pub status: SourceStatus,
    /// Line count in cached content.
    pub lines: usize,
    /// Total heading count in the document.
    #[serde(default)]
    pub headings: usize,
    /// Source tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Additional aliases for this source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Fetch timestamp in RFC3339 format (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// Content checksum (SHA-256).
    #[serde(rename = "sha256", skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Optional `ETag` for conditional fetching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Optional Last-Modified header value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    /// Optional description from metadata or descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional category from metadata or descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// NPM aliases associated with the source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub npm_aliases: Vec<String>,
    /// GitHub aliases associated with the source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub github_aliases: Vec<String>,
    /// Source origin metadata (serialized as JSON value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<serde_json::Value>,
    /// Optional descriptor metadata (serialized as JSON value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<serde_json::Value>,
}

/// Source status indicator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    /// Source is up to date.
    Fresh,
    /// Source needs updating.
    Stale,
    /// Source has errors.
    Error,
    /// Source status is unknown.
    #[default]
    Unknown,
}

impl std::fmt::Display for SourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fresh => write!(f, "fresh"),
            Self::Stale => write!(f, "stale"),
            Self::Error => write!(f, "error"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl SourceSummary {
    /// Create a new source summary with required fields.
    ///
    /// Use the builder pattern to set optional fields.
    #[must_use]
    pub fn new(alias: impl Into<String>, url: impl Into<String>, lines: usize) -> Self {
        Self {
            alias: alias.into(),
            url: url.into(),
            lines,
            ..Default::default()
        }
    }

    /// Set the status.
    #[must_use]
    pub const fn with_status(mut self, status: SourceStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the heading count.
    #[must_use]
    pub const fn with_headings(mut self, headings: usize) -> Self {
        self.headings = headings;
        self
    }

    /// Set tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set aliases.
    #[must_use]
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Set the `fetched_at` timestamp.
    #[must_use]
    pub fn with_fetched_at(mut self, fetched_at: impl Into<String>) -> Self {
        self.fetched_at = Some(fetched_at.into());
        self
    }

    /// Set the checksum.
    #[must_use]
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Set the `ETag`.
    #[must_use]
    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = Some(etag.into());
        self
    }

    /// Set the `last_modified` timestamp.
    #[must_use]
    pub fn with_last_modified(mut self, last_modified: impl Into<String>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set npm aliases.
    #[must_use]
    pub fn with_npm_aliases(mut self, npm_aliases: Vec<String>) -> Self {
        self.npm_aliases = npm_aliases;
        self
    }

    /// Set GitHub aliases.
    #[must_use]
    pub fn with_github_aliases(mut self, github_aliases: Vec<String>) -> Self {
        self.github_aliases = github_aliases;
        self
    }

    /// Set the origin as a JSON value.
    #[must_use]
    pub fn with_origin(mut self, origin: serde_json::Value) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set the descriptor as a JSON value.
    #[must_use]
    pub fn with_descriptor(mut self, descriptor: serde_json::Value) -> Self {
        self.descriptor = Some(descriptor);
        self
    }
}

/// Detailed output for a single source.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfoOutput {
    /// Source alias.
    pub alias: String,
    /// Source URL.
    pub url: String,
    /// Variant (llms, llms-full, or custom).
    pub variant: String,
    /// Additional aliases for this source.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Line count.
    pub lines: usize,
    /// Total heading count.
    pub headings: usize,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last updated timestamp (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// HTTP `ETag` if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// SHA256 checksum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Path to cached source directory.
    pub cache_path: String,
    /// Language filtering statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_stats: Option<FilterStatsOutput>,
}

/// Language filtering statistics for source info output.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterStatsOutput {
    /// Whether filtering was enabled.
    pub enabled: bool,
    /// Total number of headings before filtering.
    pub headings_total: usize,
    /// Number of headings that passed the filter.
    pub headings_accepted: usize,
    /// Number of headings that were rejected.
    pub headings_rejected: usize,
    /// Human-readable reason for filtering.
    pub reason: String,
}

impl SourceInfoOutput {
    /// Create a new source info output with required fields.
    #[must_use]
    pub fn new(
        alias: impl Into<String>,
        url: impl Into<String>,
        variant: impl Into<String>,
        lines: usize,
        headings: usize,
        size_bytes: u64,
        cache_path: impl Into<String>,
    ) -> Self {
        Self {
            alias: alias.into(),
            url: url.into(),
            variant: variant.into(),
            aliases: Vec::new(),
            lines,
            headings,
            size_bytes,
            last_updated: None,
            etag: None,
            checksum: None,
            cache_path: cache_path.into(),
            filter_stats: None,
        }
    }

    /// Set additional aliases.
    #[must_use]
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Set last updated timestamp.
    #[must_use]
    pub fn with_last_updated(mut self, last_updated: impl Into<String>) -> Self {
        self.last_updated = Some(last_updated.into());
        self
    }

    /// Set the `ETag`.
    #[must_use]
    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = Some(etag.into());
        self
    }

    /// Set the checksum.
    #[must_use]
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Set filter statistics.
    #[must_use]
    pub fn with_filter_stats(mut self, stats: FilterStatsOutput) -> Self {
        self.filter_stats = Some(stats);
        self
    }
}

/// Output shape for validation/check results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckOutput {
    /// Source alias being checked.
    pub alias: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<CheckResult>,
    /// Summary message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl CheckOutput {
    /// Create a new check output.
    #[must_use]
    pub fn new(alias: impl Into<String>, checks: Vec<CheckResult>) -> Self {
        let passed = checks.iter().all(|c| c.passed);
        Self {
            alias: alias.into(),
            passed,
            checks,
            summary: None,
        }
    }

    /// Add a summary message.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
}

/// A single check result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name/identifier.
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Check result message.
    pub message: String,
    /// Optional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Generic key-value output for metadata and simple responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericOutput {
    /// Key-value data.
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

impl GenericOutput {
    /// Create a new generic output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert a value.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be serialized to JSON.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> Result<(), serde_json::Error> {
        let v = serde_json::to_value(value)?;
        self.data.insert(key.into(), v);
        Ok(())
    }

    /// Build from an iterator of key-value pairs.
    ///
    /// # Errors
    ///
    /// Returns an error if any value cannot be serialized to JSON.
    pub fn from_iter<K, V, I>(iter: I) -> Result<Self, serde_json::Error>
    where
        K: Into<String>,
        V: Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut output = Self::new();
        for (k, v) in iter {
            output.insert(k, v)?;
        }
        Ok(output)
    }
}

impl Default for GenericOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified output shape enum for all command outputs.
///
/// Commands can return this enum, and the output system can render
/// it in any supported format (text, json, jsonl, raw).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputShape {
    /// Search results output.
    Search(SearchOutput),
    /// Retrieved content output.
    Retrieve(RetrieveOutput),
    /// Table of contents output (tree view for single source).
    Toc(TocOutput),
    /// Table of contents output (paginated flat list).
    TocPaginated(TocPaginatedOutput),
    /// Table of contents output (multi-source tree view).
    TocMulti(TocMultiOutput),
    /// Source list output.
    SourceList(SourceListOutput),
    /// Source info output.
    SourceInfo(SourceInfoOutput),
    /// Check/validation output.
    Check(CheckOutput),
    /// Generic metadata output.
    Generic(GenericOutput),
}

impl From<SearchOutput> for OutputShape {
    fn from(v: SearchOutput) -> Self {
        Self::Search(v)
    }
}

impl From<RetrieveOutput> for OutputShape {
    fn from(v: RetrieveOutput) -> Self {
        Self::Retrieve(v)
    }
}

impl From<TocOutput> for OutputShape {
    fn from(v: TocOutput) -> Self {
        Self::Toc(v)
    }
}

impl From<TocPaginatedOutput> for OutputShape {
    fn from(v: TocPaginatedOutput) -> Self {
        Self::TocPaginated(v)
    }
}

impl From<TocMultiOutput> for OutputShape {
    fn from(v: TocMultiOutput) -> Self {
        Self::TocMulti(v)
    }
}

impl From<SourceListOutput> for OutputShape {
    fn from(v: SourceListOutput) -> Self {
        Self::SourceList(v)
    }
}

impl From<SourceInfoOutput> for OutputShape {
    fn from(v: SourceInfoOutput) -> Self {
        Self::SourceInfo(v)
    }
}

impl From<CheckOutput> for OutputShape {
    fn from(v: CheckOutput) -> Self {
        Self::Check(v)
    }
}

impl From<GenericOutput> for OutputShape {
    fn from(v: GenericOutput) -> Self {
        Self::Generic(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_output_serialization() {
        let output = SearchOutput::builder(
            "test query",
            vec![SearchHitOutput {
                alias: "react".to_string(),
                lines: "12-15".to_string(),
                snippet: "useEffect example".to_string(),
                score: 95,
                raw_score: Some(14.5),
                heading_path: vec!["Hooks".to_string(), "useEffect".to_string()],
                anchor: Some("use-effect".to_string()),
                source_url: None,
                context: None,
            }],
        )
        .total_results(1)
        .total_lines_searched(1000)
        .search_time(Duration::from_millis(5))
        .sources(vec!["react".to_string()])
        .page(1)
        .page_size(10)
        .total_pages(1)
        .build();

        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("test query"));
        assert!(json.contains("react"));
    }

    #[test]
    fn test_retrieve_output_success() {
        let output = RetrieveOutput::new(vec![RetrievedContent {
            alias: "react".to_string(),
            lines: "12-15".to_string(),
            snippet: Some("content".to_string()),
            content_with_context: None,
            heading_path: vec![],
            error: None,
            context: None,
        }]);

        assert!(output.success);
        assert_eq!(output.retrieved_count, 1);
        assert_eq!(output.failed_count, 0);
    }

    #[test]
    fn test_retrieve_output_partial_failure() {
        let output = RetrieveOutput::new(vec![
            RetrievedContent {
                alias: "react".to_string(),
                lines: "12-15".to_string(),
                snippet: Some("content".to_string()),
                content_with_context: None,
                heading_path: vec![],
                error: None,
                context: None,
            },
            RetrievedContent {
                alias: "missing".to_string(),
                lines: "1-5".to_string(),
                snippet: None,
                content_with_context: None,
                heading_path: vec![],
                error: Some("Source not found".to_string()),
                context: None,
            },
        ]);

        assert!(!output.success);
        assert_eq!(output.retrieved_count, 1);
        assert_eq!(output.failed_count, 1);
    }

    #[test]
    fn test_toc_output() {
        let output = TocOutput::new(
            "react",
            vec![
                TocEntry {
                    level: 1,
                    title: "Getting Started".to_string(),
                    lines: "1-50".to_string(),
                    anchor: None,
                    heading_path: vec!["Getting Started".to_string()],
                    children: vec![],
                },
                TocEntry {
                    level: 2,
                    title: "Installation".to_string(),
                    lines: "10-30".to_string(),
                    anchor: Some("installation".to_string()),
                    heading_path: vec!["Getting Started".to_string(), "Installation".to_string()],
                    children: vec![],
                },
            ],
        );

        assert_eq!(output.total_entries, 2);
        assert_eq!(output.max_depth, 2);
    }

    #[test]
    fn test_toc_output_recursive_counting() {
        let output = TocOutput::new(
            "docs",
            vec![TocEntry {
                level: 1,
                title: "Root".to_string(),
                lines: "1-100".to_string(),
                anchor: None,
                heading_path: vec!["Root".to_string()],
                children: vec![
                    TocEntry {
                        level: 2,
                        title: "Child 1".to_string(),
                        lines: "10-50".to_string(),
                        anchor: None,
                        heading_path: vec!["Root".to_string(), "Child 1".to_string()],
                        children: vec![TocEntry {
                            level: 3,
                            title: "Grandchild".to_string(),
                            lines: "20-30".to_string(),
                            anchor: None,
                            heading_path: vec![
                                "Root".to_string(),
                                "Child 1".to_string(),
                                "Grandchild".to_string(),
                            ],
                            children: vec![],
                        }],
                    },
                    TocEntry {
                        level: 2,
                        title: "Child 2".to_string(),
                        lines: "60-80".to_string(),
                        anchor: None,
                        heading_path: vec!["Root".to_string(), "Child 2".to_string()],
                        children: vec![],
                    },
                ],
            }],
        );

        // 1 root + 2 children + 1 grandchild = 4 total
        assert_eq!(output.total_entries, 4);
        // Deepest level is 3 (grandchild)
        assert_eq!(output.max_depth, 3);
    }

    #[test]
    fn test_source_status_display() {
        assert_eq!(SourceStatus::Fresh.to_string(), "fresh");
        assert_eq!(SourceStatus::Stale.to_string(), "stale");
        assert_eq!(SourceStatus::Error.to_string(), "error");
        assert_eq!(SourceStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_check_output_all_passed() {
        let output = CheckOutput::new(
            "react",
            vec![
                CheckResult {
                    name: "index".to_string(),
                    passed: true,
                    message: "Index is valid".to_string(),
                    details: None,
                },
                CheckResult {
                    name: "content".to_string(),
                    passed: true,
                    message: "Content is valid".to_string(),
                    details: None,
                },
            ],
        );

        assert!(output.passed);
    }

    #[test]
    fn test_check_output_some_failed() {
        let output = CheckOutput::new(
            "react",
            vec![
                CheckResult {
                    name: "index".to_string(),
                    passed: true,
                    message: "Index is valid".to_string(),
                    details: None,
                },
                CheckResult {
                    name: "content".to_string(),
                    passed: false,
                    message: "Content is corrupted".to_string(),
                    details: Some("CRC mismatch".to_string()),
                },
            ],
        );

        assert!(!output.passed);
    }

    #[test]
    fn test_generic_output() {
        let mut output = GenericOutput::new();
        output.insert("version", "1.0.0").expect("insert version");
        output.insert("count", 42).expect("insert count");

        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("version"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("count"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_output_shape_from_conversions() {
        let search = SearchOutput::builder("test", vec![]).build();
        let shape: OutputShape = search.into();
        assert!(matches!(shape, OutputShape::Search(_)));

        let retrieve = RetrieveOutput::new(vec![]);
        let shape: OutputShape = retrieve.into();
        assert!(matches!(shape, OutputShape::Retrieve(_)));

        let toc = TocOutput::new("test", vec![]);
        let shape: OutputShape = toc.into();
        assert!(matches!(shape, OutputShape::Toc(_)));
    }

    #[test]
    fn test_context_info_serialization() {
        // Test basic context info
        let context = ContextInfo::new(5, "19134-19144");
        let json = serde_json::to_string(&context).expect("serialize");
        assert!(json.contains("\"contextApplied\":5"));
        assert!(json.contains("\"lines\":\"19134-19144\""));
        // Empty line_numbers should be skipped
        assert!(!json.contains("lineNumbers"));

        // Test with all fields populated
        let context = ContextInfo::new(5, "19134-19144")
            .with_line_numbers(vec![19134, 19135, 19136])
            .with_content("example content")
            .with_truncated(false);
        let json = serde_json::to_string(&context).expect("serialize");
        assert!(json.contains("\"lineNumbers\":[19134,19135,19136]"));
        assert!(json.contains("\"content\":\"example content\""));
        assert!(json.contains("\"truncated\":false"));
    }

    #[test]
    fn test_search_hit_with_context() {
        let hit = SearchHitOutput {
            alias: "react".to_string(),
            lines: "12-15".to_string(),
            snippet: "useEffect example".to_string(),
            score: 95,
            raw_score: None,
            heading_path: vec![],
            anchor: None,
            source_url: None,
            context: Some(ContextInfo::new(5, "7-20").with_line_numbers((7..=20).collect())),
        };

        let json = serde_json::to_string(&hit).expect("serialize");
        assert!(json.contains("\"context\":{"));
        assert!(json.contains("\"contextApplied\":5"));
        assert!(json.contains("\"lines\":\"7-20\""));
    }

    #[test]
    fn test_retrieved_content_with_context() {
        let content = RetrievedContent {
            alias: "react".to_string(),
            lines: "100-110".to_string(),
            snippet: Some("component code".to_string()),
            content_with_context: Some("expanded content".to_string()),
            heading_path: vec!["Components".to_string()],
            error: None,
            context: Some(ContextInfo::new(10, "90-120").with_truncated(true)),
        };

        let json = serde_json::to_string(&content).expect("serialize");
        assert!(json.contains("\"context\":{"));
        assert!(json.contains("\"contextApplied\":10"));
        assert!(json.contains("\"truncated\":true"));
    }

    #[test]
    fn test_toc_paginated_output() {
        let entries = vec![
            TocPaginatedEntry {
                alias: "react".to_string(),
                source: "react".to_string(),
                heading_path: vec!["Hooks".to_string(), "useEffect".to_string()],
                raw_heading_path: vec!["hooks".to_string(), "use-effect".to_string()],
                heading_path_normalized: vec!["hooks".to_string(), "useeffect".to_string()],
                heading_level: 2,
                lines: "100-150".to_string(),
                anchor: Some("use-effect".to_string()),
            },
            TocPaginatedEntry {
                alias: "react".to_string(),
                source: "react".to_string(),
                heading_path: vec!["Hooks".to_string(), "useState".to_string()],
                raw_heading_path: vec![],
                heading_path_normalized: vec![],
                heading_level: 2,
                lines: "200-250".to_string(),
                anchor: None,
            },
        ];

        let output = TocPaginatedOutput::new(entries, 1, 5, 100, Some(20));

        assert_eq!(output.page, 1);
        assert_eq!(output.total_pages, 5);
        assert_eq!(output.total_results, 100);
        assert_eq!(output.page_size, Some(20));
        assert_eq!(output.entries.len(), 2);

        // Test JSON serialization: entries use camelCase, pagination uses snake_case
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"headingPath\"")); // Entry field (camelCase)
        assert!(json.contains("\"headingLevel\"")); // Entry field (camelCase)
        assert!(json.contains("\"total_pages\"")); // Pagination field (snake_case)
        assert!(json.contains("\"total_results\"")); // Pagination field (snake_case)
    }

    #[test]
    fn test_toc_multi_output() {
        let react_entries = vec![TocEntry {
            level: 1,
            title: "Getting Started".to_string(),
            lines: "1-50".to_string(),
            anchor: None,
            heading_path: vec!["Getting Started".to_string()],
            children: vec![],
        }];

        let bun_entries = vec![TocEntry {
            level: 1,
            title: "Installation".to_string(),
            lines: "1-30".to_string(),
            anchor: Some("installation".to_string()),
            heading_path: vec!["Installation".to_string()],
            children: vec![],
        }];

        let output = TocMultiOutput::new(vec![
            TocOutput::new("react", react_entries),
            TocOutput::new("bun", bun_entries),
        ]);

        assert_eq!(output.sources.len(), 2);
        assert_eq!(output.total_entries, 2);
        assert_eq!(output.sources[0].alias, "react");
        assert_eq!(output.sources[1].alias, "bun");
    }

    #[test]
    fn test_toc_output_shape_conversions() {
        // Test TocPaginatedOutput conversion
        let paginated = TocPaginatedOutput::new(vec![], 1, 1, 0, None);
        let shape: OutputShape = paginated.into();
        assert!(matches!(shape, OutputShape::TocPaginated(_)));

        // Test TocMultiOutput conversion
        let multi = TocMultiOutput::new(vec![]);
        let shape: OutputShape = multi.into();
        assert!(matches!(shape, OutputShape::TocMulti(_)));
    }
}
