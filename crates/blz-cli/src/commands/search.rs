//! Search command implementation

use anyhow::{Context, Result};
use blz_core::index::{DEFAULT_SNIPPET_CHAR_LIMIT, MAX_SNIPPET_CHAR_LIMIT, MIN_SNIPPET_CHAR_LIMIT};
use blz_core::numeric::percentile_count;
use blz_core::{
    HitContext, LlmsJson, PerformanceMetrics, ResourceMonitor, SearchHit, SearchIndex, Source,
    Storage,
};
use clap::Args;
use futures::stream::{self, StreamExt};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;

use crate::args::{ContextMode, ShowComponent};
use crate::cli::{Commands, merge_context_flags};
use crate::output::{FormatParams, OutputFormat, SearchResultFormatter};
use crate::utils::cli_args::{FormatArg, flag_present};
use crate::utils::history_log;
use crate::utils::parsing::parse_line_span;
use crate::utils::preferences::{CliPreferences, SearchHistoryEntry};
use crate::utils::staleness::{self, DEFAULT_STALE_AFTER_DAYS};
use crate::utils::toc::{
    extract_block_slice, finalize_block_slice, find_heading_span, heading_level_from_line,
};

pub(super) const ALL_RESULTS_LIMIT: usize = 10_000;
pub(super) const DEFAULT_SCORE_PRECISION: u8 = 1;
/// Default maximum characters in a snippet before truncation.
pub const DEFAULT_MAX_CHARS: usize = DEFAULT_SNIPPET_CHAR_LIMIT;
/// Default limit for search results. Can be overridden via `BLZ_DEFAULT_LIMIT` env var.
pub(super) const DEFAULT_SEARCH_LIMIT: usize = 50;

/// Get the default search limit, checking `BLZ_DEFAULT_LIMIT` env var first.
pub(super) fn default_search_limit() -> usize {
    std::env::var("BLZ_DEFAULT_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
}

/// Arguments for the deprecated `blz search` command.
///
/// This command is deprecated in favor of `blz query` for search operations.
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct SearchArgs {
    /// Search query (required unless --next, --previous, or --last)
    #[arg(required_unless_present_any = ["next", "previous", "last"])]
    pub query: Option<String>,
    /// Filter by source(s) - comma-separated or repeated (-s a -s b)
    #[arg(
        long = "source",
        short = 's',
        visible_alias = "alias",
        visible_alias = "sources",
        value_name = "SOURCE",
        value_delimiter = ','
    )]
    pub sources: Vec<String>,
    /// Continue from previous search (next page)
    #[arg(
        long,
        conflicts_with = "page",
        conflicts_with = "last",
        conflicts_with = "previous",
        display_order = 50
    )]
    pub next: bool,
    /// Go back to previous page
    #[arg(
        long,
        conflicts_with = "page",
        conflicts_with = "last",
        conflicts_with = "next",
        display_order = 51
    )]
    pub previous: bool,
    /// Jump to last page of results
    #[arg(
        long,
        conflicts_with = "next",
        conflicts_with = "page",
        conflicts_with = "previous",
        display_order = 52
    )]
    pub last: bool,
    /// Maximum number of results per page (default 50; internally fetches up to 3x this value for scoring stability)
    #[arg(
        short = 'n',
        long,
        value_name = "COUNT",
        conflicts_with = "all",
        display_order = 53
    )]
    pub limit: Option<usize>,
    /// Show all results (no limit)
    #[arg(long, conflicts_with = "limit", display_order = 54)]
    pub all: bool,
    /// Page number for pagination
    #[arg(
        long,
        default_value = "1",
        conflicts_with = "next",
        conflicts_with = "last",
        display_order = 55
    )]
    pub page: usize,
    /// Show only top N percentile of results (1-100). Applied after paging is calculated.
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub top: Option<u8>,
    /// Filter results by heading level
    ///
    /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
    ///
    /// Examples:
    ///   -H <=2       # Level 1 and 2 headings only
    ///   -H >2        # Level 3+ headings only
    ///   -H 1,2,3     # Levels 1, 2, and 3 only
    ///   -H 2-4       # Levels 2, 3, and 4 only
    #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
    pub heading_level: Option<String>,
    /// Output format (text, json, jsonl)
    #[command(flatten)]
    pub format: FormatArg,
    /// Additional columns to include in text output
    #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
    pub show: Vec<ShowComponent>,
    /// Hide the summary/footer line
    #[arg(long = "no-summary")]
    pub no_summary: bool,
    /// Number of decimal places to show for scores (0-4)
    #[arg(
        long = "score-precision",
        value_name = "PLACES",
        value_parser = clap::value_parser!(u8).range(0..=4),
        env = "BLZ_SCORE_PRECISION"
    )]
    pub score_precision: Option<u8>,
    /// Maximum snippet lines to display around a hit (1-10)
    #[arg(
        long = "snippet-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(u8).range(1..=10),
        env = "BLZ_SNIPPET_LINES",
        default_value_t = 3,
        hide = true
    )]
    pub snippet_lines: u8,
    /// Maximum total characters in snippet (including newlines). Range: 50-1000, default: 200.
    #[arg(
        long = "max-chars",
        value_name = "CHARS",
        env = "BLZ_MAX_CHARS",
        value_parser = clap::value_parser!(usize)
    )]
    pub max_chars: Option<usize>,
    /// Print LINES lines of context (both before and after match). Same as -C.
    ///
    /// Use "all" to expand to the full heading section containing the match.
    /// If no heading encompasses the match, returns only the matched lines.
    ///
    /// Examples:
    ///   -C 10              # 10 lines before and after
    ///   -C all             # Expand to containing heading section
    ///   --context 5        # Long form (also valid)
    #[arg(
        short = 'C',
        long = "context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context_deprecated"],
        display_order = 30
    )]
    pub context: Option<ContextMode>,
    /// Deprecated: use -C or --context instead (hidden for backward compatibility)
    #[arg(
        short = 'c',
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context"],
        hide = true,
        display_order = 100
    )]
    pub context_deprecated: Option<ContextMode>,
    /// Print LINES lines of context after each match
    ///
    /// Examples:
    ///   -A3                # 3 lines after match
    ///   --after-context 5  # 5 lines after match
    #[arg(
        short = 'A',
        long = "after-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 31
    )]
    pub after_context: Option<usize>,
    /// Print LINES lines of context before each match
    ///
    /// Examples:
    ///   -B3                # 3 lines before match
    ///   --before-context 5 # 5 lines before match
    #[arg(
        short = 'B',
        long = "before-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 32
    )]
    pub before_context: Option<usize>,
    /// Expand to the full heading section containing each hit.
    ///
    /// If no heading encompasses the range, returns only the requested lines.
    /// Legacy alias for --context all.
    #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
    pub block: bool,
    /// Maximum number of lines to include when using block expansion (--block or --context all)
    #[arg(
        long = "max-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(usize),
        display_order = 34
    )]
    pub max_lines: Option<usize>,
    /// Restrict matches to heading text only
    #[arg(long = "headings-only", display_order = 35)]
    pub headings_only: bool,
    /// Don't save this search to history
    #[arg(long = "no-history")]
    pub no_history: bool,
    /// Copy results to clipboard using OSC 52 escape sequence
    #[arg(long)]
    pub copy: bool,
    /// Show detailed timing breakdown for performance analysis
    #[arg(long)]
    pub timing: bool,
}

