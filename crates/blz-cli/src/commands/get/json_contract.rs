#![allow(dead_code)]

//! Data structures representing the upcoming `blz get` JSON contract.
//!
//! These types mirror the shapes outlined in Linear issue BLZ-163 so that
//! subsequent work can focus on wiring the CLI logic without re-deriving the
//! schema. They are not yet used by the `get` command but will replace the
//! existing ad-hoc `serde_json::json!` builders in later slices.

use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

/// A contiguous range of snippet lines returned from a source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetRange {
    /// Inclusive line where the snippet starts (1-based).
    pub line_start: NonZeroUsize,
    /// Inclusive line where the snippet ends (1-based).
    pub line_end: NonZeroUsize,
    /// The textual content for the range.
    pub snippet: String,
}

impl SnippetRange {
    /// Constructs a snippet range after ensuring the bounds are ordered.
    pub fn try_new(
        line_start: NonZeroUsize,
        line_end: NonZeroUsize,
        snippet: impl Into<String>,
    ) -> Result<Self, ContractError> {
        ensure_start_before_end(line_start, line_end)?;

        Ok(Self {
            line_start,
            line_end,
            snippet: snippet.into(),
        })
    }
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
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub payload: Option<SnippetPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_applied: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

impl SnippetRequest {
    /// Creates a snippet request containing a single contiguous range.
    pub fn single(
        alias: impl Into<String>,
        source: impl Into<String>,
        snippet: impl Into<String>,
        line_start: NonZeroUsize,
        line_end: NonZeroUsize,
    ) -> Result<Self, ContractError> {
        ensure_start_before_end(line_start, line_end)?;

        Ok(Self {
            alias: alias.into(),
            source: source.into(),
            payload: Some(SnippetPayload::Single(SingleSnippet {
                snippet: snippet.into(),
                line_start,
                line_end,
            })),
            checksum: None,
            context_applied: None,
            truncated: None,
        })
    }

    /// Creates a snippet request backed by multiple ranges.
    pub fn with_ranges(
        alias: impl Into<String>,
        source: impl Into<String>,
        ranges: Vec<SnippetRange>,
    ) -> Result<Self, ContractError> {
        ensure_ordered_ranges(&ranges)?;

        Ok(Self {
            alias: alias.into(),
            source: source.into(),
            payload: Some(SnippetPayload::Multi(SnippetRanges { ranges })),
            checksum: None,
            context_applied: None,
            truncated: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SnippetPayload {
    Single(SingleSnippet),
    Multi(SnippetRanges),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleSnippet {
    pub snippet: String,
    pub line_start: NonZeroUsize,
    pub line_end: NonZeroUsize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetRanges {
    pub ranges: Vec<SnippetRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    LineOrder,
    OverlappingRanges,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LineOrder => f.write_str("line_start cannot be greater than line_end"),
            Self::OverlappingRanges => f.write_str("ranges must be non-overlapping and ordered"),
        }
    }
}

impl std::error::Error for ContractError {}

fn ensure_start_before_end(
    line_start: NonZeroUsize,
    line_end: NonZeroUsize,
) -> Result<(), ContractError> {
    if line_start > line_end {
        return Err(ContractError::LineOrder);
    }

    Ok(())
}

fn ensure_ordered_ranges(ranges: &[SnippetRange]) -> Result<(), ContractError> {
    let mut previous_end = None;
    for range in ranges {
        ensure_start_before_end(range.line_start, range.line_end)?;

        if let Some(end) = previous_end {
            if end > range.line_start {
                return Err(ContractError::OverlappingRanges);
            }
        }

        previous_end = Some(range.line_end);
    }

    Ok(())
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
    use std::num::NonZeroUsize;

    use super::{
        ExecutionMetadata, GetResponse, SingleSnippet, SnippetPayload, SnippetRange, SnippetRanges,
        SnippetRequest,
    };
    use serde_json::{Value, json};

    fn nzu(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("line numbers must be non-zero")
    }

    #[test]
    fn single_range_serializes_with_snippet_fields() {
        let response = GetResponse {
            requests: vec![SnippetRequest {
                alias: "bun".to_string(),
                source: "bun".to_string(),
                payload: Some(SnippetPayload::Single(SingleSnippet {
                    snippet: "# Workspaces".to_string(),
                    line_start: nzu(7_105),
                    line_end: nzu(7_164),
                })),
                checksum: Some("checksum123".to_string()),
                context_applied: Some(2),
                truncated: None,
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
                payload: Some(SnippetPayload::Multi(SnippetRanges {
                    ranges: vec![
                        SnippetRange::try_new(nzu(7_105), nzu(7_164), "# Workspaces")
                            .expect("range should be valid"),
                        SnippetRange::try_new(nzu(26_925), nzu(26_961), "### TypeScript Support")
                            .expect("range should be valid"),
                    ],
                })),
                checksum: Some("checksum123".to_string()),
                context_applied: None,
                truncated: None,
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

    #[test]
    fn roundtrip_serialization_preserves_data() {
        let response = GetResponse {
            requests: vec![
                SnippetRequest {
                    alias: "bun".into(),
                    source: "bun".into(),
                    payload: Some(SnippetPayload::Single(SingleSnippet {
                        snippet: "# Workspaces".into(),
                        line_start: nzu(7_105),
                        line_end: nzu(7_164),
                    })),
                    checksum: Some("checksum123".into()),
                    context_applied: Some(2),
                    truncated: None,
                },
                SnippetRequest {
                    alias: "bun".into(),
                    source: "bun".into(),
                    payload: Some(SnippetPayload::Multi(SnippetRanges {
                        ranges: vec![
                            SnippetRange::try_new(
                                nzu(26_925),
                                nzu(26_961),
                                "### TypeScript Support",
                            )
                            .expect("range should be valid"),
                        ],
                    })),
                    checksum: None,
                    context_applied: None,
                    truncated: None,
                },
            ],
            metadata: ExecutionMetadata {
                execution_time_ms: Some(12),
                total_sources: Some(2),
            },
        };

        let json = serde_json::to_string(&response).expect("serialization should succeed");
        let deserialized: GetResponse =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized, response);
    }
}
