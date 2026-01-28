//! Add command implementation

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use blz_core::numeric::safe_percentage;
use blz_core::{
    Fetcher, LanguageFilter, MarkdownParser, ParseResult, PerformanceMetrics, SearchIndex, Source,
    SourceDescriptor, SourceOrigin, SourceType, SourceVariant, Storage, build_llms_json,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs as sync_fs;
use std::path::Path;
use tokio::fs as async_fs;
use url::Url;

use crate::utils::count_headings;
use crate::utils::validation::{normalize_alias, validate_alias};
use blz_core::discovery::{ProbeResult, probe_domain};
use blz_core::url_resolver;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAnalysis {
    #[serde(alias = "alias")]
    name: String,
    url: String,
    final_url: String,
    analysis: ContentAnalysis,
    would_index: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentAnalysis {
    line_count: usize,
    char_count: usize,
    header_count: usize,
    sections: usize,
    file_size: String,
    content_type: String,
}

/// Descriptor metadata collected from CLI inputs or manifests.
#[derive(Debug, Clone, Default)]
pub struct DescriptorInput {
    /// Display name for the source.
    name: Option<String>,
    /// Optional description for the source.
    description: Option<String>,
    /// Optional category for grouping.
    category: Option<String>,
    /// Tag list for discovery.
    tags: Vec<String>,
    /// Additional aliases for the source.
    aliases: Vec<String>,
    /// NPM aliases for the source.
    npm_aliases: Vec<String>,
    /// GitHub aliases for the source.
    github_aliases: Vec<String>,
    /// Optional manifest origin metadata.
    manifest: Option<blz_core::ManifestOrigin>,
}

struct ResolvedAddition {
    content: String,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    resolved_url: String,
    variant: SourceVariant,
    origin: SourceOrigin,
}

// ============================================================
// Generate Flow Integration
// ============================================================

// These types and functions are prepared for integration in a follow-up PR
// that will wire them into the actual add command flow.

#[allow(dead_code)]
/// Action to take after discovery when adding a source.
///
/// Used to determine the appropriate workflow based on what documentation
/// sources are available at a domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddAction {
    /// Use native llms-full.txt (preferred - complete documentation).
    UseNative {
        /// URL to the llms-full.txt file.
        url: String,
    },
    /// Generate llms-full.txt from discovered URLs via Firecrawl.
    Generate {
        /// Number of URLs to scrape.
        url_count: usize,
    },
    /// Use llms.txt as index only (no generation).
    IndexOnly {
        /// URL to the llms.txt file.
        url: String,
    },
    /// Cancel the operation.
    Cancel,
}

impl std::fmt::Display for AddAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UseNative { url } => write!(f, "Use native llms-full.txt ({url})"),
            Self::Generate { url_count } => write!(f, "Generate from {url_count} URLs"),
            Self::IndexOnly { url } => write!(f, "Index only from llms.txt ({url})"),
            Self::Cancel => write!(f, "Cancel operation"),
        }
    }
}

/// Determine what action to take based on discovery results.
///
/// # Arguments
///
/// * `probe` - Result from probing the domain
/// * `interactive` - Whether to allow interactive prompts (currently unused)
///
/// # Returns
///
/// The recommended [`AddAction`] based on available sources.
///
/// # Errors
///
/// Returns an error if no usable documentation sources are found.
///
/// # Behavior
///
/// 1. If `llms_full_url` exists, returns `UseNative`
/// 2. If only `llms_url` exists, returns `IndexOnly`
/// 3. If only `sitemap_url` exists, returns error (requires generation)
/// 4. If nothing found, returns error
#[allow(dead_code)]
pub fn determine_add_action(probe: &ProbeResult, _interactive: bool) -> Result<AddAction> {
    // Prefer native llms-full.txt when available
    if let Some(url) = &probe.llms_full_url {
        return Ok(AddAction::UseNative { url: url.clone() });
    }

    // If llms.txt exists, can use it as index-only
    if let Some(url) = &probe.llms_url {
        return Ok(AddAction::IndexOnly { url: url.clone() });
    }

    // Sitemap only - would need generation to be useful
    if probe.sitemap_url.is_some() {
        anyhow::bail!(
            "No documentation sources found for '{}'. \
             Found sitemap.xml but no llms.txt or llms-full.txt. \
             Generation from sitemap requires Firecrawl.",
            probe.domain
        );
    }

    // Nothing found
    let subdomain_hint = if probe.docs_subdomain_checked {
        " (also checked docs.* subdomain)"
    } else {
        ""
    };
    anyhow::bail!(
        "No documentation sources found for '{}'{}. \
         Expected llms-full.txt, llms.txt, or sitemap.xml at the domain root.",
        probe.domain,
        subdomain_hint
    );
}

/// Prompt choices for generate action.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerateChoice {
    /// Generate documentation from discovered URLs.
    Generate,
    /// Use llms.txt as index only.
    IndexOnly,
    /// Cancel the operation.
    Cancel,
}