/// Search options
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct SearchOptions {
    pub query: String,
    pub sources: Vec<String>,
    pub last: bool,
    pub limit: usize,
    pub page: usize,
    pub top_percentile: Option<u8>,
    pub format: OutputFormat,
    pub show_url: bool,
    pub show_lines: bool,
    pub show_anchor: bool,
    pub show_raw_score: bool,
    pub no_summary: bool,
    pub score_precision: Option<u8>,
    pub snippet_lines: u8,
    pub(crate) all: bool,
    pub no_history: bool,
    pub copy: bool,
    pub before_context: usize,
    pub after_context: usize,
    pub block: bool,
    pub max_block_lines: Option<usize>,
    pub max_chars: usize,
    pub quiet: bool,
    pub headings_only: bool,
    pub timing: bool,
}

#[derive(Default, Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct ShowToggles {
    pub(super) url: bool,
    pub(super) lines: bool,
    pub(super) anchor: bool,
    pub(super) raw_score: bool,
}

pub(super) fn resolve_show_components(components: &[ShowComponent]) -> ShowToggles {
    let mut toggles = ShowToggles::default();
    for component in components {
        match component {
            ShowComponent::Rank => {
                // Rank is always displayed by default; accept the modifier for compatibility.
            },
            ShowComponent::Url => toggles.url = true,
            ShowComponent::Lines => toggles.lines = true,
            ShowComponent::Anchor => toggles.anchor = true,
            ShowComponent::RawScore => toggles.raw_score = true,
        }
    }
    toggles
}

/// Clamp snippet character limits to the supported range.
pub fn clamp_max_chars(value: usize) -> usize {
    value.clamp(MIN_SNIPPET_CHAR_LIMIT, MAX_SNIPPET_CHAR_LIMIT)
}

/// Execute a search across cached documentation
///
/// This function delegates to the unified `find` command for consistent behavior.
///
/// **Deprecated**: Use `blz find` instead. The `search` command will be removed in a future release.
///
/// # Parameters
///
/// * `emit_deprecation_warning` - If `true`, prints a deprecation warning to stderr.
///   Should be `true` when called from the deprecated `blz search` command,
///   and `false` when called from valid commands like `blz docs search`.
pub async fn execute(
    query: &str,
    sources: &[String],
    config: &crate::config::QueryExecutionConfig,
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
    emit_deprecation_warning: bool,
) -> Result<()> {
    // Emit deprecation warning to stderr (doesn't interfere with JSON output)
    // Only warn when called from the deprecated `blz search` command,
    // not from valid commands like `blz docs search`
    if emit_deprecation_warning {
        eprintln!("warning: `blz search` is deprecated, use `blz find` instead");
    }

    // Delegate to find command
    let inputs = vec![query.to_string()];
    super::find::execute(&inputs, sources, config, prefs, metrics, resource_monitor).await
}

pub(super) struct SearchResults {
    pub(super) hits: Vec<SearchHit>,
    pub(super) total_lines_searched: usize,
    pub(super) search_time: std::time::Duration,
    pub(super) sources: Vec<String>,
}

fn get_max_concurrent_searches() -> usize {
    std::thread::available_parallelism().map_or(8, |n| (n.get().saturating_mul(2)).min(16))
}

