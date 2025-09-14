//! Core data structures for blz cache system.
//!
//! This module defines the fundamental types used throughout blz-core for representing
//! documentation sources, search results, table of contents, and other cache metadata.
//!
//! ## Type Categories
//!
//! - **Source Management**: [`Source`], [`LlmsJson`], [`FileInfo`]
//! - **Content Structure**: [`TocEntry`], [`HeadingBlock`], [`LineIndex`]
//! - **Search Results**: [`SearchHit`]  
//! - **Change Tracking**: [`DiffEntry`], [`ChangedSection`]
//! - **Diagnostics**: [`Diagnostic`], [`DiagnosticSeverity`]
//!
//! ## Serialization
//!
//! Most types implement `Serialize` and `Deserialize` for JSON/TOML persistence.
//! The serialization format is designed to be stable across versions and readable
//! by external tools.
//!
//! ## Examples
//!
//! ### Creating a table of contents entry:
//!
//! ```rust
//! use blz_core::TocEntry;
//!
//! let toc_entry = TocEntry {
//!     heading_path: vec!["Getting Started".to_string(), "Installation".to_string()],
//!     lines: "15-42".to_string(),
//!     anchor: None,
//!     children: vec![],
//! };
//!
//! println!("Section: {} (lines {})",
//!     toc_entry.heading_path.join(" > "),
//!     toc_entry.lines);
//! ```
//!
//! ### Working with search results:
//!
//! ```rust
//! use blz_core::SearchHit;
//!
//! let hit = SearchHit {
//!     alias: "react".to_string(),
//!     source: "react".to_string(),
//!     file: "hooks.md".to_string(),
//!     heading_path: vec!["Hooks".to_string(), "useState".to_string()],
//!     lines: "120-145".to_string(),
//!     snippet: "useState returns an array with two elements...".to_string(),
//!     score: 0.92,
//!     source_url: Some("https://react.dev/hooks".to_string()),
//!     checksum: "abc123".to_string(),
//!     anchor: Some("react-hooks-usestate".to_string()),
//! };
//!
//! println!("Found: {} in {} (score: {:.2})",
//!     hit.heading_path.join(" > "),
//!     hit.alias,
//!     hit.score);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about a documentation source.
///
/// Represents metadata about a fetched llms.txt source, including caching headers
/// and content verification information. This is used to implement efficient
/// conditional fetching and cache validation.
///
/// ## Caching Strategy
///
/// The `etag` and `last_modified` fields are used for HTTP conditional requests:
/// - If `etag` is present, uses `If-None-Match` header
/// - If `last_modified` is present, uses `If-Modified-Since` header
/// - Content is only re-fetched if the server indicates changes
///
/// ## Content Integrity
///
/// The `sha256` field provides content verification and change detection:
/// - Calculated from the raw fetched content
/// - Used to detect changes even when HTTP headers are unreliable
/// - Enables diff generation between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// The URL from which this content was fetched.
    pub url: String,

    /// HTTP `ETag` header from the server response.
    ///
    /// Used for efficient conditional requests. When present, subsequent
    /// requests include `If-None-Match` header to avoid re-downloading
    /// unchanged content.
    pub etag: Option<String>,

    /// HTTP Last-Modified header from the server response.
    ///
    /// Used as fallback for conditional requests when `ETag` is not available.
    /// Formatted as HTTP date string (RFC 2822 format).
    pub last_modified: Option<String>,

    /// Timestamp when this content was last fetched.
    ///
    /// Used to determine when content should be refreshed based on
    /// configured refresh intervals.
    pub fetched_at: DateTime<Utc>,

    /// SHA-256 hash of the content.
    ///
    /// Provides content integrity verification and change detection.
    /// Calculated from the raw content bytes, not the parsed structure.
    pub sha256: String,
}