impl std::fmt::Display for GenerateChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generate => write!(f, "Generate (scrape discovered URLs)"),
            Self::IndexOnly => write!(f, "Index only (use llms.txt as-is)"),
            Self::Cancel => write!(f, "Cancel"),
        }
    }
}

/// Prompt user for action when no native llms-full.txt is available.
///
/// Shows discovered URL count and offers:
/// - Generate: scrape URLs to create llms-full.txt
/// - Index only: use llms.txt as-is (if available)
/// - Cancel: abort the operation
///
/// # Arguments
///
/// * `url_count` - Number of URLs discovered for generation
/// * `has_llms_txt` - Whether llms.txt is available for index-only mode
///
/// # Returns
///
/// The selected [`AddAction`].
///
/// # Errors
///
/// Returns an error if the prompt is interrupted or cannot be displayed.
#[allow(dead_code)]
pub fn prompt_generate_action(url_count: usize, has_llms_txt: bool) -> Result<AddAction> {
    use inquire::Select;

    let message =
        format!("No llms-full.txt available. Found {url_count} URLs. What would you like to do?");

    let mut choices = vec![GenerateChoice::Generate];
    if has_llms_txt {
        choices.push(GenerateChoice::IndexOnly);
    }
    choices.push(GenerateChoice::Cancel);

    let selection = Select::new(&message, choices)
        .with_help_message("Use arrow keys to select, Enter to confirm")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Prompt cancelled: {e}"))?;

    match selection {
        GenerateChoice::Generate => Ok(AddAction::Generate { url_count }),
        GenerateChoice::IndexOnly => Ok(AddAction::IndexOnly {
            url: String::new(), // Will be filled by caller
        }),
        GenerateChoice::Cancel => Ok(AddAction::Cancel),
    }
}

/// Statistics from a generate operation.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct GenerateStats {
    /// Number of successfully scraped pages.
    pub successful: usize,
    /// Number of failed pages.
    pub failed: usize,
    /// Total lines in generated content.
    pub total_lines: usize,
}

#[allow(dead_code)]
impl GenerateStats {
    /// Get the total number of URLs processed.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.successful + self.failed
    }
}

/// Execute the generate flow to scrape URLs and create documentation.
///
/// Uses the [`GenerateOrchestrator`] to scrape discovered URLs in parallel,
/// showing progress via an indicatif progress bar.
///
/// # Arguments
///
/// * `urls` - URLs to scrape (with optional lastmod for change detection)
/// * `concurrency` - Maximum concurrent scrape operations
/// * `quiet` - Whether to suppress progress output
///
/// # Returns
///
/// A tuple of (assembled markdown content, [`GenerateStats`]).
///
/// # Errors
///
/// Returns an error if the scraping fails completely (partial failures are recorded in stats).
///
/// [`GenerateOrchestrator`]: crate::generate::GenerateOrchestrator
#[allow(dead_code)]
pub async fn execute_generate_flow(
    urls: &[crate::generate::UrlWithLastmod],
    scraper: impl crate::generate::Scraper,
    concurrency: usize,
    quiet: bool,
) -> Result<(String, GenerateStats)> {
    use crate::generate::GenerateOrchestrator;

    if urls.is_empty() {
        return Ok((String::new(), GenerateStats::default()));
    }

    let total = urls.len();

    // Create progress bar
    let pb = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=>-"),
        );
        pb.set_message("Scraping...");
        pb
    };

    // Track progress with atomic counter for thread-safe updates
    let completed = Arc::new(AtomicUsize::new(0));
    let completed_clone = Arc::clone(&completed);
    let pb_clone = pb.clone();

    // Create orchestrator with progress callback
    let orchestrator =
        GenerateOrchestrator::new(scraper, concurrency).with_progress(move |done, _total| {
            completed_clone.store(done, Ordering::SeqCst);
            pb_clone.set_position(done as u64);
        });

    // Execute scraping
    let results = orchestrator.scrape_all(urls).await;

    pb.finish_and_clear();

    // Assemble markdown from successful scrapes
    let mut assembled = String::new();
    let mut total_lines = 0usize;

    for entry in &results.successful {
        // Add page header with source URL
        if !assembled.is_empty() {
            assembled.push_str("\n\n---\n\n");
        }

        // Add title if available
        if let Some(title) = &entry.title {
            assembled.push_str("# ");
            assembled.push_str(title);
            assembled.push_str("\n\n");
        }

        // Add source attribution
        assembled.push_str("> Source: ");
        assembled.push_str(&entry.url);
        assembled.push_str("\n\n");

        // Add content
        assembled.push_str(&entry.markdown);

        total_lines += entry.line_count;
    }

    let stats = GenerateStats {
        successful: results.successful.len(),
        failed: results.failed.len(),
        total_lines,
    };

    Ok((assembled, stats))
}