/// Resolve requested source aliases, providing fuzzy suggestions for unknown sources.
///
/// Returns the list of canonical source names to search.
fn resolve_requested_sources(
    storage: &Storage,
    requested: &[String],
    quiet: bool,
) -> Result<Vec<String>> {
    let mut resolved = Vec::new();
    for alias in requested {
        match crate::utils::resolver::resolve_source(storage, alias) {
            Ok(Some(canonical)) => resolved.push(canonical),
            Ok(None) => {
                // Source not found - use fuzzy matching to suggest similar sources
                let known = storage.list_sources();
                if !known.contains(alias) && !quiet {
                    let matcher = SkimMatcherV2::default();
                    let mut suggestions: Vec<(i64, String)> = known
                        .iter()
                        .filter_map(|source| {
                            matcher
                                .fuzzy_match(source, alias)
                                .filter(|&score| score > 0)
                                .map(|score| (score, source.clone()))
                        })
                        .collect();

                    // Sort by score descending and take top 3
                    suggestions.sort_by(|a, b| b.0.cmp(&a.0));
                    suggestions.truncate(3);

                    if suggestions.is_empty() {
                        eprintln!("Warning: Source '{alias}' not found.");
                    } else {
                        let suggestion_list = suggestions
                            .iter()
                            .map(|(_, name)| name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        eprintln!(
                            "Warning: Source '{alias}' not found. Did you mean: {suggestion_list}?"
                        );
                    }
                    eprintln!("Run 'blz list' to see all sources.");
                }
                resolved.push(alias.clone());
            },
            Err(e) => return Err(e),
        }
    }
    Ok(resolved)
}

/// Filter out sources that aren't searchable (index-only or internal).
fn filter_searchable_sources(
    storage: &Storage,
    sources: Vec<String>,
    explicit_sources_requested: bool,
) -> Vec<String> {
    sources
        .into_iter()
        .filter(|alias| {
            match storage.load_source_metadata(alias) {
                Ok(Some(metadata)) => {
                    if metadata.is_index_only() {
                        return false;
                    }
                    if !explicit_sources_requested && metadata.is_internal() {
                        return false;
                    }
                    true
                },
                Ok(None) | Err(_) => true, // Allow search when metadata missing or failed
            }
        })
        .collect()
}

/// Enrich search hits with source metadata (URL, checksum, staleness).
fn enrich_hits_with_source_metadata(hits: &mut [SearchHit], storage: &Storage) {
    let mut metadata_cache: HashMap<String, Option<Source>> = HashMap::new();
    for hit in hits {
        let entry = metadata_cache
            .entry(hit.source.clone())
            .or_insert_with(|| storage.load_source_metadata(&hit.source).ok().flatten());
        if let Some(meta) = entry {
            hit.source_url = Some(meta.url.clone());
            hit.checksum = meta.sha256.clone();
            hit.fetched_at = Some(meta.fetched_at);
            hit.is_stale = staleness::is_stale(meta.fetched_at, DEFAULT_STALE_AFTER_DAYS);
        } else {
            hit.source_url = None;
            hit.fetched_at = None;
            hit.is_stale = false;
        }
        hit.context = None;
    }
}

pub(super) async fn perform_search(
    options: &SearchOptions,
    metrics: PerformanceMetrics,
) -> Result<SearchResults> {
    let start_time = Instant::now();
    let storage = Arc::new(Storage::new()?);

    // Resolve requested sources (supports metadata aliases)
    let explicit_sources_requested = !options.sources.is_empty();
    let sources = if explicit_sources_requested {
        resolve_requested_sources(&storage, &options.sources, options.quiet)?
    } else {
        storage.list_sources()
    };

    // Filter out index-only sources (navigation-only, no searchable content)
    let sources = filter_searchable_sources(&storage, sources, explicit_sources_requested);

    if sources.is_empty() {
        return Err(anyhow::anyhow!(
            "No sources found. Use 'blz add' to add sources."
        ));
    }

    // Execute parallel searches across all sources
    let (mut all_hits, total_lines_searched, sources_searched) =
        execute_parallel_searches(&storage, sources, options, metrics).await?;

    // Process results
    deduplicate_hits(&mut all_hits);
    sort_by_score(&mut all_hits);
    apply_percentile_filter(
        &mut all_hits,
        options.top_percentile,
        matches!(options.format, OutputFormat::Text),
    );

    // Enrich results with metadata for provenance and staleness calculations
    enrich_hits_with_source_metadata(&mut all_hits, &storage);

    // Enrich with context if requested
    let mut llms_cache: HashMap<String, Option<LlmsJson>> = HashMap::new();
    let mut line_cache: HashMap<String, Vec<String>> = HashMap::new();

    if options.block {
        enrich_hits_with_blocks(
            &mut all_hits,
            options.max_block_lines,
            &storage,
            &mut llms_cache,
            &mut line_cache,
        );
    } else if options.before_context > 0 || options.after_context > 0 {
        enrich_hits_with_context(
            &mut all_hits,
            options.before_context,
            options.after_context,
            &storage,
            &mut line_cache,
        );
    }

    let mut sources_searched = sources_searched;
    sources_searched.sort();
    Ok(SearchResults {
        hits: all_hits,
        total_lines_searched,
        search_time: start_time.elapsed(),
        sources: sources_searched,
    })
}

/// Execute parallel searches across multiple sources.
///
/// Returns a tuple of (hits, total lines searched, sources searched).
async fn execute_parallel_searches(
    storage: &Arc<Storage>,
    sources: Vec<String>,
    options: &SearchOptions,
    metrics: PerformanceMetrics,
) -> Result<(Vec<SearchHit>, usize, Vec<String>)> {
    // Calculate effective limit to prevent over-fetching
    let effective_limit = if options.all {
        ALL_RESULTS_LIMIT
    } else {
        (options.limit * 3).clamp(1, 1000)
    };

    let max_concurrent_searches = get_max_concurrent_searches();
    let snippet_limit = options.max_chars;
    let headings_only = options.headings_only;
    let show_timing = options.timing;
    let storage_for_tasks = Arc::clone(storage);
    let query = options.query.clone();

    // Create futures that spawn blocking tasks for parallel search across sources
    let search_tasks = sources.into_iter().map(move |source| {
        let storage = Arc::clone(&storage_for_tasks);
        let metrics = metrics.clone();
        let query = query.clone();

        async move {
            tokio::task::spawn_blocking(
                move || -> anyhow::Result<(Vec<SearchHit>, usize, String)> {
                    let index_path = storage.index_dir(&source)?;
                    if !index_path.exists() {
                        return Ok((Vec::new(), 0, source));
                    }

                    let index = SearchIndex::open(&index_path)
                        .with_context(|| {
                            format!(
                                "open index for source={} at {}",
                                source,
                                index_path.display()
                            )
                        })?
                        .with_metrics(metrics);

                    let hits = if headings_only {
                        index.search_headings_only_with_timing(
                            &query,
                            Some(&source),
                            effective_limit,
                            snippet_limit,
                            show_timing,
                        )
                    } else {
                        index.search_with_timing(
                            &query,
                            Some(&source),
                            effective_limit,
                            snippet_limit,
                            show_timing,
                        )
                    }
                    .with_context(|| format!("search failed for source={source}"))?;

                    // Count total lines for stats
                    let total_lines = storage
                        .load_llms_json(&source)
                        .ok()
                        .map_or(0, |json| json.line_index.total_lines);

                    Ok((hits, total_lines, source))
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("search task panicked: {e}"))?
        }
    });

    // Execute searches with bounded concurrency
    let mut search_stream = stream::iter(search_tasks).buffer_unordered(max_concurrent_searches);

    let mut all_hits = Vec::new();
    let mut total_lines_searched = 0usize;
    let mut sources_searched = Vec::new();

    // Collect results from the stream
    while let Some(res) = search_stream.next().await {
        match res {
            Ok((hits, lines, source)) => {
                let has_hits = !hits.is_empty();
                all_hits.extend(hits);
                total_lines_searched += lines;
                if lines > 0 || has_hits {
                    sources_searched.push(source);
                }
            },
            Err(e) => {
                tracing::warn!("Search failed: {}", e);
            },
        }
    }

    Ok((all_hits, total_lines_searched, sources_searched))
}

fn deduplicate_hits(hits: &mut Vec<SearchHit>) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    hits.retain(|h| seen.insert((h.source.clone(), h.lines.clone(), h.heading_path.clone())));
}

fn sort_by_score(hits: &mut [SearchHit]) {
    // Sort by score with deterministic tie-breakers
    hits.sort_by(|a, b| {
        // First by score (descending) with total ordering on floats
        match b.score.total_cmp(&a.score) {
            std::cmp::Ordering::Equal => {
                // Then by alias (ascending)
                a.source.cmp(&b.source)
                    // Then by line number (ascending)
                    .then(a.lines.cmp(&b.lines))
                    // Finally by heading path (ascending)
                    .then(a.heading_path.cmp(&b.heading_path))
            },
            ordering => ordering,
        }
    });
}

fn apply_percentile_filter(
    hits: &mut Vec<SearchHit>,
    top_percentile: Option<u8>,
    is_text_output: bool,
) {
    if let Some(percentile) = top_percentile {
        let count = percentile_count(hits.len(), percentile);
        hits.truncate(count);

        if is_text_output && hits.len() < 10 {
            eprintln!(
                "Tip: Only {} results in top {}%. Try a lower percentile or remove --top flag.",
                hits.len(),
                percentile
            );
        }
    }
}

fn enrich_hits_with_context(
    hits: &mut [SearchHit],
    before_lines: usize,
    after_lines: usize,
    storage: &Arc<Storage>,
    line_cache: &mut HashMap<String, Vec<String>>,
) {
    for hit in hits {
        let Some((base_start, base_end)) = parse_line_span(&hit.lines) else {
            continue;
        };

        if let Some(lines) = ensure_lines(line_cache, storage, &hit.source) {
            if lines.is_empty() {
                continue;
            }
            let total = lines.len();
            let start = base_start.saturating_sub(before_lines).max(1);
            let end = (base_end + after_lines).min(total);
            if start > end {
                continue;
            }
            let slice = &lines[start - 1..end];
            let content = slice.join("\n");
            let line_numbers: Vec<usize> = (start..=end).collect();
            hit.context = Some(HitContext {
                lines: format!("{start}-{end}"),
                line_numbers,
                content,
                truncated: None,
            });
        }
    }
}

fn enrich_hits_with_blocks(
    hits: &mut [SearchHit],
    max_lines: Option<usize>,
    storage: &Arc<Storage>,
    llms_cache: &mut HashMap<String, Option<LlmsJson>>,
    line_cache: &mut HashMap<String, Vec<String>>,
) {
    for hit in hits {
        let lines = match ensure_lines(line_cache, storage, &hit.source) {
            Some(lines) if !lines.is_empty() => lines,
            _ => continue,
        };

        let doc = ensure_llms(llms_cache, storage, &hit.source);
        let (start, end) = doc
            .and_then(|llms| find_heading_span(&llms.toc, &hit.heading_path))
            .or_else(|| parse_line_span(&hit.lines))
            .unwrap_or((1, 1));

        let adjusted_max = max_lines.map(|limit| limit.saturating_add(1));
        if let Some(mut block) = extract_block_slice(lines, start, end, adjusted_max) {
            if let Some(level) = heading_level_from_line(&lines[start.saturating_sub(1)]) {
                let mut inferred_end = start;
                for idx in (start + 1)..=lines.len() {
                    if let Some(next_level) = heading_level_from_line(&lines[idx - 1]) {
                        if next_level <= level {
                            break;
                        }
                    }
                    inferred_end = idx;
                }

                if inferred_end > start {
                    if let Some(extended) =
                        extract_block_slice(lines, start, inferred_end, adjusted_max)
                    {
                        block = extended;
                    }
                }
            }

            let finalized = finalize_block_slice(block);
            let render_end = finalized
                .content_line_numbers
                .last()
                .copied()
                .unwrap_or(finalized.heading_line);

            hit.context = Some(HitContext {
                lines: format!(
                    "{start}-{end}",
                    start = finalized.heading_line,
                    end = render_end
                ),
                line_numbers: finalized.content_line_numbers,
                content: finalized.content_lines.join("\n"),
                truncated: finalized.truncated.then_some(true),
            });
        }
    }
}

fn ensure_lines<'a>(
    cache: &'a mut HashMap<String, Vec<String>>,
    storage: &Arc<Storage>,
    source: &str,
) -> Option<&'a Vec<String>> {
    if !cache.contains_key(source) {
        let lines = storage
            .llms_txt_path(source)
            .ok()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .map(|content| {
                content
                    .lines()
                    .map(std::string::ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();
        cache.insert(source.to_string(), lines);
    }
    cache.get(source)
}

fn ensure_llms<'a>(
    cache: &'a mut HashMap<String, Option<LlmsJson>>,
    storage: &Arc<Storage>,
    source: &str,
) -> Option<&'a LlmsJson> {
    if !cache.contains_key(source) {
        let value = storage.load_llms_json(source).ok();
        cache.insert(source.to_string(), value);
    }
    cache.get(source).and_then(|entry| entry.as_ref())
}

