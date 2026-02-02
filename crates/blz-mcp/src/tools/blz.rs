//! Unified BLZ tool for source management and metadata actions.

use std::fs;

use blz_core::refresh::{
    DefaultRefreshIndexer, RefreshContext, RefreshOutcome, RefreshStorage,
    refresh_source_with_metadata, reindex_source, resolve_refresh_url,
};
use blz_core::{
    CacheInfo, Fetcher, HeadingFilterStats, HealthCheck, HealthReport, HealthStatus,
    PerformanceMetrics, Registry, SourceHealth, SourceHealthEntry, SourceKind, Storage, TocEntry,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    cache,
    error::{McpError, McpResult},
    types::IndexCache,
};

use super::{
    learn_blz::{LearnBlzParams, handle_learn_blz},
    run_command::{RunCommandOutput, RunCommandParams, handle_run_command},
    sources::{
        ListSourcesOutput, ListSourcesParams, SourceAddOutput, SourceAddParams,
        handle_list_sources, handle_source_add,
    },
};

/// Parameters for blz tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlzParams {
    /// Action to execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<BlzAction>,

    /// Source alias (for add/remove/refresh/info/validate/history)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// URL override (for add)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Force override if source exists (for add)
    #[serde(default)]
    pub force: bool,

    /// Filter for list (installed/registry/all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Query filter for list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Re-index instead of fetching (for refresh)
    #[serde(default)]
    pub reindex: bool,

    /// Refresh all sources
    #[serde(default)]
    pub all: bool,

    /// Maximum results for lookup (default: 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Target alias for addAlias/removeAlias actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_alias: Option<String>,
}

/// Supported blz actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlzAction {
    /// List sources (installed/registry/all)
    List,
    /// Add a source to the cache
    Add,
    /// Remove a source and cached data
    Remove,
    /// Refresh cached sources
    Refresh,
    /// Show detailed info for a source
    Info,
    /// Validate source data integrity
    Validate,
    /// Show archive history for a source
    History,
    /// Search the registry for sources matching a query
    Lookup,
    /// Run health checks and diagnostics
    Doctor,
    /// Clear the entire cache
    ClearCache,
    /// Add an alias to a source
    AddAlias,
    /// Remove an alias from a source
    RemoveAlias,
    /// Return help and usage guidance
    Help,
}

/// Output from blz tool
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlzOutput {
    /// Action that was executed
    pub action: BlzAction,

    /// List output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<ListSourcesOutput>,

    /// Add output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<SourceAddOutput>,

    /// Remove output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<RemoveOutput>,

    /// Refresh output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<RefreshSummary>,

    /// Info output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<SourceInfoOutput>,

    /// Validate output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<RunCommandOutput>,

    /// History output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<RunCommandOutput>,

    /// Lookup output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup: Option<LookupOutput>,

    /// Doctor output (health check results)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doctor: Option<HealthReport>,

    /// Clear cache output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear: Option<ClearCacheOutput>,

    /// Alias add output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_add: Option<AliasOutput>,

    /// Alias remove output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_remove: Option<AliasOutput>,

    /// Help output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<Value>,
}

/// Summary information for removals
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveOutput {
    /// Alias that was removed
    pub alias: String,
    /// Human-readable removal summary
    pub message: String,
    /// Optional removal metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<RemovalInfo>,
}

/// Removal metadata
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalInfo {
    /// Canonical alias removed
    pub alias: String,
    /// Source URL
    pub url: String,
    /// Total line count at removal time
    pub total_lines: usize,
    /// Last fetched timestamp (RFC3339)
    pub fetched_at: String,
}

/// Detailed source info output
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfoOutput {
    /// Canonical source alias
    pub alias: String,
    /// Source URL
    pub url: String,
    /// Variant (llms, llms-full, custom)
    pub variant: String,
    /// Additional alias names
    pub aliases: Vec<String>,
    /// Total lines in cached content
    pub lines: usize,
    /// Total headings in the TOC
    pub headings: usize,
    /// Size of cached content in bytes
    pub size_bytes: u64,
    /// Last updated timestamp (RFC3339)
    pub last_updated: Option<String>,
    /// `ETag` value if present
    pub etag: Option<String>,
    /// SHA256 checksum if available
    pub checksum: Option<String>,
    /// Local cache directory path
    pub cache_path: String,
    /// Language filter statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_stats: Option<HeadingFilterStats>,
}

