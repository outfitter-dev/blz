//! Registry resource handler for BLZ MCP
//!
//! Exposes the bundled registry via `blz://registry` URI.

use blz_core::Registry;
use serde_json::json;

use crate::error::{McpError, McpResult};

/// Parse a registry resource URI
///
/// Supports both custom scheme (`blz://registry`) and fallback
/// (`resource://blz/registry`)
fn parse_registry_uri(uri: &str) -> McpResult<()> {
    // Try custom scheme first
    if uri == "blz://registry" {
        return Ok(());
    }

    // Try fallback scheme
    if uri == "resource://blz/registry" {
        tracing::debug!("using fallback resource:// scheme for registry URI");
        return Ok(());
    }

    Err(McpError::InvalidParams(format!(
        "Invalid registry resource URI: {uri}"
    )))
}

/// Handle registry resource read request
///
/// Returns JSON array of all registry sources with:
/// - alias (slug)
/// - url (`llms_url`)
/// - category (inferred from first word of description or "general")
#[tracing::instrument]
pub async fn handle_registry_resource(uri: &str) -> McpResult<serde_json::Value> {
    tracing::debug!(uri = %uri, "reading registry resource");

    parse_registry_uri(uri)?;

    let registry = Registry::new();
    let entries = registry.all_entries();

    let sources: Vec<_> = entries
        .iter()
        .map(|entry| {
            // Infer category from description (first word before space/colon/dash)
            let first_word = entry
                .description
                .split(&[' ', ':', '-'][..])
                .next()
                .unwrap_or("");
            let category = if first_word.is_empty() {
                "general".to_string()
            } else {
                first_word.to_lowercase()
            };

            json!({
                "alias": entry.slug,
                "url": entry.llms_url,
                "category": category,
            })
        })
        .collect();

    let payload = json!({
        "sources": sources,
    });

    tracing::debug!(count = sources.len(), "registry resource retrieved");

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry_uri_custom_scheme() {
        let uri = "blz://registry";
        let result = parse_registry_uri(uri);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_registry_uri_fallback_scheme() {
        let uri = "resource://blz/registry";
        let result = parse_registry_uri(uri);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_registry_uri_invalid() {
        let uri = "blz://sources/bun";
        let result = parse_registry_uri(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_registry_uri_invalid_fallback() {
        let uri = "resource://blz/sources/bun";
        let result = parse_registry_uri(uri);
        assert!(result.is_err());
    }
}