/// Formats and displays search results with pagination
///
/// This function safely handles edge cases in pagination including:
/// - Empty result sets (returns without error)
/// - Single result (paginates correctly)
/// - Over-large page numbers (displays helpful message)
/// - The `actual_limit` is guaranteed to be at least 1 to prevent divide-by-zero
///
/// # Example
///
/// ```ignore
/// // Example of safe pagination with empty results
/// let results = SearchResults {
///     hits: vec![], // Empty results
///     total_lines_searched: 0,
///     search_time: Duration::from_millis(10),
///     sources: vec![],
/// };
///
/// let options = SearchOptions {
///     query: "test".to_string(),
///     sources: vec![],
///     last: false,
///     limit: 10,  // Even with limit 0, actual_limit would be max(0, 1) = 1
///     page: 1,
///     top_percentile: None,
///     format: OutputFormat::Text,
///     show_url: false,
///     show_lines: false,
///     show_anchor: false,
///     show_raw_score: false,
///     no_summary: false,
///     score_precision: None,
///     snippet_lines: 3,
///     all: false,
/// };
///
/// // This will not panic even with empty results due to .max(1) guard
/// assert!(format_and_display(&results, &options).is_ok());
/// ```
pub(super) fn format_and_display(
    results: &SearchResults,
    options: &SearchOptions,
) -> Result<((usize, usize, usize), usize)> {
    let total_results = results.hits.len();
    let mut suggestion_resolver = SuggestionResolver::new(options, results);

    // Apply pagination
    let actual_limit = if options.limit >= ALL_RESULTS_LIMIT {
        results.hits.len().max(1)
    } else {
        options.limit.max(1)
    };

    let total_pages = if total_results == 0 {
        0
    } else {
        total_results.div_ceil(actual_limit)
    };
    let requested_page = if options.last {
        total_pages.max(1)
    } else {
        options.page
    };

    // Handle empty results
    if total_results == 0 {
        let suggestions = suggestion_resolver.resolve(true);
        let page_ctx = PageContext {
            hits: &[],
            start_idx: 0,
            page: 0,
            page_size: actual_limit,
            total_pages,
            suggestions,
        };
        format_page(&page_ctx, results, options)?;
        return Ok(((0, actual_limit, total_pages), total_results));
    }

    let page = requested_page.clamp(1, total_pages);
    let start_idx = (page - 1) * actual_limit;
    let end_idx = (start_idx + actual_limit).min(results.hits.len());

    // Handle out-of-range page
    if start_idx >= results.hits.len() {
        if matches!(options.format, OutputFormat::Text) {
            eprintln!(
                "Page {} is beyond available results (Page {} of {})",
                options.page, page, total_pages
            );
            eprintln!("Tip: use --last to jump to the final page.");
        }
        let suggestions = suggestion_resolver.resolve(true);
        let page_ctx = PageContext {
            hits: &[],
            start_idx: start_idx.min(results.hits.len()),
            page,
            page_size: actual_limit,
            total_pages,
            suggestions,
        };
        format_page(&page_ctx, results, options)?;
        return Ok(((page, actual_limit, total_pages), total_results));
    }

    // Normal case: format the current page
    let page_hits = &results.hits[start_idx..end_idx];
    let need_suggest = results.hits.first().map_or(0.0, |h| h.score) < 2.0;
    let suggestions = suggestion_resolver.resolve(need_suggest);
    let page_ctx = PageContext {
        hits: page_hits,
        start_idx,
        page,
        page_size: actual_limit,
        total_pages,
        suggestions,
    };
    format_page(&page_ctx, results, options)?;

    Ok(((page, actual_limit, total_pages), total_results))
}

/// Helper to lazily resolve suggestions for JSON output.
struct SuggestionResolver<'a> {
    options: &'a SearchOptions,
    results: &'a SearchResults,
    storage_cache: Option<Storage>,
}