impl DescriptorInput {
    pub fn from_cli_inputs(
        aliases: &[String],
        name: Option<&str>,
        description: Option<&str>,
        category: Option<&str>,
        tags: &[String],
    ) -> Self {
        Self {
            name: name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            description: description.map(|s| s.trim().to_string()),
            category: category
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            tags: tags.iter().map(|t| t.trim().to_string()).collect(),
            aliases: aliases.iter().map(|a| a.trim().to_string()).collect(),
            ..Self::default()
        }
    }
}

/// Prepared request for running the add flow.
pub struct AddRequest {
    /// Alias to store the source under.
    pub alias: String,
    /// Source URL to fetch.
    pub url: String,
    /// Descriptor metadata for the source.
    pub descriptor: DescriptorInput,
    /// Whether to skip writes and indexing.
    pub dry_run: bool,
    /// Suppress non-essential output.
    pub quiet: bool,
    /// Performance metrics collector.
    pub metrics: PerformanceMetrics,
    /// Disable language filtering for this add.
    pub no_language_filter: bool,
}

/// Options controlling add flow behavior.
#[derive(Clone, Copy, Debug, Default)]
pub struct AddFlowOptions {
    /// Whether to skip writes and indexing.
    pub dry_run: bool,
    /// Suppress non-essential output.
    pub quiet: bool,
    /// Disable language filtering for this add.
    pub no_language_filter: bool,
}

impl AddFlowOptions {
    pub const fn new(dry_run: bool, quiet: bool, no_language_filter: bool) -> Self {
        Self {
            dry_run,
            quiet,
            no_language_filter,
        }
    }
}

