//! Source resource handler for BLZ MCP
//!
//! Exposes individual source metadata via `blz://sources/{alias}` URIs.

use blz_core::Storage;
use serde_json::json;

use crate::error::{McpError, McpResult};

/// Parse a source resource URI and extract the alias
///
/// Supports both custom scheme (`blz://sources/{alias}`) and fallback
/// (`resource://blz/sources/{alias}`)
fn parse_source_uri(uri: &str) -> McpResult<String> {
    // Try custom scheme first
    if let Some(alias) = uri.strip_prefix("blz://sources/") {
        let normalized = normalize_alias(alias);
        if normalized.is_empty() {
            return Err(McpError::InvalidParams(
                "Invalid source URI: alias cannot be empty after normalization".to_string(),
            ));
        }
        return Ok(normalized);
    }

    // Try fallback scheme
    if let Some(alias) = uri.strip_prefix("resource://blz/sources/") {
        tracing::debug!("using fallback resource:// scheme for source URI");
        let normalized = normalize_alias(alias);
        if normalized.is_empty() {
            return Err(McpError::InvalidParams(
                "Invalid source URI: alias cannot be empty after normalization".to_string(),
            ));
        }
        return Ok(normalized);
    }

    Err(McpError::InvalidParams(format!(
        "Invalid source resource URI: {uri}"
    )))
}

/// Normalize alias to lowercase and sanitize special characters
///
/// Removes all characters except alphanumeric, hyphens, and underscores
/// to prevent path traversal and special character injection.
///
/// # Security
///
/// This function provides defense-in-depth against path traversal attempts
/// by stripping all filesystem-related special characters including:
/// - Path separators (/, \)
/// - Parent directory references (..)
/// - Special characters that could be filesystem control chars
fn normalize_alias(alias: &str) -> String {
    alias
        .to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Handle source resource read request
///
/// Returns JSON metadata for the specified source including:
/// - alias
/// - url
/// - fetchedAt (ISO 8601)
/// - totalLines (from parsed document)
/// - headings (section count)
/// - lastUpdated (same as fetchedAt)
/// - category (derived from tags if available)
#[tracing::instrument(skip(storage))]
pub async fn handle_source_resource(uri: &str, storage: &Storage) -> McpResult<serde_json::Value> {
    tracing::debug!(uri = %uri, "reading source resource");

    let alias = parse_source_uri(uri)?;

    // Load source metadata
    let source_meta = storage
        .load_source_metadata(&alias)?
        .ok_or_else(|| McpError::SourceNotFound(alias.clone()))?;

    // Load parsed document to get line and heading counts
    let parsed_doc = storage.load_llms_json(&alias)?;

    let total_lines = parsed_doc.line_index.total_lines;
    let headings = parsed_doc.toc.len();

    // Extract category from tags (use first tag or "uncategorized")
    let category = source_meta
        .tags
        .first()
        .map_or("uncategorized", String::as_str);

    let payload = json!({
        "alias": alias,
        "url": source_meta.url,
        "fetchedAt": source_meta.fetched_at.to_rfc3339(),
        "totalLines": total_lines,
        "headings": headings,
        "lastUpdated": source_meta.fetched_at.to_rfc3339(),
        "category": category,
    });

    tracing::debug!(
        alias = %alias,
        total_lines = total_lines,
        headings = headings,
        "source resource retrieved"
    );

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_uri_custom_scheme() {
        let uri = "blz://sources/bun";
        let result = parse_source_uri(uri);
        assert!(result.is_ok());
        if let Ok(alias) = result {
            assert_eq!(alias, "bun");
        }
    }

    #[test]
    fn test_parse_source_uri_fallback_scheme() {
        let uri = "resource://blz/sources/react";
        let result = parse_source_uri(uri);
        assert!(result.is_ok());
        if let Ok(alias) = result {
            assert_eq!(alias, "react");
        }
    }

    #[test]
    fn test_parse_source_uri_invalid() {
        let uri = "https://example.com/source";
        let result = parse_source_uri(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_alias() {
        assert_eq!(normalize_alias("React"), "react");
        assert_eq!(normalize_alias("  BUN  "), "bun");
        assert_eq!(normalize_alias("next-js"), "next-js");
    }

    #[test]
    fn test_normalize_alias_path_traversal() {
        // Path traversal attempts should have separators removed
        assert_eq!(normalize_alias("../../../etc/passwd"), "etcpasswd");
        assert_eq!(normalize_alias("foo/../bar"), "foobar");
        assert_eq!(normalize_alias("./config"), "config");
        assert_eq!(normalize_alias("../../"), "");
    }

    #[test]
    fn test_normalize_alias_special_chars() {
        // Special characters should be filtered
        assert_eq!(normalize_alias("foo/bar"), "foobar");
        assert_eq!(normalize_alias("test@example.com"), "testexamplecom");
        assert_eq!(normalize_alias("foo:bar"), "foobar");
        assert_eq!(normalize_alias("foo\\bar"), "foobar"); // Windows path separator
        assert_eq!(normalize_alias("foo|bar"), "foobar");
    }

    #[test]
    fn test_parse_source_uri_empty_alias() {
        // Empty aliases after normalization should be rejected
        assert!(parse_source_uri("blz://sources/").is_err());
        assert!(parse_source_uri("blz://sources/   ").is_err());
        assert!(parse_source_uri("blz://sources/...").is_err()); // Becomes empty
        assert!(parse_source_uri("blz://sources///").is_err());
        assert!(parse_source_uri("resource://blz/sources/").is_err());
        assert!(parse_source_uri("resource://blz/sources/@@@").is_err()); // All special chars
    }

    #[test]
    fn test_parse_source_uri_path_traversal_blocked() {
        // Path traversal attempts should be sanitized, not rejected
        let result = parse_source_uri("blz://sources/../../../etc/passwd");
        assert!(result.is_ok());
        if let Ok(alias) = result {
            assert_eq!(alias, "etcpasswd"); // Sanitized, not traversed
        }

        let result = parse_source_uri("blz://sources/foo/../bar");
        assert!(result.is_ok());
        if let Ok(alias) = result {
            assert_eq!(alias, "foobar");
        }
    }

    #[test]
    fn test_normalize_alias_backward_compatibility() {
        // Ensure existing valid aliases still work
        assert_eq!(normalize_alias("bun"), "bun");
        assert_eq!(normalize_alias("React"), "react");
        assert_eq!(normalize_alias("next-js"), "next-js");
        assert_eq!(normalize_alias("lodash_utils"), "lodash_utils");
        assert_eq!(normalize_alias("vue3"), "vue3");
    }
}