impl<'a> SuggestionResolver<'a> {
    const fn new(options: &'a SearchOptions, results: &'a SearchResults) -> Self {
        Self {
            options,
            results,
            storage_cache: None,
        }
    }

    fn resolve(&mut self, need: bool) -> Option<Vec<serde_json::Value>> {
        if !matches!(self.options.format, OutputFormat::Json) || !need {
            return None;
        }
        if self.storage_cache.is_none() {
            self.storage_cache = match Storage::new() {
                Ok(storage) => Some(storage),
                Err(err) => {
                    warn!("suggestions disabled: failed to open storage: {err}");
                    None
                },
            };
        }
        self.storage_cache
            .as_ref()
            .map(|storage| compute_suggestions(&self.options.query, storage, &self.results.sources))
    }
}

/// Pagination context for formatting a page of results.
struct PageContext<'a> {
    /// Hits to display on this page.
    hits: &'a [SearchHit],
    /// Zero-based index of the first hit on this page.
    start_idx: usize,
    /// Current page number (1-based).
    page: usize,
    /// Results per page.
    page_size: usize,
    /// Total pages available.
    total_pages: usize,
    /// Optional fuzzy suggestions.
    suggestions: Option<Vec<serde_json::Value>>,
}

/// Format and display a page of search results.
fn format_page(
    page_ctx: &PageContext<'_>,
    results: &SearchResults,
    options: &SearchOptions,
) -> Result<()> {
    let formatter = SearchResultFormatter::new(options.format);
    let params = FormatParams {
        hits: page_ctx.hits,
        query: &options.query,
        total_results: results.hits.len(),
        total_lines_searched: results.total_lines_searched,
        search_time: results.search_time,
        sources: &results.sources,
        start_idx: page_ctx.start_idx,
        page: page_ctx.page,
        total_pages: page_ctx.total_pages,
        page_size: page_ctx.page_size,
        show_url: options.show_url,
        show_lines: options.show_lines,
        show_anchor: options.show_anchor,
        show_raw_score: options.show_raw_score,
        no_summary: options.no_summary,
        score_precision: options.score_precision.unwrap_or(DEFAULT_SCORE_PRECISION),
        snippet_lines: usize::from(options.snippet_lines.max(1)),
        suggestions: page_ctx.suggestions.clone(),
    };
    formatter.format(&params)
}

fn compute_suggestions(
    query: &str,
    storage: &blz_core::Storage,
    sources: &[String],
) -> Vec<serde_json::Value> {
    // Tokenize query (lowercase alphanumeric words)
    let qtokens = tokenize(query);
    if qtokens.is_empty() {
        return Vec::new();
    }

    let mut suggestions: Vec<(f32, String, String, String)> = Vec::new(); // (score, alias, heading, lines)
    for alias in sources {
        if let Ok(doc) = storage.load_llms_json(alias) {
            collect_suggestions_from_toc(&doc, alias, &qtokens, &mut suggestions);
        }
    }
    // Sort by score desc and take top 5
    suggestions.sort_by(|a, b| b.0.total_cmp(&a.0));
    suggestions.truncate(5);
    suggestions
        .into_iter()
        .map(|(score, alias, heading, lines)| {
            serde_json::json!({
                "alias": alias,
                "heading": heading,
                "lines": lines,
                "score": score,
            })
        })
        .collect()
}

fn collect_suggestions_from_toc(
    doc: &blz_core::LlmsJson,
    alias: &str,
    qtokens: &[String],
    out: &mut Vec<(f32, String, String, String)>,
) {
    fn walk(
        list: &[blz_core::TocEntry],
        alias: &str,
        qtokens: &[String],
        out: &mut Vec<(f32, String, String, String)>,
    ) {
        for e in list {
            let display_path = e.heading_path_display.as_ref().unwrap_or(&e.heading_path);
            if let Some(name) = display_path.last() {
                let score = score_tokens(&tokenize(name), qtokens);
                if score > 0.2 {
                    out.push((score, alias.to_string(), name.clone(), e.lines.clone()));
                }
            }
            if !e.children.is_empty() {
                walk(&e.children, alias, qtokens, out);
            }
        }
    }
    walk(&doc.toc, alias, qtokens, out);
}