/// An entry in the table of contents.
///
/// Represents a section in the documentation with its hierarchical position,
/// line range, and any subsections. The structure mirrors the heading hierarchy
/// in the source markdown.
///
/// ## Line Range Format
///
/// The `lines` field uses the format `"start-end"` where both numbers are
/// 1-based line numbers in the source document (e.g., `"15-42"`).
///
/// ## Hierarchical Structure
///
/// TOC entries can be nested to represent the document structure:
/// - Top-level entries have no parent
/// - Child entries are stored in the `children` vector
/// - The `heading_path` includes all parent headings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Full hierarchical path to this heading.
    ///
    /// Contains all parent heading titles leading to this entry.
    /// For example: `["Getting Started", "Installation", "Prerequisites"]`
    pub heading_path: Vec<String>,

    /// Line range where this section appears.
    ///
    /// Format: `"start-end"` where both are 1-based line numbers.
    /// Examples: `"15-42"`, `"1-10"`, `"100-100"` (single line)
    pub lines: String,

    /// Stable content anchor for this section.
    ///
    /// Computed from heading text and leading content to remap sections
    /// across updates when text moves. Base64(SHA-256) truncated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,

    /// Nested subsections under this heading.
    ///
    /// Each child entry represents a subsection with its own potential
    /// children, forming a tree structure that matches the document hierarchy.
    pub children: Vec<TocEntry>,
}

/// Information about a file in the cache.
///
/// Tracks individual files that are part of a documentation source,
/// including their content hashes for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Relative path to the file within the source.
    ///
    /// Typically this is just "llms.txt" for simple sources, but may
    /// include subdirectories for sources with multiple files.
    pub path: String,

    /// SHA-256 hash of the file content.
    ///
    /// Used for integrity verification and change detection.
    /// Calculated from the raw file bytes.
    pub sha256: String,
}

/// Information about line indexing in the source.
///
/// Provides metadata about how lines are indexed and whether byte offsets
/// are tracked for efficient content access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineIndex {
    /// Total number of lines in the source document.
    ///
    /// Used for validation and progress reporting during parsing operations.
    pub total_lines: usize,

    /// Whether byte offsets are tracked for each line.
    ///
    /// When `true`, the system maintains byte offset information that
    /// enables faster random access to specific lines. When `false`,
    /// line access requires sequential reading from the beginning.
    pub byte_offsets: bool,
}

/// A diagnostic message from parsing or processing operations.
///
/// Represents warnings, errors, or informational messages generated during
/// content processing. Diagnostics help identify issues with source content
/// or processing configuration.
///
/// ## Severity Levels
///
/// - **Error**: Critical issues that prevent processing
/// - **Warn**: Issues that may affect quality but allow processing to continue
/// - **Info**: Informational messages about processing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level of this diagnostic.
    pub severity: DiagnosticSeverity,

    /// Human-readable description of the issue.
    ///
    /// Should be clear and actionable when possible, explaining what
    /// went wrong and potentially how to fix it.
    pub message: String,

    /// Line number where the issue occurred (1-based).
    ///
    /// `None` if the diagnostic applies to the entire document or
    /// a specific line cannot be determined.
    pub line: Option<usize>,
}

/// Severity level for diagnostic messages.
///
/// Determines how diagnostic messages should be handled and displayed.
/// The levels follow standard logging conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// Critical error that prevents successful processing.
    ///
    /// Processing should be aborted when error diagnostics are present.
    Error,

    /// Warning about potential issues.
    ///
    /// Processing can continue but the results may be affected.
    /// Users should review warnings to ensure content quality.
    Warn,

    /// Informational message about processing decisions.
    ///
    /// Useful for debugging and understanding how content was processed.
    Info,
}