/// Registry lookup results
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupOutput {
    /// Query that was searched
    pub query: String,
    /// Matching entries
    pub results: Vec<LookupResult>,
    /// Total results found
    pub total: usize,
}

/// Single registry lookup result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupResult {
    /// Source name
    pub name: String,
    /// Source slug/alias
    pub slug: String,
    /// Description
    pub description: String,
    /// Primary URL
    pub url: String,
    /// Match score (higher = better)
    pub score: i64,
    /// Which field matched (name, slug, alias, description)
    pub match_field: String,
}

/// Output from cache clear action
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCacheOutput {
    /// Human-readable message
    pub message: String,
    /// Number of sources that were cleared
    pub cleared: usize,
    /// List of source aliases that were removed
    pub sources: Vec<String>,
}

/// Output from alias add/remove actions
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasOutput {
    /// Human-readable message
    pub message: String,
    /// The source that was modified
    pub source: String,
    /// The alias that was added or removed
    pub alias: String,
    /// Whether the operation was a no-op (alias already exists/doesn't exist)
    pub no_op: bool,
    /// Current list of aliases for the source after the operation
    pub current_aliases: Vec<String>,
}

/// Refresh summary for one or more sources
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSummary {
    /// Per-source refresh results
    pub results: Vec<RefreshResult>,
    /// Count of refreshed sources
    pub refreshed: usize,
    /// Count of unchanged sources
    pub unchanged: usize,
    /// Count of reindexed sources
    pub reindexed: usize,
    /// Count of failures
    pub errors: usize,
}

/// Per-source refresh result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    /// Source alias
    pub alias: String,
    /// Refresh status
    pub status: RefreshStatus,
    /// Heading count (refresh only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headings: Option<usize>,
    /// Line count (refresh only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
    /// Heading count before reindex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headings_before: Option<usize>,
    /// Heading count after reindex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headings_after: Option<usize>,
    /// Number of headings filtered during reindex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered: Option<usize>,
    /// Error message if refresh failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Refresh status values
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshStatus {
    /// Source content refreshed and re-indexed
    Refreshed,
    /// Source content unchanged
    Unchanged,
    /// Source reindexed without fetching
    Reindexed,
    /// Refresh failed
    Error,
}

const fn empty_output(action: BlzAction) -> BlzOutput {
    BlzOutput {
        action,
        list: None,
        add: None,
        remove: None,
        refresh: None,
        info: None,
        validate: None,
        history: None,
        lookup: None,
        doctor: None,
        clear: None,
        alias_add: None,
        alias_remove: None,
        help: None,
    }
}

const fn resolve_action(params: &BlzParams) -> BlzAction {
    if let Some(action) = params.action {
        return action;
    }

    if params.url.is_some() || params.force {
        return BlzAction::Add;
    }

    if params.reindex || params.all {
        return BlzAction::Refresh;
    }

    if params.alias.is_some() {
        return BlzAction::Info;
    }

    BlzAction::List
}

fn resolve_alias(storage: &Storage, requested: &str) -> McpResult<Option<String>> {
    let requested_str = requested.to_string();
    let known = storage.list_sources();

    if known.iter().any(|alias| alias == requested) {
        return Ok(Some(requested_str));
    }

    let mut resolved_sources = Vec::new();
    for source in &known {
        if let Ok(Some(metadata)) = storage.load_source_metadata(source) {
            if metadata.aliases.iter().any(|alias| alias == requested) {
                resolved_sources.push(source.clone());
                continue;
            }
        }

        if let Ok(llms) = storage.load_llms_json(source) {
            if llms.metadata.aliases.iter().any(|alias| alias == requested) {
                resolved_sources.push(source.clone());
            }
        }
    }

    match resolved_sources.len() {
        0 => Ok(None),
        1 => Ok(resolved_sources.pop()),
        _ => Err(McpError::InvalidParams(format!(
            "Alias '{}' is ambiguous across multiple sources: {}",
            requested,
            resolved_sources.join(", ")
        ))),
    }
}

fn resolve_required_alias(
    storage: &Storage,
    alias: Option<String>,
    action: BlzAction,
) -> McpResult<String> {
    let alias = alias.ok_or_else(|| McpError::MissingParameter("alias".to_string()))?;
    let resolved = resolve_alias(storage, &alias)?.ok_or_else(|| {
        McpError::SourceNotFound(format!(
            "No source found for '{alias}' (required for {action:?})"
        ))
    })?;
    Ok(resolved)
}