fn tokenize(s: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            toks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

#[allow(clippy::cast_precision_loss)]
fn score_tokens(h: &[String], q: &[String]) -> f32 {
    if h.is_empty() || q.is_empty() {
        return 0.0;
    }
    let hset: std::collections::BTreeSet<&str> =
        h.iter().map(std::string::String::as_str).collect();
    let qset: std::collections::BTreeSet<&str> =
        q.iter().map(std::string::String::as_str).collect();
    let inter = hset.intersection(&qset).count() as f32;
    inter / (qset.len() as f32)
}

// alias resolution moved to utils::resolver

/// Copy search results to clipboard using OSC 52
pub(super) fn copy_results_to_clipboard(
    results: &SearchResults,
    page: usize,
    page_size: usize,
) -> Result<()> {
    use crate::utils::clipboard;

    // Calculate which hits are on the current page
    let start_idx = (page - 1) * page_size;
    let end_idx = (start_idx + page_size).min(results.hits.len());
    let page_hits = &results.hits[start_idx..end_idx];

    // Build clipboard content with source, heading, and snippet for each hit
    let mut content = String::new();
    for hit in page_hits {
        use std::fmt::Write;
        writeln!(
            content,
            "# {} > {}",
            hit.source,
            hit.heading_path.join(" > ")
        )
        .ok();
        writeln!(content, "{}\n", hit.snippet).ok();
    }

    // Trim trailing whitespace
    let content = content.trim_end();

    clipboard::copy_to_clipboard(content).context("Failed to copy results to clipboard")?;

    Ok(())
}

// ============================================================================
// Dispatch and Handler Functions (moved from lib.rs)
// ============================================================================

use crate::config::{
    ContentConfig, DisplayConfig, QueryExecutionConfig, SearchConfig, SnippetConfig,
};
use crate::utils::heading_filter::HeadingLevelFilter;

const DEFAULT_SNIPPET_LINES: u8 = 3;

/// Parse heading level filter from string.
fn parse_heading_filter(filter_str: Option<&str>) -> Result<Option<HeadingLevelFilter>> {
    filter_str
        .map(|s| {
            s.parse::<HeadingLevelFilter>()
                .map_err(|e| anyhow::anyhow!("Invalid heading level filter: {e}"))
        })
        .transpose()
}

/// Dispatch a Search command variant, handling destructuring internally.
///
/// This function extracts all fields from the `Commands::Search` variant,
/// resolves history-based pagination, builds config structs, and delegates
/// to `execute()`.
#[allow(clippy::too_many_lines)]
pub async fn dispatch(
    cmd: Commands,
    quiet: bool,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    const DEFAULT_LIMIT: usize = 50;

    let Commands::Search(args) = cmd else {
        unreachable!("dispatch called with non-Search command");
    };

    let resolved_format = args.format.resolve(quiet);
    let merged_context = merge_context_flags(
        args.context,
        args.context_deprecated,
        args.after_context,
        args.before_context,
    );

    let provided_query = args.query.is_some();
    let limit_was_explicit = args.all || args.limit.is_some();
    let mut use_headings_only = args.headings_only;

    // Emit deprecation warning if --snippet-lines was explicitly set
    if args.snippet_lines != DEFAULT_SNIPPET_LINES {
        let cli_args: Vec<String> = std::env::args().collect();
        if flag_present(&cli_args, "--snippet-lines") || std::env::var("BLZ_SNIPPET_LINES").is_ok()
        {
            crate::utils::cli_args::emit_snippet_lines_deprecation(false);
        }
    }

    if args.next {
        validate_continuation_flag(
            "--next",
            provided_query,
            &args.sources,
            args.page,
            args.last,
        )?;
    }

    if args.previous {
        validate_continuation_flag(
            "--previous",
            provided_query,
            &args.sources,
            args.page,
            args.last,
        )?;
    }

    let history_entry = if args.next || args.previous || !provided_query {
        let mut records = history_log::recent_for_active_scope(1);
        if records.is_empty() {
            anyhow::bail!("No previous search found. Use 'blz search <query>' first.");
        }
        Some(records.remove(0))
    } else {
        None
    };

    if let Some(entry) = history_entry.as_ref() {
        if (args.next || args.previous) && args.headings_only != entry.headings_only {
            anyhow::bail!(
                "Cannot change --headings-only while using --next/--previous. Rerun without continuation flags."
            );
        }
        if !args.headings_only {
            use_headings_only = entry.headings_only;
        }
    }

    let actual_query = resolve_query(args.query, history_entry.as_ref())?;
    let actual_sources = resolve_sources(args.sources, history_entry.as_ref());

    let base_limit = if args.all {
        ALL_RESULTS_LIMIT
    } else {
        args.limit.unwrap_or(DEFAULT_LIMIT)
    };
    let actual_max_chars = args.max_chars.map_or(DEFAULT_MAX_CHARS, clamp_max_chars);

    let (actual_page, actual_limit) = if let Some(entry) = history_entry.as_ref() {
        let ctx = PaginationContext {
            entry,
            next: args.next,
            previous: args.previous,
            provided_query,
            all: args.all,
            limit: args.limit,
            limit_was_explicit,
            current_page: args.page,
            current_limit: base_limit,
            all_results_limit: ALL_RESULTS_LIMIT,
        };
        let adj = apply_history_pagination(&ctx)?;
        (adj.page, adj.limit)
    } else {
        (args.page, base_limit)
    };

    // Parse heading filter
    let heading_filter = parse_heading_filter(args.heading_level.as_deref())?;

    // Build config structs
    let search_config = SearchConfig::new()
        .with_limit(actual_limit)
        .with_page(actual_page)
        .with_top_percentile(args.top)
        .with_heading_filter(heading_filter)
        .with_headings_only(use_headings_only)
        .with_last(args.last)
        .with_no_history(args.no_history);

    let display_config = DisplayConfig::new(resolved_format)
        .with_show(args.show)
        .with_no_summary(args.no_summary)
        .with_timing(args.timing)
        .with_quiet(quiet);

    let snippet_config = SnippetConfig::new()
        .with_lines(args.snippet_lines)
        .with_max_chars(actual_max_chars)
        .with_score_precision(args.score_precision);

    let content_config = ContentConfig::new()
        .with_context(merged_context)
        .with_max_lines(args.max_lines)
        .with_copy(args.copy)
        .with_block(args.block);

    let config = QueryExecutionConfig::new(
        search_config,
        display_config,
        snippet_config,
        content_config,
    );

    execute(
        &actual_query,
        &actual_sources,
        &config,
        Some(prefs),
        metrics,
        None,
        true, // emit deprecation warning - this is the deprecated `blz search` command
    )
    .await
}

/// Pagination adjustments computed from history and continuation flags.
struct PaginationAdjustment {
    page: usize,
    limit: usize,
}

/// Pagination context for history-based adjustments.
///
/// This struct bundles pagination parameters to reduce function argument counts.
/// The bools represent distinct user-specified flags from the CLI.
#[allow(clippy::struct_excessive_bools)]
struct PaginationContext<'a> {
    entry: &'a SearchHistoryEntry,
    next: bool,
    previous: bool,
    provided_query: bool,
    all: bool,
    limit: Option<usize>,
    limit_was_explicit: bool,
    current_page: usize,
    current_limit: usize,
    all_results_limit: usize,
}

/// Apply history-based pagination adjustments for --next/--previous flags.
///
/// Returns adjusted page and limit values based on history entry and continuation mode.
fn apply_history_pagination(ctx: &PaginationContext<'_>) -> Result<PaginationAdjustment> {
    let mut actual_page = ctx.current_page;
    let mut actual_limit = ctx.current_limit;

    if ctx.next {
        validate_history_has_results(ctx.entry)?;
        validate_pagination_limit_consistency(
            "--next",
            ctx.all,
            ctx.limit_was_explicit,
            ctx.limit,
            ctx.entry.limit,
            ctx.all_results_limit,
        )?;

        if let (Some(prev_page), Some(total_pages)) = (ctx.entry.page, ctx.entry.total_pages) {
            if prev_page >= total_pages {
                anyhow::bail!("Already at the last page (page {prev_page} of {total_pages})");
            }
            actual_page = prev_page + 1;
        } else {
            actual_page = ctx.entry.page.unwrap_or(1) + 1;
        }

        if !ctx.limit_was_explicit {
            actual_limit = ctx.entry.limit.unwrap_or(actual_limit);
        }
    } else if ctx.previous {
        validate_history_has_results(ctx.entry)?;
        validate_pagination_limit_consistency(
            "--previous",
            ctx.all,
            ctx.limit_was_explicit,
            ctx.limit,
            ctx.entry.limit,
            ctx.all_results_limit,
        )?;

        if let Some(prev_page) = ctx.entry.page {
            if prev_page <= 1 {
                anyhow::bail!("Already on first page");
            }
            actual_page = prev_page - 1;
        } else {
            anyhow::bail!("No previous page found in search history");
        }

        if !ctx.limit_was_explicit {
            actual_limit = ctx.entry.limit.unwrap_or(actual_limit);
        }
    } else if !ctx.provided_query && !ctx.limit_was_explicit {
        actual_limit = ctx.entry.limit.unwrap_or(actual_limit);
    }

    Ok(PaginationAdjustment {
        page: actual_page,
        limit: actual_limit,
    })
}

/// Validate that history entry has non-zero results.
fn validate_history_has_results(entry: &SearchHistoryEntry) -> Result<()> {
    if matches!(entry.total_pages, Some(0)) || matches!(entry.total_results, Some(0)) {
        anyhow::bail!(
            "Previous search returned 0 results. Rerun with a different query or source."
        );
    }
    Ok(())
}

