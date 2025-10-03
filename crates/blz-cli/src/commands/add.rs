//! Add command implementation

use anyhow::Result;
use blz_core::{
    Fetcher, MarkdownParser, PerformanceMetrics, SearchIndex, Source, SourceDescriptor,
    SourceOrigin, SourceType, SourceVariant, Storage,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs as sync_fs;
use std::path::Path;
use tokio::fs as async_fs;
use url::Url;

use crate::utils::count_headings;
use crate::utils::json_builder::build_llms_json;
use crate::utils::url_resolver;
use crate::utils::validation::{normalize_alias, validate_alias};

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

#[derive(Debug, Clone, Default)]
pub struct DescriptorInput {
    name: Option<String>,
    description: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    aliases: Vec<String>,
    npm_aliases: Vec<String>,
    github_aliases: Vec<String>,
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

pub struct AddRequest {
    pub alias: String,
    pub url: String,
    pub descriptor: DescriptorInput,
    pub dry_run: bool,
    pub quiet: bool,
    pub metrics: PerformanceMetrics,
}

impl AddRequest {
    pub fn new(
        alias: impl Into<String>,
        url: impl Into<String>,
        descriptor: DescriptorInput,
        dry_run: bool,
        quiet: bool,
        metrics: PerformanceMetrics,
    ) -> Self {
        Self {
            alias: alias.into(),
            url: url.into(),
            descriptor,
            dry_run,
            quiet,
            metrics,
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

/// Add a new documentation source
///
/// # Arguments
/// Execute the add flow given a prepared request.
pub async fn execute(request: AddRequest) -> Result<()> {
    let AddRequest {
        alias,
        url,
        descriptor,
        dry_run,
        quiet,
        metrics,
    } = request;

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
        dry_run,
        quiet,
        fetcher,
        metrics,
    )
    .await
}

pub async fn execute_manifest(
    manifest_path: &Path,
    only: &[String],
    auto_yes: bool,
    dry_run: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let _ = auto_yes;
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
            name: non_empty_string(&Some(entry.name.clone())),
            description: entry.description.clone().map(|s| s.trim().to_string()),
            category: non_empty_string(&Some(entry.category.clone())),
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
                    url.to_string(),
                    descriptor_input.clone(),
                    dry_run,
                    quiet,
                    metrics.clone(),
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

#[allow(clippy::too_many_lines)]
async fn fetch_and_index(
    alias: &str,
    url: &str,
    descriptor_input: DescriptorInput,
    dry_run: bool,
    quiet: bool,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
) -> Result<()> {
    // Check if source already exists (validate even in dry-run mode)
    let storage = Storage::new()?;
    if storage.exists(alias) {
        anyhow::bail!(
            "Source '{}' already exists. Use 'blz update {}' or choose a different alias.",
            alias,
            alias
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
        spinner.finish_and_clear();
        eprintln!(
            "{} This appears to be a navigation index only ({} lines).\n\
             BLZ works best with full documentation files (llms-full.txt).",
            "⚠".yellow(),
            resolved.line_count
        );
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
    let parse_result = parser.parse(&content)?;

    // In dry-run mode, analyze content and output JSON instead of indexing
    if dry_run {
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
        spinner.finish_and_clear();
        return Ok(());
    }
    let resolved_addition = ResolvedAddition {
        content,
        sha256,
        etag,
        last_modified,
        resolved_url: resolved.final_url.clone(),
        variant: resolved.variant,
        origin: SourceOrigin {
            manifest: None,
            source_type: Some(SourceType::Remote {
                url: resolved.final_url.clone(),
            }),
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

async fn add_local_source(
    alias: &str,
    path: &Path,
    descriptor_input: DescriptorInput,
    dry_run: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let storage = Storage::new()?;
    if storage.exists(alias) {
        anyhow::bail!(
            "Source '{}' already exists. Use 'blz update {}' or choose a different alias.",
            alias,
            alias
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
    let sha256 = format!("{:x}", hasher.finalize());

    spinner.set_message("Parsing markdown...");
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;

    if dry_run {
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

fn finalize_add(
    storage: &Storage,
    alias: &str,
    resolved: ResolvedAddition,
    mut descriptor_input: DescriptorInput,
    parse_result: &blz_core::ParseResult,
    spinner: &ProgressBar,
    metrics: PerformanceMetrics,
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

    let metadata_description = non_empty_string(&Some(descriptor_description.clone()));
    let metadata_category = non_empty_string(&Some(descriptor_category.clone()));

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
    origin.manifest = descriptor_input.manifest.clone();
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
    };
    storage.save_source_metadata(alias, &metadata)?;

    let (descriptor_url, descriptor_path) = match &origin.source_type {
        Some(SourceType::Remote { url }) => (Some(url.clone()), None),
        Some(SourceType::LocalFile { path }) => (None, Some(path.clone())),
        None => (Some(resolved.resolved_url.clone()), None),
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

fn non_empty_string(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
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