/// Complete metadata for a cached documentation source.
///
/// This is the main structure stored as JSON metadata for each cached source.
/// It contains all information needed to understand the cached content structure,
/// validate integrity, and provide search capabilities.
///
/// ## Storage Location
///
/// Stored as `<cache_root>/<alias>/llms.json` for each source.
///
/// ## Version Compatibility
///
/// The JSON format is designed to be forward-compatible. New fields can be
/// added without breaking existing readers, and missing fields are handled
/// gracefully with sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsJson {
    /// Unique identifier for this source.
    ///
    /// Used as the directory name and in search results. Should be
    /// URL-safe and filesystem-safe.
    pub alias: String,

    /// Source metadata and caching information.
    pub source: Source,

    /// Table of contents extracted from the document.
    ///
    /// Provides hierarchical navigation and enables section-specific search.
    pub toc: Vec<TocEntry>,

    /// Information about files in this source.
    ///
    /// Typically contains a single entry for "llms.txt", but may include
    /// additional files for complex sources.
    pub files: Vec<FileInfo>,

    /// Line indexing information for the source.
    pub line_index: LineIndex,

    /// Diagnostic messages from processing this source.
    ///
    /// Includes warnings about malformed content, missing sections,
    /// or processing issues that users should be aware of.
    pub diagnostics: Vec<Diagnostic>,

    /// Parser/segmentation metadata for durability across updates.
    ///
    /// Optional for forward/backward compatibility. When present, indicates
    /// how the document was segmented and which parser version produced it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_meta: Option<ParseMeta>,
}

/// Metadata about how parsing/segmentation was performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseMeta {
    /// Monotonic parser version used to generate this JSON.
    pub parser_version: u32,
    /// Segmentation strategy used (e.g., "structured", "windowed").
    pub segmentation: String,
}

/// A search result hit.
///
/// Represents a single match from a search query, including location information,
/// relevance scoring, and content snippet for display.
///
/// ## Relevance Scoring
///
/// The `score` field uses BM25 ranking with scores typically in the range 0.0-10.0.
/// Higher scores indicate better relevance to the search query.
///
/// ## Line Range Format
///
/// The `lines` field uses the same format as [`TocEntry`]: `"start-end"` with
/// 1-based line numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// Source alias where this hit was found.
    ///
    /// Corresponds to the directory name in the cache and the `alias`
    /// field in [`LlmsJson`].
    pub alias: String,

    /// Canonical source name (user-facing "source").
    ///
    /// For now, this is the same as `alias`. Kept separate to allow future
    /// rename/alias semantics without breaking JSON consumers.
    pub source: String,

    /// Filename within the source where the hit was found.
    ///
    /// Typically "llms.txt" but may be other files for multi-file sources.
    pub file: String,

    /// Hierarchical path to the section containing this hit.
    ///
    /// Shows the full context of nested headings leading to this result.
    /// Empty vector indicates content not under any specific heading.
    pub heading_path: Vec<String>,

    /// Line range containing the matching content.
    ///
    /// Format: `"start-end"` with 1-based line numbers.
    /// May represent a single line (`"42-42"`) or a range (`"42-45"`).
    pub lines: String,

    /// Content snippet showing the match context.
    ///
    /// Contains the relevant portion of the content with the search terms
    /// highlighted or emphasized. Length is limited for display purposes.
    pub snippet: String,

    /// Relevance score for this hit.
    ///
    /// Higher scores indicate better relevance. Typically uses BM25 scoring
    /// with values in the range 0.0-10.0, though scores can exceed this range
    /// for highly relevant matches.
    pub score: f32,

    /// Original URL of the source document.
    ///
    /// Provides a link back to the original documentation for reference.
    /// May be `None` for local or generated content.
    pub source_url: Option<String>,

    /// Content checksum for verification.
    ///
    /// Used to verify that the search result corresponds to the expected
    /// version of the content. Helps detect stale results after content updates.
    pub checksum: String,

    /// Stable anchor for the section (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

/// Mapping between stable content anchors and line ranges across updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMapping {
    /// Stable anchor value computed from heading and leading content
    pub anchor: String,
    /// Previous line range (e.g., "15-42")
    pub old_lines: String,
    /// New line range after update
    pub new_lines: String,
    /// Heading path for context
    pub heading_path: Vec<String>,
}

