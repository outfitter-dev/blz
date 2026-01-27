//! Source management tools for listing and adding documentation sources

use blz_core::{Registry, SourceDescriptor, Storage};
use serde::{Deserialize, Serialize};

use crate::{cache, error::McpError, error::McpResult, types::IndexCache};

/// Maximum allowed alias length
const MAX_ALIAS_LEN: usize = 64;

/// Parameters for list-sources tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSourcesParams {
    /// Optional filter by source kind (installed/registry/all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Optional search query to filter sources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// Parameters for source-add tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAddParams {
    /// Alias for the source
    pub alias: String,

    /// Optional URL override (if not from registry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Force override if source already exists
    #[serde(default)]
    pub force: bool,
}

/// Output from list-sources
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSourcesOutput {
    /// List of available sources
    pub sources: Vec<SourceEntry>,
}

/// Individual source entry
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEntry {
    /// Source alias
    pub alias: String,
    /// Source kind: "installed" or "registry"
    pub kind: String,
    /// URL to the llms.txt file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Last update timestamp in RFC3339 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// Suggested CLI command to add this source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<String>,
}

/// Output from source-add
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAddOutput {
    /// Alias of the added source
    pub alias: String,
    /// URL of the added source
    pub url: String,
    /// Success message
    pub message: String,
}