fn remove_source(storage: &Storage, alias: &str) -> McpResult<RemoveOutput> {
    if !storage.exists(alias) {
        return Err(McpError::SourceNotFound(alias.to_string()));
    }

    let info = storage.load_llms_json(alias).ok().map(|llms| RemovalInfo {
        alias: alias.to_string(),
        url: llms.metadata.url.clone(),
        total_lines: llms.line_index.total_lines,
        fetched_at: llms.metadata.fetched_at.to_rfc3339(),
    });

    let dir = storage.tool_dir(alias)?;
    fs::remove_dir_all(&dir).map_err(|e| {
        McpError::Internal(format!(
            "Failed to remove source directory '{}': {e}",
            dir.display()
        ))
    })?;
    storage.remove_descriptor(alias)?;

    Ok(RemoveOutput {
        alias: alias.to_string(),
        message: format!("Removed source '{alias}' and cached data"),
        info,
    })
}

fn count_headings(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + count_headings(&entry.children))
        .sum()
}

fn load_info(storage: &Storage, alias: &str) -> McpResult<SourceInfoOutput> {
    if !storage.exists(alias) {
        return Err(McpError::SourceNotFound(alias.to_string()));
    }

    let llms = storage.load_llms_json(alias)?;
    let metadata = llms.metadata.clone();
    let llms_path = storage.llms_txt_path(alias)?;
    let file_metadata = fs::metadata(&llms_path).map_err(|e| {
        McpError::Internal(format!("Failed to read source file for '{alias}': {e}"))
    })?;
    let cache_path = storage.tool_dir(alias)?.to_string_lossy().to_string();

    Ok(SourceInfoOutput {
        alias: alias.to_string(),
        url: metadata.url.clone(),
        variant: format!("{:?}", metadata.variant),
        aliases: metadata.aliases.clone(),
        lines: llms.line_index.total_lines,
        headings: count_headings(&llms.toc),
        size_bytes: file_metadata.len(),
        last_updated: Some(metadata.fetched_at.to_rfc3339()),
        etag: metadata.etag.clone(),
        checksum: Some(metadata.sha256),
        cache_path,
        filter_stats: llms.filter_stats,
    })
}

async fn refresh_one(
    storage: &Storage,
    index_cache: &IndexCache,
    fetcher: &Fetcher,
    alias: &str,
    metrics: PerformanceMetrics,
    indexer: &DefaultRefreshIndexer,
) -> McpResult<RefreshResult> {
    let metadata = storage.load_metadata(alias)?;
    let aliases = storage.load_llms_aliases(alias)?;
    let filter_preference = metadata.filter_non_english.unwrap_or(true);

    let resolution = resolve_refresh_url(fetcher, &metadata).await?;
    let ctx = RefreshContext::new(metadata, aliases, resolution);
    let outcome = refresh_source_with_metadata(
        storage,
        fetcher,
        alias,
        &ctx,
        metrics,
        indexer,
        filter_preference,
    )
    .await?;

    cache::invalidate_cache(index_cache, alias).await;

    match outcome {
        RefreshOutcome::Refreshed {
            alias,
            headings,
            lines,
        } => Ok(RefreshResult {
            alias,
            status: RefreshStatus::Refreshed,
            headings: Some(headings),
            lines: Some(lines),
            headings_before: None,
            headings_after: None,
            filtered: None,
            message: None,
        }),
        RefreshOutcome::Unchanged { alias } => Ok(RefreshResult {
            alias,
            status: RefreshStatus::Unchanged,
            headings: None,
            lines: None,
            headings_before: None,
            headings_after: None,
            filtered: None,
            message: None,
        }),
    }
}

async fn reindex_one(
    storage: &Storage,
    index_cache: &IndexCache,
    alias: &str,
    metrics: PerformanceMetrics,
    indexer: &DefaultRefreshIndexer,
) -> McpResult<RefreshResult> {
    let metadata = storage.load_metadata(alias)?;
    let filter_preference = metadata.filter_non_english.unwrap_or(true);
    let outcome = reindex_source(storage, alias, metrics, indexer, filter_preference)?;

    cache::invalidate_cache(index_cache, alias).await;

    Ok(RefreshResult {
        alias: outcome.alias,
        status: RefreshStatus::Reindexed,
        headings: None,
        lines: None,
        headings_before: Some(outcome.headings_before),
        headings_after: Some(outcome.headings_after),
        filtered: Some(outcome.filtered),
        message: None,
    })
}