impl AddRequest {
    pub fn new(
        alias: impl Into<String>,
        url: impl Into<String>,
        descriptor: DescriptorInput,
        dry_run: bool,
        quiet: bool,
        metrics: PerformanceMetrics,
        no_language_filter: bool,
    ) -> Self {
        Self {
            alias: alias.into(),
            url: url.into(),
            descriptor,
            dry_run,
            quiet,
            metrics,
            no_language_filter,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(default)]
    version: Option<String>,
    #[serde(rename = "source", default)]
    sources: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    alias: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    #[serde(rename = "aliases")]
    alias_sets: ManifestAliases,
    #[serde(default)]
    _notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestAliases {
    #[serde(default)]
    npm: Vec<String>,
    #[serde(default)]
    github: Vec<String>,
}

/// Add a new documentation source.
///
/// # Arguments
/// Execute the add flow given a prepared request.
///
/// # Errors
///
/// Returns an error if validation, fetching, or indexing fails.
pub async fn execute(request: AddRequest) -> Result<()> {
    let AddRequest {
        alias,
        url,
        descriptor,
        dry_run,
        quiet,
        metrics,
        no_language_filter,
    } = request;
    let options = AddFlowOptions::new(dry_run, quiet, no_language_filter);

    // Normalize the alias to kebab-case lowercase
    let normalized_alias = normalize_alias(&alias);

    // Show normalization if it changed
    if normalized_alias != alias && !quiet && !dry_run {
        println!(
            "Normalizing alias: '{}' → '{}'",
            alias,
            normalized_alias.green()
        );
    }

    // Validate the normalized alias
    validate_alias(&normalized_alias)?;

    let fetcher = Fetcher::new()?;

    if let Ok(parsed) = Url::parse(&url) {
        match parsed.scheme() {
            "http" | "https" => {},
            other => {
                if !quiet && !dry_run {
                    eprintln!(
                        "Warning: URL scheme '{other}' may not be supported for fetching ({url}).\n \
                         If this is a local file, consider hosting llms.txt or using a supported HTTP URL."
                    );
                }
            },
        }
    } else if !quiet && !dry_run {
        eprintln!("Warning: URL appears invalid: {url}");
    }

    fetch_and_index(
        &normalized_alias,
        &url,
        descriptor,
        fetcher,
        metrics,
        options,
    )
    .await
}

pub async fn execute_manifest(
    manifest_path: &Path,
    only: &[String],
    metrics: PerformanceMetrics,
    options: AddFlowOptions,
) -> Result<()> {
    let AddFlowOptions {
        dry_run,
        quiet,
        no_language_filter,
    } = options;
    let manifest_text = async_fs::read_to_string(manifest_path).await?;
    let manifest: ManifestFile = toml::from_str(&manifest_text)?;

    let manifest_abs =
        sync_fs::canonicalize(manifest_path).unwrap_or_else(|_| manifest_path.to_path_buf());
    let manifest_path_str = manifest_abs.to_string_lossy().to_string();

    let mut filter: Vec<String> = only.iter().map(|alias| normalize_alias(alias)).collect();
    if !filter.is_empty() {
        filter.sort();
    }

    let mut processed = 0usize;
    for entry in manifest.sources {
        let normalized_alias = normalize_alias(&entry.alias);

        if !filter.is_empty() && filter.binary_search(&normalized_alias).is_err() {
            continue;
        }

        validate_alias(&normalized_alias)?;

        let descriptor_input = DescriptorInput {
            name: non_empty_string(Some(&entry.name)),
            description: entry.description.clone().map(|s| s.trim().to_string()),
            category: non_empty_string(Some(&entry.category)),
            tags: dedupe_sorted(entry.tags.clone()),
            aliases: Vec::new(),
            npm_aliases: dedupe_sorted(entry.alias_sets.npm.clone()),
            github_aliases: dedupe_sorted(entry.alias_sets.github.clone()),
            manifest: Some(blz_core::ManifestOrigin {
                path: manifest_path_str.clone(),
                entry_alias: entry.alias.clone(),
                version: manifest.version.clone(),
            }),
        };

        match (entry.url.as_ref(), entry.path.as_ref()) {
            (Some(url), None) => {
                let request = AddRequest::new(
                    normalized_alias.clone(),
                    url.clone(),
                    descriptor_input.clone(),
                    dry_run,
                    quiet,
                    metrics.clone(),
                    no_language_filter,
                );
                execute(request).await?;
            },
            (None, Some(path)) => {
                let base_dir = manifest_abs.parent().unwrap_or_else(|| Path::new("."));
                let resolved = if Path::new(path).is_absolute() {
                    Path::new(path).to_path_buf()
                } else {
                    base_dir.join(path)
                };
                add_local_source(
                    &normalized_alias,
                    &resolved,
                    descriptor_input,
                    dry_run,
                    quiet,
                    metrics.clone(),
                    no_language_filter,
                )
                .await?;
            },
            (Some(_), Some(_)) => {
                anyhow::bail!(
                    "Manifest entry '{}' must specify either 'url' or 'path', not both",
                    entry.alias
                );
            },
            (None, None) => {
                anyhow::bail!(
                    "Manifest entry '{}' is missing 'url' or 'path'",
                    entry.alias
                );
            },
        }

        processed += 1;
    }

    if processed == 0 && !quiet {
        eprintln!("No manifest sources matched the provided filters.");
    }

    Ok(())
}

async fn fetch_and_index(
    alias: &str,
    url: &str,
    descriptor_input: DescriptorInput,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
    options: AddFlowOptions,
) -> Result<()> {
    let AddFlowOptions {
        dry_run,
        quiet,
        no_language_filter,
    } = options;
    // Check if source already exists (validate even in dry-run mode)
    let storage = Storage::new()?;
    if storage.exists(alias) {
        anyhow::bail!(
            "Source '{alias}' already exists. Use 'blz refresh {alias}' or choose a different alias."
        );
    }

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner("Resolving URL...")
    };

    // Resolve the best URL variant (llms-full.txt vs llms.txt)
    spinner.set_message("Resolving URL variant...");
    let resolved = url_resolver::resolve_best_url(&fetcher, url).await?;

    // Show warning if index file
    if resolved.should_warn && !quiet && !dry_run {
        warn_index_only_file(&spinner, resolved.line_count);
    }

    // Fetch from resolved URL
    spinner.set_message("Fetching documentation...");
    let fetch_result = fetcher
        .fetch_with_cache(&resolved.final_url, None, None)
        .await?;

    let (content, sha256, etag, last_modified) = match fetch_result {
        blz_core::FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => (content, sha256, etag, last_modified),
        blz_core::FetchResult::NotModified { .. } => {
            anyhow::bail!(
                "Server returned 304 Not Modified on initial fetch. This should not happen for new sources."
            );
        },
    };

    // Parse the content
    spinner.set_message("Parsing markdown...");
    let mut parser = MarkdownParser::new()?;
    let mut parse_result = parser.parse(&content)?;

    // Apply language filtering if enabled
    apply_language_filter(&mut parse_result, no_language_filter, quiet);

    // In dry-run mode, analyze content and output JSON instead of indexing
    if dry_run {
        output_dry_run_analysis(alias, url, &resolved, &content, &parse_result)?;
        spinner.finish_and_clear();
        return Ok(());
    }
    let resolved_addition = build_remote_addition(content, sha256, etag, last_modified, &resolved);

    let llms_json = finalize_add(
        &storage,
        alias,
        resolved_addition,
        descriptor_input,
        &parse_result,
        &spinner,
        metrics,
        no_language_filter,
    )?;

    spinner.finish_and_clear();

    if !quiet {
        println!(
            "{} {} ({} headings, {} lines)",
            "✓ Added".green(),
            alias.green(),
            count_headings(&llms_json.toc),
            llms_json.line_index.total_lines
        );
    }

    Ok(())
}

/// Output dry-run analysis as JSON for remote sources.
fn output_dry_run_analysis(
    alias: &str,
    url: &str,
    resolved: &url_resolver::ResolvedUrl,
    content: &str,
    parse_result: &blz_core::ParseResult,
) -> Result<()> {
    let char_count = content.len();
    let header_count = parse_result.heading_blocks.len();
    let sections = parse_result.toc.len();
    let file_size = format_size(content.len());

    let content_type = match resolved.content_type {
        blz_core::ContentType::Full => "full",
        blz_core::ContentType::Index => "index",
        blz_core::ContentType::Mixed => "mixed",
    };

    let analysis = SourceAnalysis {
        name: alias.to_string(),
        url: url.to_string(),
        final_url: resolved.final_url.clone(),
        analysis: ContentAnalysis {
            line_count: resolved.line_count,
            char_count,
            header_count,
            sections,
            file_size,
            content_type: content_type.to_string(),
        },
        would_index: true,
    };

    let json = serde_json::to_string_pretty(&analysis)?;
    println!("{json}");
    Ok(())
}

/// Output dry-run analysis as JSON for local file sources.
fn output_local_dry_run_analysis(
    alias: &str,
    path: &Path,
    content: &str,
    parse_result: &blz_core::ParseResult,
) -> Result<()> {
    let analysis = SourceAnalysis {
        name: alias.to_string(),
        url: path.display().to_string(),
        final_url: path.display().to_string(),
        analysis: ContentAnalysis {
            line_count: parse_result.line_count,
            char_count: content.len(),
            header_count: parse_result.heading_blocks.len(),
            sections: parse_result.toc.len(),
            file_size: format_size(content.len()),
            content_type: "local".to_string(),
        },
        would_index: true,
    };
    let json = serde_json::to_string_pretty(&analysis)?;
    println!("{json}");
    Ok(())
}

async fn add_local_source(
    alias: &str,
    path: &Path,
    descriptor_input: DescriptorInput,
    dry_run: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
    no_language_filter: bool,
) -> Result<()> {
    let storage = Storage::new()?;
    if storage.exists(alias) {
        anyhow::bail!(
            "Source '{alias}' already exists. Use 'blz refresh {alias}' or choose a different alias."
        );
    }

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner("Reading local file...")
    };