/// Resolve the search query from explicit input or history.
fn resolve_query(
    mut query: Option<String>,
    history: Option<&SearchHistoryEntry>,
) -> Result<String> {
    if let Some(value) = query.take() {
        Ok(value)
    } else if let Some(entry) = history {
        Ok(entry.query.clone())
    } else {
        anyhow::bail!("No previous search found. Use 'blz search <query>' first.");
    }
}

/// Resolve source filters from explicit input or history.
fn resolve_sources(sources: Vec<String>, history: Option<&SearchHistoryEntry>) -> Vec<String> {
    if !sources.is_empty() {
        sources
    } else if let Some(entry) = history {
        entry.source.as_ref().map_or_else(Vec::new, |source_str| {
            source_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        })
    } else {
        Vec::new()
    }
}

/// Validate that pagination limit hasn't changed when using continuation flags.
fn validate_pagination_limit_consistency(
    flag: &str,
    all: bool,
    limit_was_explicit: bool,
    limit: Option<usize>,
    history_limit: Option<usize>,
    all_results_limit: usize,
) -> Result<()> {
    let history_all = history_limit.is_some_and(|value| value >= all_results_limit);
    if all != history_all {
        anyhow::bail!(
            "Cannot use {flag} when changing page size or --all; rerun without {flag} or reuse the previous pagination flags."
        );
    }
    if limit_was_explicit {
        if let Some(requested_limit) = limit {
            if history_limit != Some(requested_limit) {
                anyhow::bail!(
                    "Cannot use {flag} when changing page size; rerun without {flag} or reuse the previous limit."
                );
            }
        }
    }
    Ok(())
}

/// Validate that a continuation flag (--next/--previous) is not combined with incompatible options.
fn validate_continuation_flag(
    flag: &str,
    provided_query: bool,
    sources: &[String],
    page: usize,
    last: bool,
) -> Result<()> {
    if provided_query {
        anyhow::bail!(
            "Cannot combine {flag} with an explicit query. Remove the query to continue from the previous search."
        );
    }
    if !sources.is_empty() {
        anyhow::bail!(
            "Cannot combine {flag} with --source. Omit --source to reuse the last search context."
        );
    }
    if page != 1 {
        anyhow::bail!("Cannot combine {flag} with --page. Use one pagination option at a time.");
    }
    if last {
        anyhow::bail!("Cannot combine {flag} with --last. Choose a single continuation flag.");
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)] // Test code precision is not critical
mod tests {
    use super::*;
    use blz_core::SearchHit;
    use chrono::Utc;

    /// Creates a test `SearchResults` with the specified number of hits
    fn create_test_results(num_hits: usize) -> SearchResults {
        let hits: Vec<SearchHit> = (0..num_hits)
            .map(|i| SearchHit {
                source: format!("test-{i}"),
                file: "llms.txt".to_string(),
                heading_path: vec![format!("heading-{i}")],
                raw_heading_path: Some(vec![format!("heading-{i}")]),
                level: 1,
                lines: format!("{start}-{end}", start = i * 10, end = i * 10 + 5),
                line_numbers: Some(vec![i * 10, i * 10 + 5]),
                snippet: format!("test content {i}"),
                score: (i as f32).mul_add(-0.01, 1.0),
                source_url: Some(format!("https://example.com/test-{i}")),
                fetched_at: Some(Utc::now()),
                is_stale: false,
                checksum: format!("checksum-{i}"),
                anchor: Some("unit-test-anchor".to_string()),
                context: None,
            })
            .collect();

        SearchResults {
            hits,
            total_lines_searched: 1000,
            search_time: std::time::Duration::from_millis(10),
            sources: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_pagination_with_zero_hits() {
        // Test that pagination handles empty results without panic
        let results = create_test_results(0);
        let options = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        // Should not panic even with empty results
        let result = format_and_display(&results, &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clamp_max_chars_bounds() {
        assert_eq!(clamp_max_chars(10), MIN_SNIPPET_CHAR_LIMIT);
        assert_eq!(clamp_max_chars(DEFAULT_MAX_CHARS), DEFAULT_MAX_CHARS);
        assert_eq!(clamp_max_chars(10_000), MAX_SNIPPET_CHAR_LIMIT);
    }

    #[test]
    fn test_pagination_with_single_hit() {
        // Test edge case where there's only one result
        let results = create_test_results(1);
        let options = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pagination_prevents_divide_by_zero() {
        // This is the main regression test for the divide-by-zero fix
        // Test case 1: Empty results with ALL_RESULTS_LIMIT
        let empty_results = create_test_results(0);
        let options_empty = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 2, // Try to access page 2 to trigger div_ceil
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: true,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        // This should NOT panic even with empty results
        let result = format_and_display(&empty_results, &options_empty);
        assert!(result.is_ok(), "Should handle empty results without panic");

        // Test case 2: Normal results with high page number
        let results = create_test_results(5);
        let options_high_page = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 100, // Very high page to trigger the div_ceil in the message
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: true,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let result = format_and_display(&results, &options_high_page);
        assert!(
            result.is_ok(),
            "Should handle page out of bounds without panic"
        );
    }

    #[test]
    fn test_pagination_with_overlarge_page_number() {
        // Test that requesting a page beyond available results is handled gracefully
        let results = create_test_results(5);
        let options = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 2,
            page: 100, // Way beyond available pages
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok()); // Should handle gracefully, not panic
    }

    #[test]
    fn test_pagination_boundary_conditions() {
        // Test exact boundary conditions
        let results = create_test_results(10);

        // Exactly at the boundary (page 2 with limit 5 for 10 results)
        let options = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 5,
            page: 2,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok());

        // Just beyond the boundary (page 3 with limit 5 for 10 results)
        let options_beyond = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 5,
            page: 3,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let test_results = create_test_results(10);
        let result_beyond = format_and_display(&test_results, &options_beyond);
        assert!(result_beyond.is_ok());
    }

    #[test]
    fn test_actual_limit_calculation() {
        // Test that actual_limit is always at least 1
        // This directly tests the fix for the divide-by-zero issue

        // Case 1: Normal limit
        let options1 = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: false,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let results1 = create_test_results(8);
        let actual_limit1 = if options1.limit >= ALL_RESULTS_LIMIT {
            results1.hits.len().max(1)
        } else {
            options1.limit.max(1)
        };
        assert!(actual_limit1 >= 1, "actual_limit must be at least 1");

        // Case 2: ALL_RESULTS_LIMIT with empty results
        let options2 = SearchOptions {
            query: "test".to_string(),
            sources: vec![],
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 1,
            top_percentile: None,
            format: OutputFormat::Text,
            show_url: false,
            show_lines: false,
            show_anchor: false,
            show_raw_score: false,
            no_summary: false,
            score_precision: None,
            snippet_lines: 3,
            all: true,
            no_history: false,
            copy: false,
            before_context: 0,
            after_context: 0,
            block: false,
            max_block_lines: None,
            max_chars: DEFAULT_MAX_CHARS,
            quiet: false,
            headings_only: false,
            timing: false,
        };

        let results2 = create_test_results(0);
        let actual_limit2 = if options2.limit >= ALL_RESULTS_LIMIT {
            results2.hits.len().max(1)
        } else {
            options2.limit.max(1)
        };
        assert!(
            actual_limit2 >= 1,
            "actual_limit must be at least 1 even with empty results"
        );
    }

    #[test]
    fn test_div_ceil_safety() {
        // Ensure div_ceil never receives 0 as divisor
        let test_cases = vec![
            (0_usize, 1_usize),    // 0 results
            (1_usize, 1_usize),    // 1 result
            (5_usize, 2_usize),    // 5 results with limit 2
            (10_usize, 10_usize),  // 10 results with limit 10
            (100_usize, 25_usize), // 100 results with limit 25
        ];

        for (total_results, limit) in test_cases {
            // Simulating the actual_limit calculation with the fix
            let actual_limit = limit.max(1);

            // This is what was causing the panic before the fix
            let pages = total_results.div_ceil(actual_limit);

            // Verify it doesn't panic and produces sensible results
            if total_results == 0 {
                assert_eq!(pages, 0);
            } else {
                assert!(pages >= 1);
            }
        }
    }

    // Tests for fuzzy matching warning feature (BLZ-154)

    #[test]
    fn test_fuzzy_matcher_finds_close_matches() {
        // Test that fuzzy matching correctly identifies similar sources
        // NOTE: SkimMatcherV2 is case-sensitive and uses subsequence matching
        let matcher = SkimMatcherV2::default();
        let known_sources = ["react".to_string(), "vue".to_string(), "node".to_string()];
        let typo = "reac"; // Close match for "react" (substring)

        let mut suggestions: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, typo)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        suggestions.sort_by(|a, b| b.0.cmp(&a.0));

        // Should find "react" as the best match
        assert!(
            !suggestions.is_empty(),
            "Should find at least one suggestion"
        );
        assert_eq!(suggestions[0].1, "react", "Best match should be 'react'");
    }

    #[test]
    fn test_fuzzy_matcher_no_matches_for_unrelated_term() {
        // Test that completely unrelated terms don't match
        let matcher = SkimMatcherV2::default();
        let known_sources = ["react".to_string(), "vue".to_string()];
        let unrelated = "xyz123";

        let suggestions: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, unrelated)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        // Should not find any good matches
        assert!(
            suggestions.is_empty(),
            "Should not find suggestions for completely unrelated term"
        );
    }

    #[test]
    fn test_fuzzy_matcher_limits_to_top_three() {
        // Test that we only show top 3 suggestions even with many matches
        let matcher = SkimMatcherV2::default();
        let known_sources = [
            "react-dom".to_string(),
            "react-native".to_string(),
            "react-router".to_string(),
            "react-query".to_string(),
            "react-hook-form".to_string(),
            "preact".to_string(),
        ];
        let query = "react";

        let mut suggestions: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, query)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        suggestions.sort_by(|a, b| b.0.cmp(&a.0));
        suggestions.truncate(3);

        // Should limit to 3 suggestions
        assert_eq!(
            suggestions.len(),
            3,
            "Should limit to top 3 suggestions even when more match"
        );

        // Should be sorted by score (all should have scores)
        assert!(
            suggestions[0].0 > 0,
            "First suggestion should have positive score"
        );
        assert!(
            suggestions[1].0 > 0,
            "Second suggestion should have positive score"
        );
        assert!(
            suggestions[2].0 > 0,
            "Third suggestion should have positive score"
        );
    }

