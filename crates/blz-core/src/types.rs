use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub heading_path: Vec<String>,
    pub lines: String, // "120-168"
    pub children: Vec<TocEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineIndex {
    pub total_lines: usize,
    pub byte_offsets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsJson {
    pub alias: String,
    pub source: Source,
    pub toc: Vec<TocEntry>,
    pub files: Vec<FileInfo>,
    pub line_index: LineIndex,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub alias: String,
    pub file: String,
    pub heading_path: Vec<String>,
    pub lines: String,
    pub snippet: String,
    pub score: f32,
    pub source_url: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub ts: DateTime<Utc>,
    pub alias: String,
    pub etag_before: Option<String>,
    pub etag_after: Option<String>,
    pub sha_before: String,
    pub sha_after: String,
    pub unified_diff_path: String,
    pub changed_sections: Vec<ChangedSection>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSection {
    pub heading_path: Vec<String>,
    pub lines: String,
}

#[derive(Debug, Clone)]
pub struct HeadingBlock {
    pub path: Vec<String>,
    pub content: String,
    pub start_line: usize,
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
            file: "hooks.md".to_string(),
            heading_path: vec!["React".to_string(), "Hooks".to_string()],
            lines: "100-120".to_string(),
            snippet: "useState is a React hook...".to_string(),
            score: 0.95,
            source_url: Some("https://react.dev".to_string()),
            checksum: "abc123".to_string(),
        };

        let hit2 = SearchHit {
            alias: "react".to_string(),
            file: "hooks.md".to_string(),
            heading_path: vec!["React".to_string(), "Hooks".to_string()],
            lines: "100-120".to_string(),
            snippet: "useState is a React hook...".to_string(),
            score: 0.90, // Different score
            source_url: Some("https://react.dev".to_string()),
            checksum: "abc123".to_string(),
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
            _ => panic!("Expected Error severity"),
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