    let metadata = async_fs::metadata(path).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to access local source at '{}': {}",
            path.display(),
            e
        )
    })?;
    if !metadata.is_file() {
        anyhow::bail!("Local source '{}' is not a file", path.display());
    }

    spinner.set_message("Reading local file...");
    let content = async_fs::read_to_string(path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read local source '{}': {}", path.display(), e))?;

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    // Use base64 encoding to match remote sources
    let sha256 = STANDARD.encode(hasher.finalize());

    spinner.set_message("Parsing markdown...");
    let mut parser = MarkdownParser::new()?;
    let mut parse_result = parser.parse(&content)?;

    // Apply language filtering for consistency with remote sources
    apply_language_filter(&mut parse_result, no_language_filter, quiet);

    if dry_run {
        output_local_dry_run_analysis(alias, path, &content, &parse_result)?;
        spinner.finish_and_clear();
        return Ok(());
    }

    let abs_path = sync_fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs_path.to_string_lossy().to_string();

    let resolved_addition = ResolvedAddition {
        content,
        sha256,
        etag: None,
        last_modified: None,
        resolved_url: path_str.clone(),
        variant: SourceVariant::Llms,
        origin: SourceOrigin {
            manifest: None,
            source_type: Some(SourceType::LocalFile { path: path_str }),
        },
    };

    let llms_json = finalize_add(
        &storage,
        alias,
        resolved_addition,
        descriptor_input,
        &parse_result,
        &spinner,
        metrics,
        no_language_filter,
    )?;

    spinner.finish_and_clear();

    if !quiet {
        println!(
            "{} {} ({} headings, {} lines)",
            "✓ Added".green(),
            alias.green(),
            count_headings(&llms_json.toc),
            llms_json.line_index.total_lines
        );
    }

    Ok(())
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(message.to_string());
    pb
}

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        let whole = bytes / KB;
        let tenths = ((bytes % KB) * 10) / KB;
        format!("{whole}.{tenths} KB")
    } else {
        let whole = bytes / MB;
        let tenths = ((bytes % MB) * 10) / MB;
        format!("{whole}.{tenths} MB")
    }
}

/// Warn that the source appears to be an index-only file.
fn warn_index_only_file(spinner: &ProgressBar, line_count: usize) {
    spinner.finish_and_clear();
    eprintln!(
        "{} This appears to be a navigation index only ({line_count} lines).\n\
         BLZ works best with full documentation files (llms-full.txt).\n\
         If this is a hub or registry, add the downstream source URL instead.",
        "⚠".yellow(),
    );
}

