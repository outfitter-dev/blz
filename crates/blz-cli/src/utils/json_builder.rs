//! Helper utilities for building blz-core JSON structures

use blz_core::{FileInfo, LineIndex, LlmsJson, ParseMeta, ParseResult, Source};
use chrono::Utc;

/// Build a `LlmsJson` structure from parse results and metadata
///
/// This helper constructs the complete JSON structure that gets saved
/// to disk for each source, containing TOC, line index, and metadata.
pub fn build_llms_json(
    alias: &str,
    url: &str,
    file_name: &str,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    parse_result: &ParseResult,
) -> LlmsJson {
    LlmsJson {
        source: alias.to_string(),
        metadata: Source {
            url: url.to_string(),
            etag,
            last_modified,
            fetched_at: Utc::now(),
            sha256: sha256.clone(),
            variant: blz_core::SourceVariant::Llms, // Default, will be updated by caller
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
        filter_stats: None,
        toc: parse_result.toc.clone(),
        files: vec![FileInfo {
            path: file_name.to_string(),
            sha256,
        }],
        line_index: LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics.clone(),
        parse_meta: Some(ParseMeta {
            parser_version: 1,
            segmentation: "structured".to_string(),
        }),
    }
}