async fn refresh_sources(
    storage: &Storage,
    index_cache: &IndexCache,
    alias: Option<String>,
    all: bool,
    reindex: bool,
) -> McpResult<RefreshSummary> {
    if all && alias.is_some() {
        return Err(McpError::InvalidParams(
            "Provide either 'alias' or 'all', not both".to_string(),
        ));
    }

    let targets = if all {
        let sources = storage.list_sources();
        if sources.is_empty() {
            return Err(McpError::InvalidParams(
                "No sources available to refresh".to_string(),
            ));
        }
        sources
    } else {
        let alias = resolve_required_alias(storage, alias, BlzAction::Refresh)?;
        vec![alias]
    };

    let fetcher = Fetcher::new()?;
    let indexer = DefaultRefreshIndexer;
    let metrics = PerformanceMetrics::default();

    let mut results = Vec::new();
    let mut refreshed = 0;
    let mut unchanged = 0;
    let mut reindexed = 0;
    let mut errors = 0;

    for alias in targets {
        let result = if reindex {
            reindex_one(storage, index_cache, &alias, metrics.clone(), &indexer).await
        } else {
            refresh_one(
                storage,
                index_cache,
                &fetcher,
                &alias,
                metrics.clone(),
                &indexer,
            )
            .await
        };

        match result {
            Ok(entry) => {
                match entry.status {
                    RefreshStatus::Refreshed => refreshed += 1,
                    RefreshStatus::Unchanged => unchanged += 1,
                    RefreshStatus::Reindexed => reindexed += 1,
                    RefreshStatus::Error => errors += 1,
                }
                results.push(entry);
            },
            Err(e) => {
                errors += 1;
                results.push(RefreshResult {
                    alias,
                    status: RefreshStatus::Error,
                    headings: None,
                    lines: None,
                    headings_before: None,
                    headings_after: None,
                    filtered: None,
                    message: Some(e.to_string()),
                });
            },
        }
    }

    Ok(RefreshSummary {
        results,
        refreshed,
        unchanged,
        reindexed,
        errors,
    })
}

async fn handle_list_action(
    kind: Option<String>,
    query: Option<String>,
    storage: &Storage,
) -> McpResult<BlzOutput> {
    let list_params = ListSourcesParams { kind, query };
    let output = handle_list_sources(list_params, storage).await?;
    let mut response = empty_output(BlzAction::List);
    response.list = Some(output);
    Ok(response)
}