/// Build a `ResolvedAddition` for a remote source.
fn build_remote_addition(
    content: String,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    resolved: &url_resolver::ResolvedUrl,
) -> ResolvedAddition {
    ResolvedAddition {
        content,
        sha256,
        etag,
        last_modified,
        resolved_url: resolved.final_url.clone(),
        variant: resolved.variant.clone(),
        origin: SourceOrigin {
            manifest: None,
            source_type: Some(SourceType::Remote {
                url: resolved.final_url.clone(),
            }),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn finalize_add(
    storage: &Storage,
    alias: &str,
    resolved: ResolvedAddition,
    mut descriptor_input: DescriptorInput,
    parse_result: &blz_core::ParseResult,
    spinner: &ProgressBar,
    metrics: PerformanceMetrics,
    no_language_filter: bool,
) -> Result<blz_core::LlmsJson> {
    spinner.set_message("Saving content...");
    storage.save_llms_txt(alias, &resolved.content)?;

    spinner.set_message("Building metadata...");

    let aliases = dedupe_sorted(std::mem::take(&mut descriptor_input.aliases));
    let tags = dedupe_sorted(std::mem::take(&mut descriptor_input.tags));
    let npm_aliases = dedupe_sorted(std::mem::take(&mut descriptor_input.npm_aliases));
    let github_aliases = dedupe_sorted(std::mem::take(&mut descriptor_input.github_aliases));

    let descriptor_name = descriptor_input
        .name
        .clone()
        .unwrap_or_else(|| display_name_from_alias(alias));
    let descriptor_category = descriptor_input
        .category
        .clone()
        .unwrap_or_else(|| "uncategorized".to_string());
    let descriptor_description = descriptor_input.description.unwrap_or_default();

    let metadata_description = non_empty_string(Some(&descriptor_description));
    let metadata_category = non_empty_string(Some(&descriptor_category));

    let mut llms_json = build_llms_json(
        alias,
        &resolved.resolved_url,
        "llms.txt",
        resolved.sha256.clone(),
        resolved.etag.clone(),
        resolved.last_modified.clone(),
        parse_result,
    );

    llms_json.metadata.variant = resolved.variant.clone();
    llms_json.metadata.aliases.clone_from(&aliases);
    llms_json.metadata.tags.clone_from(&tags);
    llms_json
        .metadata
        .description
        .clone_from(&metadata_description);
    llms_json.metadata.category.clone_from(&metadata_category);
    llms_json.metadata.npm_aliases.clone_from(&npm_aliases);
    llms_json
        .metadata
        .github_aliases
        .clone_from(&github_aliases);

    let mut origin = resolved.origin.clone();
    origin.manifest.clone_from(&descriptor_input.manifest);
    llms_json.metadata.origin = origin.clone();
    storage.save_llms_json(alias, &llms_json)?;

    spinner.set_message("Persisting metadata...");
    let metadata = Source {
        url: resolved.resolved_url.clone(),
        etag: resolved.etag,
        last_modified: resolved.last_modified,
        fetched_at: Utc::now(),
        sha256: resolved.sha256,
        variant: resolved.variant,
        aliases: aliases.clone(),
        tags: tags.clone(),
        description: metadata_description.clone(),
        category: metadata_category.clone(),
        npm_aliases: npm_aliases.clone(),
        github_aliases: github_aliases.clone(),
        origin: origin.clone(),
        filter_non_english: Some(!no_language_filter),
    };
    storage.save_source_metadata(alias, &metadata)?;

    let (descriptor_url, descriptor_path) = match &origin.source_type {
        Some(SourceType::Remote { url }) => (Some(url.clone()), None),
        Some(SourceType::LocalFile { path }) => (None, Some(path.clone())),
        None => (Some(resolved.resolved_url), None),
    };

    let descriptor = SourceDescriptor {
        alias: alias.to_string(),
        name: Some(descriptor_name),
        description: Some(descriptor_description),
        category: Some(descriptor_category),
        tags,
        url: descriptor_url,
        path: descriptor_path,
        aliases,
        npm_aliases,
        github_aliases,
        origin,
    };
    storage.save_descriptor(&descriptor)?;

    spinner.set_message("Indexing content...");
    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create(&index_path)?.with_metrics(metrics);
    index.index_blocks(alias, &parse_result.heading_blocks)?;

    Ok(llms_json)
}

/// Apply language filtering to parse results
///
/// Filters out non-English heading blocks using hybrid URL-based and text-based detection.
/// Prints filtering statistics if blocks were filtered and not in quiet mode.
fn apply_language_filter(parse_result: &mut ParseResult, no_language_filter: bool, quiet: bool) {
    if no_language_filter {
        return;
    }

    let mut language_filter = LanguageFilter::new(true);

    // Filter heading blocks using both URL-based and text-based methods
    let original_count = parse_result.heading_blocks.len();
    parse_result.heading_blocks.retain(|block| {
        // First check URLs in content (fast, catches locale-based URLs)
        let urls_in_content = extract_urls_from_content(&block.content);
        let url_check = urls_in_content.is_empty()
            || urls_in_content
                .iter()
                .all(|url| language_filter.is_english_url(url));

        // Then check heading text (catches non-URL-based translations)
        let heading_check = language_filter.is_english_heading_path(&block.path);

        // Block must pass both checks to be kept
        url_check && heading_check
    });

    let filtered_count = original_count - parse_result.heading_blocks.len();
    if filtered_count > 0 && !quiet {
        println!(
            "Filtered {} non-English content blocks ({:.1}% reduction)",
            filtered_count,
            percentage(filtered_count, original_count)
        );
    }
}

fn dedupe_sorted(values: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = values
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();
    cleaned.sort();
    cleaned.dedup();
    cleaned
}

fn percentage(part: usize, total: usize) -> f64 {
    safe_percentage(part, total)
}

fn non_empty_string(value: Option<&String>) -> Option<String> {
    value
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

#[inline]
#[allow(clippy::missing_const_for_fn)]
fn is_url_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ')' | ']' | '>' | '"' | '\'' | '`')
}

#[inline]
#[allow(clippy::missing_const_for_fn)]
fn trailing_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | ';' | ':' | ')' | ']' | '>' | '"' | '\'' | '`'
    )
}