/// Validate alias format
fn validate_alias(alias: &str) -> McpResult<()> {
    if alias.is_empty() {
        return Err(McpError::InvalidParams("Alias cannot be empty".to_string()));
    }

    if alias.len() > MAX_ALIAS_LEN {
        return Err(McpError::InvalidParams(format!(
            "Alias exceeds maximum length of {MAX_ALIAS_LEN} characters"
        )));
    }

    if !alias
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
    {
        return Err(McpError::InvalidParams(
            "Alias must start with a letter (a-z, A-Z)".to_string(),
        ));
    }

    if !alias
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(McpError::InvalidParams(
            "Alias can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }

    Ok(())
}

/// Validate URL format
fn validate_url(url: &str) -> McpResult<()> {
    if url.is_empty() {
        return Err(McpError::InvalidParams("URL cannot be empty".to_string()));
    }

    let lower = url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(McpError::InvalidParams(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(())
}

/// Resolve the source URL from params or registry lookup.
fn resolve_source_url(params: &SourceAddParams) -> McpResult<String> {
    if let Some(ref url) = params.url {
        validate_url(url)?;
        Ok(url.clone())
    } else {
        let registry = Registry::new();
        let search_results = registry.search(&params.alias);

        let entry = search_results
            .iter()
            .find(|result| result.entry.slug == params.alias)
            .or_else(|| search_results.first())
            .ok_or_else(|| McpError::SourceNotFound(params.alias.clone()))?;

        Ok(entry.entry.llms_url.clone())
    }
}

/// Result of fetching source content.
struct FetchedContent {
    content: String,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
}

/// Fetch source content from URL.
async fn fetch_source_content(url: &str) -> McpResult<FetchedContent> {
    let fetcher = blz_core::Fetcher::new()
        .map_err(|e| McpError::Internal(format!("Failed to create fetcher: {e}")))?;

    let fetch_result = fetcher
        .fetch_with_cache(url, None, None)
        .await
        .map_err(|e| McpError::Internal(format!("Failed to fetch source: {e}")))?;

    match fetch_result {
        blz_core::FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => Ok(FetchedContent {
            content,
            sha256,
            etag,
            last_modified,
        }),
        blz_core::FetchResult::NotModified { .. } => Err(McpError::Internal(
            "Server returned 304 Not Modified on initial fetch".to_string(),
        )),
    }
}

/// Build the `LlmsJson` structure for a new source.
fn build_source_metadata(
    alias: &str,
    url: &str,
    fetched: &FetchedContent,
    parse_result: &blz_core::ParseResult,
) -> blz_core::LlmsJson {
    blz_core::LlmsJson {
        source: alias.to_string(),
        metadata: blz_core::Source {
            url: url.to_string(),
            etag: fetched.etag.clone(),
            last_modified: fetched.last_modified.clone(),
            fetched_at: chrono::Utc::now(),
            sha256: fetched.sha256.clone(),
            variant: blz_core::SourceVariant::Llms,
            aliases: Vec::new(),
            tags: Vec::new(),
            description: None,
            category: None,
            npm_aliases: Vec::new(),
            github_aliases: Vec::new(),
            origin: blz_core::SourceOrigin {
                manifest: None,
                source_type: Some(blz_core::SourceType::Remote {
                    url: url.to_string(),
                }),
            },
            filter_non_english: None,
        },
        toc: parse_result.toc.clone(),
        files: vec![blz_core::FileInfo {
            path: "llms.txt".to_string(),
            sha256: fetched.sha256.clone(),
        }],
        line_index: blz_core::LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics.clone(),
        parse_meta: Some(blz_core::ParseMeta {
            parser_version: 1,
            segmentation: "structured".to_string(),
        }),
        filter_stats: None,
    }
}

/// Persist source files to storage (llms.txt, llms.json, metadata, descriptor).
fn persist_source_files(
    storage: &Storage,
    alias: &str,
    content: &str,
    llms_json: &blz_core::LlmsJson,
) -> McpResult<()> {
    storage
        .save_llms_txt(alias, content)
        .map_err(|e| McpError::Internal(format!("Failed to save llms.txt: {e}")))?;

    storage
        .save_llms_json(alias, llms_json)
        .map_err(|e| McpError::Internal(format!("Failed to save llms.json: {e}")))?;

    storage
        .save_source_metadata(alias, &llms_json.metadata)
        .map_err(|e| McpError::Internal(format!("Failed to save source metadata: {e}")))?;

    let descriptor = SourceDescriptor::from_source(alias, &llms_json.metadata);
    storage
        .save_descriptor(&descriptor)
        .map_err(|e| McpError::Internal(format!("Failed to save source descriptor: {e}")))?;

    Ok(())
}

/// Build and populate the search index for a source.
fn build_source_index(
    storage: &Storage,
    alias: &str,
    heading_blocks: &[blz_core::HeadingBlock],
) -> McpResult<()> {
    let index_path = storage
        .index_dir(alias)
        .map_err(|e| McpError::Internal(format!("Failed to get index directory: {e}")))?;

    let index = blz_core::SearchIndex::create(&index_path)
        .map_err(|e| McpError::Internal(format!("Failed to create search index: {e}")))?;

    index
        .index_blocks(alias, heading_blocks)
        .map_err(|e| McpError::Internal(format!("Failed to index blocks: {e}")))?;

    Ok(())
}

/// Handle list-sources tool
#[tracing::instrument(skip(storage))]
pub async fn handle_list_sources(
    params: ListSourcesParams,
    storage: &Storage,
) -> McpResult<ListSourcesOutput> {
    tracing::debug!(?params, "listing sources");

    // Validate kind parameter if present
    if let Some(ref kind) = params.kind {
        if kind != "installed" && kind != "registry" && kind != "all" {
            return Err(McpError::InvalidParams(format!(
                "Invalid kind '{kind}'. Must be 'installed', 'registry', or 'all'"
            )));
        }
    }

    let mut sources = Vec::new();

    // Determine which kinds to include
    let include_installed = params
        .kind
        .as_ref()
        .is_none_or(|k| k == "installed" || k == "all");
    let include_registry = params
        .kind
        .as_ref()
        .is_none_or(|k| k == "registry" || k == "all");

    // Get installed sources
    if include_installed {
        let installed = storage.list_sources();
        for alias in installed {
            // Apply query filter if present
            if let Some(ref query) = params.query {
                if !alias.to_lowercase().contains(&query.to_lowercase()) {
                    continue;
                }
            }

            // Get metadata for last_updated
            let last_updated = match storage.load_source_metadata(&alias) {
                Ok(Some(meta)) => Some(meta.fetched_at.to_rfc3339()),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(alias = %alias, error = %e, "failed to load metadata");
                    None
                },
            };

            sources.push(SourceEntry {
                alias,
                kind: "installed".to_string(),
                url: None,
                description: None,
                last_updated,
                suggested_command: None,
            });
        }
    }

    // Get registry sources
    if include_registry {
        let registry = Registry::new();
        let registry_entries = params.query.as_ref().map_or_else(
            || registry.all_entries().to_vec(),
            |query| {
                registry
                    .search(query)
                    .into_iter()
                    .map(|result| result.entry)
                    .collect()
            },
        );

        for entry in registry_entries {
            // Skip if already installed (unless explicitly asking for registry)
            if params.kind.as_ref().is_none_or(|k| k != "registry") && storage.exists(&entry.slug) {
                continue;
            }

            sources.push(SourceEntry {
                alias: entry.slug.clone(),
                kind: "registry".to_string(),
                url: Some(entry.llms_url.clone()),
                description: Some(entry.description.clone()),
                last_updated: None,
                suggested_command: Some(format!("blz add {}", entry.slug)),
            });
        }
    }

    tracing::debug!(count = sources.len(), "listed sources");

    Ok(ListSourcesOutput { sources })
}

/// Handle source-add tool
#[tracing::instrument(skip(storage, index_cache))]
pub async fn handle_source_add(
    params: SourceAddParams,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<SourceAddOutput> {
    tracing::debug!(?params, "adding source");

    validate_alias(&params.alias)?;

    if storage.exists(&params.alias) && !params.force {
        return Err(McpError::SourceExists(params.alias.clone()));
    }

    let url = resolve_source_url(&params)?;
    tracing::info!(alias = %params.alias, url = %url, "adding source");

    let fetched = fetch_source_content(&url).await?;

    let mut parser = blz_core::MarkdownParser::new()
        .map_err(|e| McpError::Internal(format!("Failed to create parser: {e}")))?;

    let parse_result = parser
        .parse(&fetched.content)
        .map_err(|e| McpError::Internal(format!("Failed to parse markdown: {e}")))?;

    let llms_json = build_source_metadata(&params.alias, &url, &fetched, &parse_result);

    persist_source_files(storage, &params.alias, &fetched.content, &llms_json)?;
    build_source_index(storage, &params.alias, &parse_result.heading_blocks)?;

    tracing::info!(
        alias = %params.alias,
        lines = parse_result.line_count,
        headings = parse_result.heading_blocks.len(),
        "source added successfully"
    );

    cache::invalidate_cache(index_cache, &params.alias).await;

    let message = if params.force {
        format!("Source '{}' updated successfully", params.alias)
    } else {
        format!("Source '{}' added successfully", params.alias)
    };

    Ok(SourceAddOutput {
        alias: params.alias,
        url,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_alias_valid() {
        assert!(validate_alias("react").is_ok());
        assert!(validate_alias("my-tool").is_ok());
        assert!(validate_alias("tool_123").is_ok());
        assert!(validate_alias("ABC123").is_ok());
    }

    #[test]
    fn test_validate_alias_invalid() {
        assert!(validate_alias("").is_err());
        assert!(validate_alias("tool with spaces").is_err());
        assert!(validate_alias("tool/slash").is_err());
        assert!(validate_alias("tool@special").is_err());
        assert!(validate_alias(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com/llms.txt").is_ok());
        assert!(validate_url("http://example.com/llms.txt").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("ftp://example.com/llms.txt").is_err());
        assert!(validate_url("example.com/llms.txt").is_err());
        assert!(validate_url("//example.com/llms.txt").is_err());
    }

    mod logic_tests {
        use super::*;

        #[test]
        fn test_alias_length_boundary() {
            // Test exact boundary for MAX_ALIAS_LEN
            let valid_64 = "a".repeat(64);
            let invalid_65 = "a".repeat(65);

            assert!(validate_alias(&valid_64).is_ok());
            assert!(validate_alias(&invalid_65).is_err());
        }

        #[test]
        fn test_alias_numeric_only() {
            // Numeric-only aliases are now invalid - must start with letter
            assert!(validate_alias("123").is_err());
            assert!(validate_alias("42").is_err());
            // But aliases with numbers after letters are valid
            assert!(validate_alias("test123").is_ok());
            assert!(validate_alias("v42").is_ok());
        }

        #[test]
        fn test_alias_starting_with_dash() {
            // Aliases cannot start with dash/underscore - must start with letter
            assert!(validate_alias("-test").is_err());
            assert!(validate_alias("_test").is_err());
            // But they can contain them after the first letter
            assert!(validate_alias("my-test").is_ok());
            assert!(validate_alias("my_test").is_ok());
        }

        #[test]
        fn test_url_case_sensitivity() {
            // URLs should be case-insensitive for protocol
            assert!(validate_url("HTTP://example.com").is_ok());
            assert!(validate_url("HTTPS://example.com").is_ok());
            assert!(validate_url("Http://example.com").is_ok());
        }

        #[test]
        fn test_url_with_port() {
            assert!(validate_url("https://example.com:8080/llms.txt").is_ok());
            assert!(validate_url("http://localhost:3000/docs").is_ok());
        }

        #[test]
        fn test_url_with_query_params() {
            assert!(validate_url("https://example.com/llms.txt?version=1.0").is_ok());
        }

        #[test]
        fn test_url_with_fragment() {
            assert!(validate_url("https://example.com/llms.txt#section").is_ok());
        }

        #[test]
        fn test_url_edge_cases() {
            // These should all fail
            assert!(validate_url("").is_err());
            assert!(validate_url("not-a-url").is_err());
            assert!(validate_url("file:///local/path").is_err());
        }
    }
}
