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