fn find_url_end(content: &str, start: usize) -> usize {
    for (offset, ch) in content[start..].char_indices() {
        if is_url_delimiter(ch) {
            return start + offset;
        }
    }
    content.len()
}

fn clean_url_slice(slice: &str) -> Option<&str> {
    let trimmed = slice.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return None;
    }

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

/// Extract URLs from markdown content using simple string parsing
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

    for prefix in ["http://", "https://"] {
        let mut look_from = 0;
        while let Some(rel) = content[look_from..].find(prefix) {
            let start = look_from + rel;
            let end = find_url_end(content, start);
            if end > start {
                if let Some(cleaned) = clean_url_slice(&content[start..end]) {
                    urls.push(cleaned.to_string());
                }
            }
            look_from = end;
        }
    }

    urls.sort();
    urls.dedup();
    urls
}

/// Check if the input appears to be a domain-only string (no protocol, no path).
///
/// Domain-only inputs are detected by:
/// - Not containing "://" (no protocol)
/// - Not starting with "http" (even without protocol prefix)
/// - Containing at least one dot (has TLD)
/// - Not containing "/" (no path component)
/// - Not starting with "." (not a dotfile)
///
/// This function is used during add command execution to determine if the input
/// should trigger domain discovery (e.g., `blz add hono.dev`).
///
/// # Examples
///
/// ```ignore
/// assert!(is_domain_only("hono.dev"));
/// assert!(is_domain_only("docs.tanstack.com"));
/// assert!(!is_domain_only("https://hono.dev"));
/// assert!(!is_domain_only("react")); // No dot = alias
/// assert!(!is_domain_only("/path/to/file"));
/// ```
#[allow(dead_code)]
#[must_use]
pub fn is_domain_only(input: &str) -> bool {
    // Empty string is not a domain
    if input.is_empty() {
        return false;
    }

    // Has protocol prefix - not domain-only
    if input.contains("://") || input.starts_with("http") {
        return false;
    }

    // Must contain at least one dot (for TLD)
    if !input.contains('.') {
        return false;
    }

    // Must not contain path separator
    if input.contains('/') {
        return false;
    }

    // Must not start with dot (dotfile)
    if input.starts_with('.') {
        return false;
    }

    true
}

/// Probe a domain for documentation sources.
///
/// Uses `blz_core::discovery::probe_domain` to check for llms.txt, llms-full.txt,
/// and sitemap.xml at the given domain.
///
/// # Arguments
///
/// * `domain` - The domain to probe (e.g., "hono.dev")
///
/// # Returns
///
/// A [`ProbeResult`] containing URLs for any discovered documentation sources.
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be constructed or network errors occur.
///
/// # Examples
///
/// ```ignore
/// let result = discover_for_domain("hono.dev").await?;
/// if let Some(url) = result.best_url() {
///     println!("Found documentation at: {}", url);
/// }
/// ```
#[allow(dead_code)]
pub async fn discover_for_domain(domain: &str) -> Result<ProbeResult> {
    probe_domain(domain).await.map_err(Into::into)
}

