//! Discover documentation sources prompt
//!
//! Helps agents find and add relevant documentation sources based on technologies.

use blz_core::{Registry, Storage};
use rmcp::model::{PromptMessage, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{McpError, McpResult};

/// Parameters for discover-docs prompt
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDocsParams {
    /// Comma-separated list of technologies to discover documentation for
    pub technologies: String,
}

/// Output from discover-docs prompt
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDocsOutput {
    /// Messages to display to the agent
    pub messages: Vec<PromptMessage>,
}

/// Handle discover-docs prompt
///
/// Three-step flow:
/// 1. List matching installed sources
/// 2. Surface registry-only entries (sources in registry but not installed)
/// 3. Suggest `find` tool usage for installed sources
///
/// Cap output at 3 messages total.
#[tracing::instrument(skip(storage))]
#[allow(clippy::too_many_lines)] // Complex flow with multiple message generation steps
pub fn handle_discover_docs(
    params: &DiscoverDocsParams,
    storage: &Storage,
) -> McpResult<DiscoverDocsOutput> {
    tracing::debug!(?params, "handling discover-docs prompt");

    let technologies: Vec<String> = params
        .technologies
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if technologies.is_empty() {
        return Err(McpError::Internal(
            "At least one technology must be provided".to_string(),
        ));
    }

    let mut messages = Vec::new();
    let registry = Registry::new();

    // Step 1: List matching installed sources
    let installed_sources: Vec<String> = storage
        .list_sources()
        .into_iter()
        .filter(|alias| {
            let alias_lower = alias.to_lowercase();
            technologies.iter().any(|tech| alias_lower.contains(tech))
        })
        .collect();

    if !installed_sources.is_empty() {
        let installed_list = installed_sources
            .iter()
            .map(|alias| format!("- {alias}"))
            .collect::<Vec<_>>()
            .join("\n");

        messages.push(PromptMessage::new_text(
            PromptMessageRole::Assistant,
            format!(
                "Found {} installed source(s) matching your technologies:\n\n{}",
                installed_sources.len(),
                installed_list
            ),
        ));
    }

    // Step 2: Surface registry-only entries (not installed)
    let mut registry_only = Vec::new();
    for tech in &technologies {
        let search_results = registry.search(tech);
        for result in search_results {
            // Skip if already installed
            if storage.exists(&result.entry.slug) {
                continue;
            }
            // Avoid duplicates
            if registry_only
                .iter()
                .any(|(slug, _, _)| slug == &result.entry.slug)
            {
                continue;
            }
            registry_only.push((
                result.entry.slug.clone(),
                result.entry.llms_url.clone(),
                result.entry.description.clone(),
            ));
        }
    }

    if !registry_only.is_empty() {
        let registry_list = registry_only
            .iter()
            .take(10) // Limit to top 10 to keep message size reasonable
            .map(|(slug, _url, desc)| format!("- **{slug}**: {desc}"))
            .collect::<Vec<_>>()
            .join("\n");

        let add_command = registry_only
            .first()
            .map(|(slug, _, _)| format!("blz add {slug}"))
            .unwrap_or_default();

        messages.push(PromptMessage::new_text(
            PromptMessageRole::Assistant,
            format!(
                "Found {} source(s) in the registry (not yet installed):\n\n{}\n\nTo add a source, use the `source-add` tool or run: `{}`",
                registry_only.len(),
                registry_list,
                add_command
            ),
        ));
    }

    // Step 3: Suggest `find` tool usage for installed sources
    if let Some(example_source) = installed_sources.first() {
        messages.push(PromptMessage::new_text(
            PromptMessageRole::Assistant,
            format!(
                "To search within installed sources, use the `find` tool:\n\n```json\n{{\n  \"query\": \"your search query\",\n  \"source\": \"{example_source}\"\n}}\n```"
            ),
        ));
    } else if registry_only.is_empty() {
        // No results at all
        messages.push(PromptMessage::new_text(
            PromptMessageRole::Assistant,
            format!(
                "No documentation sources found for: {}\n\nTry different technology names or check the registry with the `list-sources` tool.",
                params.technologies
            ),
        ));
    }

    // Cap at 3 messages
    messages.truncate(3);

    tracing::debug!(message_count = messages.len(), "discover-docs complete");

    Ok(DiscoverDocsOutput { messages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage =
            Storage::with_paths(temp_dir.path().join("config"), temp_dir.path().join("data"))
                .expect("Failed to create storage");
        (storage, temp_dir)
    }

    #[test]
    fn test_discover_docs_empty_technologies() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: String::new(),
        };

        let result = handle_discover_docs(&params, &storage);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("At least one technology"));
        }
    }

    #[test]
    fn test_discover_docs_no_matches() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "nonexistent-tech-xyz".to_string(),
        };

        let result = handle_discover_docs(&params, &storage).expect("Should succeed");
        assert_eq!(result.messages.len(), 1);

        // Should get a "no results" message
        let message = &result.messages[0];
        match &message.content {
            rmcp::model::PromptMessageContent::Text { text } => {
                assert!(text.contains("No documentation sources found"));
            },
            _ => {
                assert!(
                    matches!(
                        &message.content,
                        rmcp::model::PromptMessageContent::Text { .. }
                    ),
                    "Expected text content"
                );
            },
        }
    }

    #[test]
    fn test_discover_docs_registry_matches() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "react".to_string(),
        };

        let result = handle_discover_docs(&params, &storage).expect("Should succeed");
        // Should have at least one message about registry results
        assert!(!result.messages.is_empty());
        assert!(result.messages.len() <= 3);

        // First message should mention registry sources
        let message = &result.messages[0];
        match &message.content {
            rmcp::model::PromptMessageContent::Text { text } => {
                assert!(text.contains("registry") || text.contains("source"));
            },
            _ => {
                assert!(
                    matches!(
                        &message.content,
                        rmcp::model::PromptMessageContent::Text { .. }
                    ),
                    "Expected text content"
                );
            },
        }
    }

    #[test]
    fn test_discover_docs_multiple_technologies() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "react, vue, angular".to_string(),
        };

        let result = handle_discover_docs(&params, &storage).expect("Should succeed");
        // Should have messages
        assert!(!result.messages.is_empty());
        assert!(result.messages.len() <= 3);
    }

    #[test]
    fn test_discover_docs_message_cap() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "javascript, typescript, react, vue".to_string(),
        };

        let result = handle_discover_docs(&params, &storage).expect("Should succeed");
        // Should never exceed 3 messages
        assert!(result.messages.len() <= 3);
    }

    #[test]
    fn test_discover_docs_params_deserialization() {
        let json = r#"{"technologies": "rust, python"}"#;
        let params: DiscoverDocsParams = serde_json::from_str(json).expect("Should parse JSON");
        assert_eq!(params.technologies, "rust, python");
    }

    #[test]
    fn test_discover_docs_case_insensitive() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "REACT".to_string(),
        };

        let result = handle_discover_docs(&params, &storage);
        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_docs_whitespace_handling() {
        let (storage, _temp_dir) = create_test_storage();
        let params = DiscoverDocsParams {
            technologies: "  react  ,  vue  ".to_string(),
        };

        let result = handle_discover_docs(&params, &storage);
        assert!(result.is_ok());
    }
}