    #[test]
    fn test_fuzzy_matcher_suggestions_are_sorted() {
        // Test that suggestions are sorted by match quality
        let matcher = SkimMatcherV2::default();
        let known_sources = [
            "javascript".to_string(),
            "java".to_string(),
            "typescript".to_string(),
        ];
        let query = "java";

        let mut suggestions: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, query)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        suggestions.sort_by(|a, b| b.0.cmp(&a.0));

        // Should find matches (both "java" and "javascript" contain "java")
        assert!(!suggestions.is_empty(), "Should find at least one match");
        // Note: The fuzzy matcher may rank "javascript" higher than "java" because it's a longer match
        // What matters is that matches are found and sorted by score

        // Scores should be in descending order
        for i in 0..suggestions.len().saturating_sub(1) {
            assert!(
                suggestions[i].0 >= suggestions[i + 1].0,
                "Suggestions should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_fuzzy_matcher_case_sensitivity() {
        // Test fuzzy matching behavior with different cases
        // SkimMatcherV2 performs case-insensitive matching by default
        let matcher = SkimMatcherV2::default();
        let known_sources = ["React".to_string(), "Vue".to_string()];

        let lowercase_query = "reac"; // Substring match
        let suggestions_lower: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, lowercase_query)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        // Should match "React" even with lowercase query
        assert!(
            !suggestions_lower.is_empty(),
            "Should match with lowercase query"
        );
        assert_eq!(suggestions_lower[0].1, "React");
    }

    #[test]
    fn test_fuzzy_matcher_handles_typos() {
        // Test common typo patterns
        let matcher = SkimMatcherV2::default();
        let known_sources = [
            "typescript".to_string(),
            "javascript".to_string(),
            "python".to_string(),
        ];

        // Transposition typo: "typscript" -> "typescript"
        let typo1 = "typscript";
        let suggestions1: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, typo1)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        assert!(
            !suggestions1.is_empty(),
            "Should handle transposition typos"
        );
        assert_eq!(suggestions1[0].1, "typescript");

        // Missing letter: "pythn" -> "python"
        let typo2 = "pythn";
        let suggestions2: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, typo2)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        assert!(
            !suggestions2.is_empty(),
            "Should handle missing letter typos"
        );
        assert_eq!(suggestions2[0].1, "python");
    }

    #[test]
    fn test_fuzzy_matcher_partial_matches() {
        // Test that partial matches work (prefix, suffix, infix)
        let matcher = SkimMatcherV2::default();
        let known_sources = [
            "react-native".to_string(),
            "vue-router".to_string(),
            "angular".to_string(),
        ];

        // Prefix match
        let prefix = "react";
        let suggestions_prefix: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, prefix)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        assert!(!suggestions_prefix.is_empty(), "Should match prefix");
        assert_eq!(suggestions_prefix[0].1, "react-native");

        // Suffix match
        let suffix = "router";
        let suggestions_suffix: Vec<(i64, String)> = known_sources
            .iter()
            .filter_map(|source| {
                matcher
                    .fuzzy_match(source, suffix)
                    .filter(|&score| score > 0)
                    .map(|score| (score, source.clone()))
            })
            .collect();

        assert!(!suggestions_suffix.is_empty(), "Should match suffix");
        assert_eq!(suggestions_suffix[0].1, "vue-router");
    }
}
