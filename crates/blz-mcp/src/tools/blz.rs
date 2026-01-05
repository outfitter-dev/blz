//! Unified BLZ tool for source management and metadata actions.

use std::fs;

use blz_core::refresh::{
    DefaultRefreshIndexer, RefreshOutcome, RefreshStorage, refresh_source_with_metadata,
    reindex_source, resolve_refresh_url,
};
use blz_core::{Fetcher, HeadingFilterStats, PerformanceMetrics, Storage, TocEntry};
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
    let outcome = refresh_source_with_metadata(
        storage,
        fetcher,
        alias,
        metadata,
        aliases,
        &resolution,
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
        BlzAction::Help => handle_help_action().await,
    }
}