/// Anchors remapping file saved per alias to help remap citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorsMap {
    /// Timestamp when this map was generated
    pub updated_at: DateTime<Utc>,
    /// Mappings from anchors to new line ranges
    pub mappings: Vec<AnchorMapping>,
}

/// An entry recording changes between content versions.
///
/// Tracks what changed when a documentation source was updated, including
/// metadata about the change and references to detailed diff information.
///
/// ## Diff Storage
///
/// The actual content differences are stored in a separate unified diff file
/// referenced by `unified_diff_path`. This keeps the metadata lightweight
/// while preserving detailed change information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Timestamp when this change was detected.
    pub ts: DateTime<Utc>,

    /// Source alias that was changed.
    pub alias: String,

    /// `ETag` before the change.
    ///
    /// `None` if this is the initial version or `ETag` was not available.
    pub etag_before: Option<String>,

    /// `ETag` after the change.
    ///
    /// `None` if `ETag` is not available from the server.
    pub etag_after: Option<String>,

    /// Content SHA-256 hash before the change.
    pub sha_before: String,

    /// Content SHA-256 hash after the change.
    pub sha_after: String,

    /// Path to the unified diff file.
    ///
    /// Relative path within the cache directory structure.
    /// The diff file contains detailed line-by-line changes in standard
    /// unified diff format.
    pub unified_diff_path: String,

    /// Sections that were modified in this change.
    ///
    /// Provides a high-level summary of which document sections were
    /// affected without requiring parsing of the full diff.
    pub changed_sections: Vec<ChangedSection>,

    /// Optional human-readable summary of changes.
    ///
    /// May be generated automatically or provided manually to describe
    /// the nature of the changes in user-friendly terms.
    pub summary: Option<String>,
}

/// Information about a section that was modified in a content update.
///
/// Part of [`DiffEntry`] to provide section-level change tracking without
/// requiring detailed diff parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSection {
    /// Hierarchical path to the modified section.
    ///
    /// Matches the format used in [`TocEntry::heading_path`] and
    /// [`SearchHit::heading_path`].
    pub heading_path: Vec<String>,

    /// Line range that was modified.
    ///
    /// Format: `"start-end"` with 1-based line numbers, consistent
    /// with other line range fields in this module.
    pub lines: String,
}

/// A contiguous block of content under a specific heading.
///
/// Used during parsing to represent sections of the document that belong
/// together under a heading hierarchy. This is an intermediate representation
/// that gets converted to search index entries and TOC entries.
///
/// ## Content Processing
///
/// The `content` field contains the raw text of the section, which may include:
/// - The heading itself
/// - All text under the heading until the next same-level or higher-level heading
/// - Code blocks, lists, and other markdown elements
///
/// ## Line Numbers
///
/// Both `start_line` and `end_line` are 1-based and inclusive, matching the
/// format used throughout the rest of the system.
#[derive(Debug, Clone)]
pub struct HeadingBlock {
    /// Hierarchical path to this heading.
    ///
    /// Contains all parent heading titles, consistent with other path
    /// representations in this module.
    pub path: Vec<String>,

    /// Raw content text for this block.
    ///
    /// Includes the heading itself and all content until the next
    /// same-level or higher-level heading.
    pub content: String,

    /// Starting line number (1-based, inclusive).
    pub start_line: usize,

