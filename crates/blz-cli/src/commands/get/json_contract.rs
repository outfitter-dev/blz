#![allow(dead_code)]

//! Data structures representing the upcoming `blz get` JSON contract.
//!
//! These types mirror the shapes outlined in Linear issue BLZ-163 so that
//! subsequent work can focus on wiring the CLI logic without re-deriving the
//! schema. They are not yet used by the `get` command but will replace the
//! existing ad-hoc `serde_json::json!` builders in later slices.

use serde::{Deserialize, Serialize};

/// A contiguous range of snippet lines returned from a source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetRange {
    /// Inclusive line where the snippet starts.
    pub line_start: usize,
    /// Inclusive line where the snippet ends.
    pub line_end: usize,
    /// The textual content for the range.
    pub snippet: String,
}

/// Represents a single retrieval request for a source.
///
/// Depending on the context, either `snippet`/`line_start`/`line_end` will be
/// populated (single range) or multiple `ranges` entries will be provided. The
/// optional fields align with the contract notes in BLZ-163.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetRequest {
    pub alias: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranges: Option<Vec<SnippetRange>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_applied: Option<usize>,
}

/// Execution metadata describing the overall `blz get` invocation.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_sources: Option<usize>,
}

/// Top-level payload for JSON/JSONL outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponse {
    pub requests: Vec<SnippetRequest>,
    #[serde(flatten)]
    pub metadata: ExecutionMetadata,
}

#[cfg(test)]
mod tests {
    use super::{ExecutionMetadata, GetResponse, SnippetRange, SnippetRequest};
    use serde_json::{Value, json};

    #[test]
    fn single_range_serializes_with_snippet_fields() {
        let response = GetResponse {
            requests: vec![SnippetRequest {
                alias: "bun".to_string(),
                source: "bun".to_string(),
                snippet: Some("# Workspaces".to_string()),
                line_start: Some(7_105),
                line_end: Some(7_164),
                ranges: None,
                checksum: Some("checksum123".to_string()),
                context_applied: Some(2),
            }],
            metadata: ExecutionMetadata {
                execution_time_ms: Some(12),
                total_sources: Some(1),
            },
        };

        let value = serde_json::to_value(response).expect("serialization should succeed");
        let expected: Value = json!({
            "requests": [{
                "alias": "bun",
                "source": "bun",
                "snippet": "# Workspaces",
                "lineStart": 7105,
                "lineEnd": 7164,
                "checksum": "checksum123",
                "contextApplied": 2
            }],
            "executionTimeMs": 12,
            "totalSources": 1
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn multi_range_serializes_under_ranges_array() {
        let response = GetResponse {
            requests: vec![SnippetRequest {
                alias: "bun".to_string(),
                source: "bun".to_string(),
                snippet: None,
                line_start: None,
                line_end: None,
                ranges: Some(vec![
                    SnippetRange {
                        line_start: 7_105,
                        line_end: 7_164,
                        snippet: "# Workspaces".to_string(),
                    },
                    SnippetRange {
                        line_start: 26_925,
                        line_end: 26_961,
                        snippet: "### TypeScript Support".to_string(),
                    },
                ]),
                checksum: Some("checksum123".to_string()),
                context_applied: None,
            }],
            metadata: ExecutionMetadata::default(),
        };

        let value = serde_json::to_value(response).expect("serialization should succeed");
        let expected: Value = json!({
            "requests": [{
                "alias": "bun",
                "source": "bun",
                "ranges": [
                    {
                        "lineStart": 7105,
                        "lineEnd": 7164,
                        "snippet": "# Workspaces"
                    },
                    {
                        "lineStart": 26925,
                        "lineEnd": 26961,
                        "snippet": "### TypeScript Support"
                    }
                ],
                "checksum": "checksum123"
            }]
        });

        assert_eq!(value, expected);
    }
}