fn display_name_from_alias(alias: &str) -> String {
    let mut title = String::new();
    for (idx, part) in alias
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .filter(|s| !s.is_empty())
        .enumerate()
    {
        if idx > 0 {
            title.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            title.extend(first.to_uppercase());
            title.push_str(&chars.as_str().to_lowercase());
        }
    }
    if title.is_empty() {
        alias.to_string()
    } else {
        title
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls_from_content() {
        let content = r"
        # Documentation

        Check out [React docs](https://reactjs.org/docs) for more info.
        Also see https://developer.mozilla.org/en-US/docs/Web/JavaScript.
        
        Some non-English content:
        [German docs](https://de.reactjs.org/docs/getting-started.html)
        Visit https://fr.reactjs.org/tutorial for French tutorial.
        ";

        let urls = extract_urls_from_content(content);

        assert!(urls.contains(&"https://reactjs.org/docs".to_string()));
        assert!(
            urls.contains(&"https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string())
        );
        assert!(urls.contains(&"https://de.reactjs.org/docs/getting-started.html".to_string()));
        assert!(urls.contains(&"https://fr.reactjs.org/tutorial".to_string()));

        assert_eq!(urls.len(), 4);
    }

    #[test]
    fn test_extract_urls_bare_urls() {
        let content = "Visit https://example.com and http://test.org for more info.";
        let urls = extract_urls_from_content(content);

        assert!(urls.contains(&"https://example.com".to_string()));
        assert!(urls.contains(&"http://test.org".to_string()));
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_extract_urls_no_duplicates() {
        let content = "[Link](https://example.com) and https://example.com again.";
        let urls = extract_urls_from_content(content);

        assert_eq!(urls.len(), 1);
        assert!(urls.contains(&"https://example.com".to_string()));
    }

    // ============================================
    // is_domain_only tests
    // ============================================

    #[test]
    fn test_domain_only_simple_domains() {
        // Standard domains should be detected
        assert!(is_domain_only("hono.dev"));
        assert!(is_domain_only("docs.tanstack.com"));
        assert!(is_domain_only("react.dev"));
        assert!(is_domain_only("example.com"));
    }

    #[test]
    fn test_domain_only_rejects_urls() {
        // URLs with protocol should NOT be domain-only
        assert!(!is_domain_only("https://hono.dev"));
        assert!(!is_domain_only("http://hono.dev"));
        assert!(!is_domain_only("https://docs.tanstack.com"));
    }

    #[test]
    fn test_domain_only_rejects_aliases() {
        // Simple words without dots are aliases, not domains
        assert!(!is_domain_only("react"));
        assert!(!is_domain_only("bun"));
        assert!(!is_domain_only("hono"));
        assert!(!is_domain_only("tanstack"));
    }

    #[test]
    fn test_domain_only_rejects_paths() {
        // File paths should not be domain-only
        assert!(!is_domain_only("/path/to/file"));
        assert!(!is_domain_only("./local/file.txt"));
        assert!(!is_domain_only("../parent/file"));
    }

    #[test]
    fn test_domain_only_rejects_dotfiles() {
        // Dotfiles should not be domain-only
        assert!(!is_domain_only(".env"));
        assert!(!is_domain_only(".gitignore"));
        assert!(!is_domain_only(".config"));
    }

    #[test]
    fn test_domain_only_rejects_urls_with_paths() {
        // Domains with paths are NOT domain-only
        assert!(!is_domain_only("hono.dev/docs"));
        assert!(!is_domain_only("example.com/llms.txt"));
    }

    #[test]
    fn test_domain_only_edge_cases() {
        // Edge cases
        assert!(!is_domain_only("")); // Empty string
        assert!(!is_domain_only("localhost")); // No TLD
        assert!(is_domain_only("localhost.dev")); // Has TLD
    }

    // ============================================
    // AddAction and determine_add_action tests
    // ============================================

    #[test]
    fn test_determine_action_native_available() {
        let probe = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: Some("https://example.com/llms-full.txt".to_string()),
            llms_url: None,
            sitemap_url: None,
            docs_subdomain_checked: false,
        };

        let action = determine_add_action(&probe, false).unwrap();
        assert!(matches!(action, AddAction::UseNative { .. }));

        if let AddAction::UseNative { url } = action {
            assert_eq!(url, "https://example.com/llms-full.txt");
        }
    }

    #[test]
    fn test_determine_action_native_with_fallbacks_prefers_native() {
        // When llms-full.txt exists, always use it even if other sources exist
        let probe = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: Some("https://example.com/llms-full.txt".to_string()),
            llms_url: Some("https://example.com/llms.txt".to_string()),
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        let action = determine_add_action(&probe, false).unwrap();
        assert!(matches!(action, AddAction::UseNative { .. }));
    }

    #[test]
    fn test_determine_action_llms_only_non_interactive() {
        // When only llms.txt exists and non-interactive, suggest index-only
        let probe = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: Some("https://example.com/llms.txt".to_string()),
            sitemap_url: None,
            docs_subdomain_checked: false,
        };

        let action = determine_add_action(&probe, false).unwrap();
        // In non-interactive mode without --yes, should suggest IndexOnly
        assert!(matches!(action, AddAction::IndexOnly { .. }));
    }

    #[test]
    fn test_determine_action_sitemap_only_non_interactive() {
        // When only sitemap exists, cannot index without generation capability
        let probe = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        // Without llms.txt or llms-full.txt, sitemap alone cannot provide an index
        let result = determine_add_action(&probe, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_determine_action_no_sources() {
        let probe = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: None,
            docs_subdomain_checked: true,
        };

        let result = determine_add_action(&probe, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No documentation sources found"));
    }

    #[test]
    fn test_add_action_display() {
        let native = AddAction::UseNative {
            url: "https://example.com/llms-full.txt".to_string(),
        };
        let generate = AddAction::Generate { url_count: 42 };
        let index_only = AddAction::IndexOnly {
            url: "https://example.com/llms.txt".to_string(),
        };
        let cancel = AddAction::Cancel;

        // Verify that Display is implemented correctly
        assert!(format!("{native}").contains("llms-full.txt"));
        assert!(format!("{generate}").contains("42"));
        assert!(format!("{index_only}").contains("llms.txt"));
        assert!(format!("{cancel}").contains("Cancel"));
    }
}