    /// Ending line number (1-based, inclusive).
    pub end_line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_search_hit_equality() {
        // Test that SearchHit can be compared for deduplication
        let hit1 = SearchHit {
            alias: "react".to_string(),
            source: "react".to_string(),
            file: "hooks.md".to_string(),
            heading_path: vec!["React".to_string(), "Hooks".to_string()],
            lines: "100-120".to_string(),
            snippet: "useState is a React hook...".to_string(),
            score: 0.95,
            source_url: Some("https://react.dev".to_string()),
            checksum: "abc123".to_string(),
            anchor: Some("anchor1".to_string()),
        };

        let hit2 = SearchHit {
            alias: "react".to_string(),
            source: "react".to_string(),
            file: "hooks.md".to_string(),
            heading_path: vec!["React".to_string(), "Hooks".to_string()],
            lines: "100-120".to_string(),
            snippet: "useState is a React hook...".to_string(),
            score: 0.90, // Different score
            source_url: Some("https://react.dev".to_string()),
            checksum: "abc123".to_string(),
            anchor: Some("anchor1".to_string()),
        };

        // Should be considered the same for deduplication (same alias, lines, heading_path)
        assert_eq!(hit1.alias, hit2.alias);
        assert_eq!(hit1.lines, hit2.lines);
        assert_eq!(hit1.heading_path, hit2.heading_path);
    }

    #[test]
    fn test_source_creation() {
        let now = Utc::now();
        let source = Source {
            url: "https://example.com/llms.txt".to_string(),
            etag: Some("abc123".to_string()),
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            fetched_at: now,
            sha256: "deadbeef".to_string(),
        };

        assert_eq!(source.url, "https://example.com/llms.txt");
        assert_eq!(source.etag, Some("abc123".to_string()));
        assert_eq!(source.sha256, "deadbeef");
    }

    #[test]
    fn test_toc_entry_creation() {
        let entry = TocEntry {
            heading_path: vec!["Getting Started".to_string(), "Installation".to_string()],
            lines: "1-25".to_string(),
            anchor: None,
            children: vec![],
        };

        assert_eq!(entry.heading_path.len(), 2);
        assert_eq!(entry.lines, "1-25");
        assert!(entry.children.is_empty());
    }

    #[test]
    fn test_line_index_creation() {
        let index = LineIndex {
            total_lines: 1000,
            byte_offsets: true,
        };

        assert_eq!(index.total_lines, 1000);
        assert!(index.byte_offsets);
    }

    #[test]
    fn test_diagnostic_severity_serialization() {
        let diagnostic = Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Missing heading".to_string(),
            line: Some(42),
        };

        // Test serialization/deserialization
        let json = serde_json::to_string(&diagnostic).expect("Should serialize");
        let deserialized: Diagnostic = serde_json::from_str(&json).expect("Should deserialize");

        match deserialized.severity {
            DiagnosticSeverity::Error => {},
            _ => unreachable!("Expected Error severity"),
        }
        assert_eq!(deserialized.message, "Missing heading");
        assert_eq!(deserialized.line, Some(42));
    }

    #[test]
    fn test_llms_json_structure() {
        let llms_json = LlmsJson {
            alias: "test".to_string(),
            source: Source {
                url: "https://example.com".to_string(),
                etag: None,
                last_modified: None,
                fetched_at: Utc::now(),
                sha256: "hash".to_string(),
            },
            toc: vec![],
            files: vec![FileInfo {
                path: "llms.txt".to_string(),
                sha256: "hash".to_string(),
            }],
            line_index: LineIndex {
                total_lines: 100,
                byte_offsets: false,
            },
            diagnostics: vec![],
            parse_meta: None,
        };

        assert_eq!(llms_json.alias, "test");
        assert_eq!(llms_json.files.len(), 1);
        assert_eq!(llms_json.line_index.total_lines, 100);
    }

    #[test]
    fn test_heading_block_creation() {
        let block = HeadingBlock {
            path: vec!["API".to_string(), "Reference".to_string()],
            content: "This is the API reference content...".to_string(),
            start_line: 50,
            end_line: 75,
        };

        assert_eq!(block.path.len(), 2);
        assert_eq!(block.start_line, 50);
        assert_eq!(block.end_line, 75);
        assert!(block.content.starts_with("This is the API"));
    }
}