async fn handle_add_action(
    alias: Option<String>,
    url: Option<String>,
    force: bool,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<BlzOutput> {
    let alias = alias.ok_or_else(|| McpError::MissingParameter("alias".to_string()))?;
    let add_params = SourceAddParams { alias, url, force };
    let output = handle_source_add(add_params, storage, index_cache).await?;
    let mut response = empty_output(BlzAction::Add);
    response.add = Some(output);
    Ok(response)
}

async fn handle_remove_action(
    alias: Option<String>,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<BlzOutput> {
    let alias = resolve_required_alias(storage, alias, BlzAction::Remove)?;
    let output = remove_source(storage, &alias)?;
    cache::invalidate_cache(index_cache, &alias).await;
    let mut response = empty_output(BlzAction::Remove);
    response.remove = Some(output);
    Ok(response)
}

async fn handle_refresh_action(
    alias: Option<String>,
    all: bool,
    reindex: bool,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<BlzOutput> {
    let output = refresh_sources(storage, index_cache, alias, all, reindex).await?;
    let mut response = empty_output(BlzAction::Refresh);
    response.refresh = Some(output);
    Ok(response)
}

fn handle_info_action(alias: Option<String>, storage: &Storage) -> McpResult<BlzOutput> {
    let alias = resolve_required_alias(storage, alias, BlzAction::Info)?;
    let output = load_info(storage, &alias)?;
    let mut response = empty_output(BlzAction::Info);
    response.info = Some(output);
    Ok(response)
}

async fn handle_validate_action(alias: Option<String>, storage: &Storage) -> McpResult<BlzOutput> {
    let resolved = match alias {
        Some(value) => resolve_alias(storage, &value)?,
        None => None,
    };
    let output = handle_run_command(
        RunCommandParams {
            command: "validate".to_string(),
            source: resolved,
        },
        storage,
    )
    .await?;
    let mut response = empty_output(BlzAction::Validate);
    response.validate = Some(output);
    Ok(response)
}

async fn handle_history_action(alias: Option<String>, storage: &Storage) -> McpResult<BlzOutput> {
    let alias = resolve_required_alias(storage, alias, BlzAction::History)?;
    let output = handle_run_command(
        RunCommandParams {
            command: "history".to_string(),
            source: Some(alias),
        },
        storage,
    )
    .await?;
    let mut response = empty_output(BlzAction::History);
    response.history = Some(output);
    Ok(response)
}

async fn handle_help_action() -> McpResult<BlzOutput> {
    let output = handle_learn_blz(LearnBlzParams {}).await?;
    let mut response = empty_output(BlzAction::Help);
    response.help = Some(output.content);
    Ok(response)
}

fn handle_doctor_action(storage: &Storage) -> BlzOutput {
    let report = run_health_checks(storage);
    let mut response = empty_output(BlzAction::Doctor);
    response.doctor = Some(report);
    response
}

// ─────────────────────────────────────────────────────────────────────────────
// Health check decomposition: intermediate result types
// ─────────────────────────────────────────────────────────────────────────────

/// Result from disk health checks.
struct DiskHealthResult {
    /// Directory and disk usage checks.
    checks: Vec<HealthCheck>,
    /// Total cache size in bytes.
    total_size_bytes: u64,
    /// Recommendations from disk checks.
    recommendations: Vec<String>,
}

/// Result from source health analysis.
struct SourceAnalysisResult {
    /// Per-source health entries.
    entries: Vec<SourceHealthEntry>,
    /// Count of healthy sources.
    healthy_count: usize,
    /// Count of stale sources.
    stale_count: usize,
    /// List of stale source aliases.
    stale_sources: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Health check decomposition: focused helper functions
// ─────────────────────────────────────────────────────────────────────────────

/// Check cache and config directories, and compute disk usage.
fn check_disk_health(
    cache_dir: &std::path::Path,
    config_dir: &std::path::Path,
) -> DiskHealthResult {
    const WARN_THRESHOLD_BYTES: u64 = 1_000_000_000; // 1 GB

    let mut checks = Vec::new();
    let mut recommendations = Vec::new();

    // Directory checks
    checks.push(directory_check("Cache Directory", cache_dir));
    checks.push(directory_check("Config Directory", config_dir));

    // Disk usage check
    let total_size = calculate_dir_size(cache_dir);
    let disk_status = if total_size < WARN_THRESHOLD_BYTES {
        HealthStatus::Healthy
    } else {
        recommendations.push("Consider clearing unused sources to free disk space".to_string());
        HealthStatus::Warning
    };

    #[allow(clippy::cast_precision_loss)] // Acceptable for display purposes
    let size_mb = total_size as f64 / 1_048_576.0;
    checks.push(HealthCheck {
        name: "Disk Usage".to_string(),
        status: disk_status,
        message: format!("Cache size: {size_mb:.1} MB"),
        fixable: disk_status == HealthStatus::Warning,
    });

    DiskHealthResult {
        checks,
        total_size_bytes: total_size,
        recommendations,
    }
}

/// Analyze health of all sources.
fn analyze_source_health(storage: &Storage) -> SourceAnalysisResult {
    let sources = storage.list_sources();
    let stale_threshold = chrono::Duration::days(7);
    let now = chrono::Utc::now();

    let mut healthy_count = 0;
    let mut stale_count = 0;
    let mut stale_sources = Vec::new();
    let mut entries = Vec::new();

    for alias in &sources {
        let mut entry = SourceHealthEntry {
            alias: alias.clone(),
            source_type: SourceKind::Native,
            line_count: 0,
            is_stale: false,
            stale_days: None,
            failed_pages: None,
            native_available: None,
        };

        if let Ok(Some(metadata)) = storage.load_source_metadata(alias) {
            let age = now.signed_duration_since(metadata.fetched_at);

            if age > stale_threshold {
                entry.is_stale = true;
                // num_days() returns i64 but we know it's positive since age > threshold
                #[allow(clippy::cast_sign_loss)]
                let days = age.num_days() as u64;
                entry.stale_days = Some(days);
                stale_count += 1;
                stale_sources.push(alias.clone());
            } else {
                healthy_count += 1;
            }

            // Check if generated source
            if metadata.url.contains("firecrawl") || metadata.url.starts_with("generate://") {
                entry.source_type = SourceKind::Generated;
            }
        }

        // Get line count from llms.json if available
        if let Ok(llms) = storage.load_llms_json(alias) {
            entry.line_count = llms.line_index.total_lines;
        }

        entries.push(entry);
    }

    SourceAnalysisResult {
        entries,
        healthy_count,
        stale_count,
        stale_sources,
    }
}

/// Build a [`HealthCheck`] and recommendations from source analysis.
fn build_source_health_check(analysis: &SourceAnalysisResult) -> (HealthCheck, Vec<String>) {
    let mut recommendations = Vec::new();
    let total_sources = analysis.entries.len();

    let sources_status = if analysis.stale_count > 0 {
        recommendations.push(format!(
            "Run `blz sync --all` to refresh {} stale source(s)",
            analysis.stale_count
        ));
        HealthStatus::Warning
    } else if total_sources == 0 {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    };

    let check = HealthCheck {
        name: "Source Health".to_string(),
        status: sources_status,
        message: format!(
            "{} source(s): {} healthy, {} stale",
            total_sources, analysis.healthy_count, analysis.stale_count
        ),
        fixable: analysis.stale_count > 0,
    };

    (check, recommendations)
}

/// Compute overall status from all checks (worst-case wins).
fn compute_overall_status(checks: &[HealthCheck]) -> HealthStatus {
    checks
        .iter()
        .map(|c| c.status)
        .max_by_key(|s| match s {
            HealthStatus::Healthy => 0,
            HealthStatus::Warning => 1,
            HealthStatus::Error => 2,
        })
        .unwrap_or(HealthStatus::Healthy)
}

/// Assemble the final health report from all components.
fn aggregate_health_report(
    checks: Vec<HealthCheck>,
    recommendations: Vec<String>,
    cache_dir: &std::path::Path,
    config_dir: &std::path::Path,
    total_size_bytes: u64,
    source_analysis: SourceAnalysisResult,
) -> HealthReport {
    let total_files = count_files(cache_dir);
    let overall_status = compute_overall_status(&checks);

    HealthReport {
        overall_status,
        checks,
        recommendations,
        cache_info: CacheInfo {
            cache_dir: cache_dir.to_path_buf(),
            config_dir: config_dir.to_path_buf(),
            total_size_bytes,
            total_sources: source_analysis.entries.len(),
            total_files,
        },
        source_health: SourceHealth {
            total: source_analysis.entries.len(),
            healthy: source_analysis.healthy_count,
            stale: source_analysis.stale_count,
            corrupted: 0,
            stale_sources: source_analysis.stale_sources,
        },
        source_entries: source_analysis.entries,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main health check orchestrator
// ─────────────────────────────────────────────────────────────────────────────

/// Run health checks using core Storage APIs (no CLI dependency).
fn run_health_checks(storage: &Storage) -> HealthReport {
    let cache_dir = storage.root_dir();
    let config_dir = storage.config_dir();

    // Phase 1: Disk health checks
    let disk_result = check_disk_health(cache_dir, config_dir);

    // Phase 2: Source health analysis
    let source_analysis = analyze_source_health(storage);

    // Phase 3: Build source health check
    let (source_check, source_recommendations) = build_source_health_check(&source_analysis);

    // Phase 4: Combine all results
    let mut checks = disk_result.checks;
    checks.push(source_check);

    let mut recommendations = disk_result.recommendations;
    recommendations.extend(source_recommendations);

    aggregate_health_report(
        checks,
        recommendations,
        cache_dir,
        config_dir,
        disk_result.total_size_bytes,
        source_analysis,
    )
}

fn directory_check(name: &str, path: &std::path::Path) -> HealthCheck {
    let exists = path.exists();
    let writable = exists
        && path
            .metadata()
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false);

    let (status, message) = if writable {
        (
            HealthStatus::Healthy,
            format!("{name} exists and is writable: {}", path.display()),
        )
    } else if exists {
        (
            HealthStatus::Error,
            format!("{name} exists but is read-only: {}", path.display()),
        )
    } else {
        (
            HealthStatus::Error,
            format!("{name} missing: {}", path.display()),
        )
    };

    HealthCheck {
        name: name.to_string(),
        status,
        message,
        fixable: false,
    }
}

fn calculate_dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.filter_map(Result::ok) {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }
                if file_type.is_dir() {
                    total += calculate_dir_size(&entry.path());
                } else if file_type.is_file() {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    total
}

fn count_files(path: &std::path::Path) -> usize {
    let mut count = 0;
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.filter_map(Result::ok) {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }
                if file_type.is_dir() {
                    count += count_files(&entry.path());
                } else if file_type.is_file() {
                    count += 1;
                }
            }
        }
    }
    count
}

fn handle_lookup_action(query: Option<String>, limit: Option<usize>) -> McpResult<BlzOutput> {
    let query = query.ok_or_else(|| McpError::MissingParameter("query".to_string()))?;
    let limit = limit.unwrap_or(10);
    let registry = Registry::default();
    let search_results = registry.search(&query);
    let total = search_results.len();

    let results: Vec<LookupResult> = search_results
        .into_iter()
        .take(limit)
        .map(|r| LookupResult {
            name: r.entry.name,
            slug: r.entry.slug,
            description: r.entry.description,
            url: r.entry.llms_url,
            score: r.score,
            match_field: r.match_field,
        })
        .collect();

    let mut response = empty_output(BlzAction::Lookup);
    response.lookup = Some(LookupOutput {
        query,
        results,
        total,
    });
    Ok(response)
}

async fn handle_clear_cache_action(
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<BlzOutput> {
    let sources = storage.list_sources();
    let count = sources.len();

    // Invalidate all known source caches
    for source in &sources {
        cache::invalidate_cache(index_cache, source).await;
    }

    // Always call clear_cache to remove any orphaned/corrupted data
    // that may exist even when list_sources() returns empty
    // (list_sources only returns directories with valid llms.json)
    storage.clear_cache()?;

    let message = if count == 0 {
        "Cache cleared (no valid sources found, but orphaned data may have been removed)"
            .to_string()
    } else {
        format!("Cleared cache with {count} source(s)")
    };

    let mut response = empty_output(BlzAction::ClearCache);
    response.clear = Some(ClearCacheOutput {
        message,
        cleared: count,
        sources,
    });
    Ok(response)
}

/// Reserved keywords that cannot be used as aliases to avoid CLI conflicts.
const RESERVED_KEYWORDS: &[&str] = &[
    "add",
    "search",
    "get",
    "list",
    "sources",
    "update",
    "remove",
    "rm",
    "delete",
    "help",
    "version",
    "completions",
    "diff",
    "lookup",
    "plugin",
    "claude-plugin",
    "config",
    "settings",
    "serve",
    "server",
    "start",
    "stop",
    "status",
    "sync",
    "refresh",
    "info",
    "doctor",
    "map",
    "toc",
    "query",
    "find",
    "generate",
    "alias",
];

fn validate_alias(alias: &str) -> McpResult<()> {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        return Err(McpError::InvalidParams("Alias cannot be empty".to_string()));
    }
    if trimmed.len() > 100 {
        return Err(McpError::InvalidParams(
            "Alias exceeds maximum length (100)".to_string(),
        ));
    }
    if trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(McpError::InvalidParams(
            "Alias cannot contain whitespace or control characters".to_string(),
        ));
    }
    // Check reserved keywords to prevent CLI conflicts
    if RESERVED_KEYWORDS.contains(&trimmed.to_lowercase().as_str()) {
        return Err(McpError::InvalidParams(format!(
            "Alias '{alias}' is reserved (CLI command name)"
        )));
    }
    Ok(())
}

fn find_alias_owner(storage: &Storage, alias: &str) -> Option<String> {
    for src in storage.list_sources() {
        if let Ok(llms) = storage.load_llms_json(&src) {
            if llms.metadata.aliases.iter().any(|a| a == alias) {
                return Some(src);
            }
        }
    }
    None
}

fn handle_add_alias_action(
    source: Option<String>,
    target_alias: Option<String>,
    storage: &Storage,
) -> McpResult<BlzOutput> {
    let source = source.ok_or_else(|| McpError::MissingParameter("alias".to_string()))?;
    let new_alias =
        target_alias.ok_or_else(|| McpError::MissingParameter("targetAlias".to_string()))?;

    if !storage.exists(&source) {
        return Err(McpError::SourceNotFound(source));
    }

    validate_alias(&new_alias)?;

    // Prevent adding the canonical as an alias
    if source.eq_ignore_ascii_case(&new_alias) {
        return Err(McpError::InvalidParams(format!(
            "Alias '{new_alias}' matches the canonical source name"
        )));
    }

    // Enforce uniqueness across all sources (metadata aliases)
    if let Some(owner) = find_alias_owner(storage, &new_alias) {
        if owner != source {
            return Err(McpError::InvalidParams(format!(
                "Alias '{new_alias}' is already used by source '{owner}'"
            )));
        }
    }

    // Prevent aliases that match another source's canonical name
    // (resolve_alias prefers canonical names, making such aliases unusable)
    let canonical_sources: Vec<String> = storage.list_sources();
    if canonical_sources
        .iter()
        .any(|s| s.eq_ignore_ascii_case(&new_alias) && s != &source)
    {
        return Err(McpError::InvalidParams(format!(
            "Alias '{new_alias}' conflicts with an existing source's canonical name"
        )));
    }

    let mut llms = storage.load_llms_json(&source)?;

    // Check if alias already exists
    if llms.metadata.aliases.iter().any(|a| a == &new_alias) {
        let mut response = empty_output(BlzAction::AddAlias);
        response.alias_add = Some(AliasOutput {
            message: format!("'{source}' already has alias '{new_alias}'"),
            source,
            alias: new_alias,
            no_op: true,
            current_aliases: llms.metadata.aliases,
        });
        return Ok(response);
    }

    // Add the alias
    llms.metadata.aliases.push(new_alias.clone());
    storage.save_llms_json(&source, &llms)?;
    storage.save_source_metadata(&source, &llms.metadata)?;

    let mut response = empty_output(BlzAction::AddAlias);
    response.alias_add = Some(AliasOutput {
        message: format!("Added alias '{new_alias}' to '{source}'"),
        source,
        alias: new_alias,
        no_op: false,
        current_aliases: llms.metadata.aliases,
    });
    Ok(response)
}

fn handle_remove_alias_action(
    source: Option<String>,
    target_alias: Option<String>,
    storage: &Storage,
) -> McpResult<BlzOutput> {
    let source = source.ok_or_else(|| McpError::MissingParameter("alias".to_string()))?;
    let alias_to_remove =
        target_alias.ok_or_else(|| McpError::MissingParameter("targetAlias".to_string()))?;

    if !storage.exists(&source) {
        return Err(McpError::SourceNotFound(source));
    }

    let mut llms = storage.load_llms_json(&source)?;

    let before = llms.metadata.aliases.len();
    llms.metadata.aliases.retain(|a| a != &alias_to_remove);

    // Check if alias was found
    if llms.metadata.aliases.len() == before {
        let mut response = empty_output(BlzAction::RemoveAlias);
        response.alias_remove = Some(AliasOutput {
            message: format!("Alias '{alias_to_remove}' not found on '{source}'"),
            source,
            alias: alias_to_remove,
            no_op: true,
            current_aliases: llms.metadata.aliases,
        });
        return Ok(response);
    }

    // Save the updated metadata
    storage.save_llms_json(&source, &llms)?;
    storage.save_source_metadata(&source, &llms.metadata)?;

    let mut response = empty_output(BlzAction::RemoveAlias);
    response.alias_remove = Some(AliasOutput {
        message: format!("Removed alias '{alias_to_remove}' from '{source}'"),
        source,
        alias: alias_to_remove,
        no_op: false,
        current_aliases: llms.metadata.aliases,
    });
    Ok(response)
}

/// Main handler for blz tool
#[tracing::instrument(skip(storage, index_cache))]
pub async fn handle_blz(
    params: BlzParams,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<BlzOutput> {
    let action = resolve_action(&params);
    let BlzParams {
        alias,
        url,
        force,
        kind,
        query,
        reindex,
        all,
        limit,
        target_alias,
        ..
    } = params;

    match action {
        BlzAction::List => handle_list_action(kind, query, storage).await,
        BlzAction::Add => handle_add_action(alias, url, force, storage, index_cache).await,
        BlzAction::Remove => handle_remove_action(alias, storage, index_cache).await,
        BlzAction::Refresh => {
            handle_refresh_action(alias, all, reindex, storage, index_cache).await
        },
        BlzAction::Info => handle_info_action(alias, storage),
        BlzAction::Validate => handle_validate_action(alias, storage).await,
        BlzAction::History => handle_history_action(alias, storage).await,
        BlzAction::Lookup => handle_lookup_action(query, limit),
        BlzAction::Doctor => Ok(handle_doctor_action(storage)),
        BlzAction::ClearCache => handle_clear_cache_action(storage, index_cache).await,
        BlzAction::AddAlias => handle_add_alias_action(alias, target_alias, storage),
        BlzAction::RemoveAlias => handle_remove_alias_action(alias, target_alias, storage),
        BlzAction::Help => handle_help_action().await,
    }
}
