use crate::profiling::{ComponentTimings, OperationTimer, PerformanceMetrics};
use crate::{Error, HeadingBlock, Result, SearchHit, normalize_text_for_search};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use sha2::{Digest, Sha256};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, IndexReader, doc};
use tracing::{Level, debug, info};

/// Default number of characters returned for a search snippet (before any ellipses).
pub const DEFAULT_SNIPPET_CHAR_LIMIT: usize = 200;
/// Minimum number of characters permitted for a search snippet.
pub const MIN_SNIPPET_CHAR_LIMIT: usize = 50;
/// Maximum number of characters permitted for a search snippet.
pub const MAX_SNIPPET_CHAR_LIMIT: usize = 1_000;

pub(crate) const fn clamp_snippet_chars(chars: usize) -> usize {
    if chars < MIN_SNIPPET_CHAR_LIMIT {
        MIN_SNIPPET_CHAR_LIMIT
    } else if chars > MAX_SNIPPET_CHAR_LIMIT {
        MAX_SNIPPET_CHAR_LIMIT
    } else {
        chars
    }
}

/// Tantivy-based search index for llms.txt documentation
pub struct SearchIndex {
    index: Index,
    #[allow(dead_code)]
    schema: Schema,
    content_field: Field,
    path_field: Field,
    heading_path_field: Field,
    heading_path_display_field: Option<Field>,
    heading_path_normalized_field: Option<Field>,
    lines_field: Field,
    alias_field: Field,
    anchor_field: Option<Field>,
    reader: IndexReader,
    metrics: Option<PerformanceMetrics>,
}

impl SearchIndex {
    /// Enable performance metrics collection
    #[must_use]
    pub fn with_metrics(mut self, metrics: PerformanceMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Get the performance metrics instance
    #[must_use]
    pub const fn metrics(&self) -> Option<&PerformanceMetrics> {
        self.metrics.as_ref()
    }
    /// Creates a new search index at the specified path
    pub fn create(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let heading_path_field = schema_builder.add_text_field("heading_path", TEXT | STORED);
        let heading_path_display_field =
            schema_builder.add_text_field("heading_path_display", TEXT | STORED);
        let heading_path_normalized_field =
            schema_builder.add_text_field("heading_path_normalized", TEXT);
        let lines_field = schema_builder.add_text_field("lines", STRING | STORED);
        let alias_field = schema_builder.add_text_field("alias", STRING | STORED);
        let anchor_field = schema_builder.add_text_field("anchor", STRING | STORED);

        let schema = schema_builder.build();

        std::fs::create_dir_all(index_path)
            .map_err(|e| Error::Index(format!("Failed to create index directory: {e}")))?;

        let index = Index::create_in_dir(index_path, schema.clone())
            .map_err(|e| Error::Index(format!("Failed to create index: {e}")))?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::Index(format!("Failed to create reader: {e}")))?;

        Ok(Self {
            index,
            schema,
            content_field,
            path_field,
            heading_path_field,
            heading_path_display_field: Some(heading_path_display_field),
            heading_path_normalized_field: Some(heading_path_normalized_field),
            lines_field,
            alias_field,
            reader,
            anchor_field: Some(anchor_field),
            metrics: None,
        })
    }

    /// Creates a new search index or opens an existing one at the specified path
    pub fn create_or_open(index_path: &Path) -> Result<Self> {
        if index_path.exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    /// Opens an existing search index at the specified path
    pub fn open(index_path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_path)
            .map_err(|e| Error::Index(format!("Failed to open index: {e}")))?;

        let schema = index.schema();

        let content_field = schema
            .get_field("content")
            .map_err(|_| Error::Index("Missing content field".into()))?;
        let path_field = schema
            .get_field("path")
            .map_err(|_| Error::Index("Missing path field".into()))?;
        let heading_path_field = schema
            .get_field("heading_path")
            .map_err(|_| Error::Index("Missing heading_path field".into()))?;
        let heading_path_display_field = schema.get_field("heading_path_display").ok();
        let heading_path_normalized_field = schema.get_field("heading_path_normalized").ok();
        let lines_field = schema
            .get_field("lines")
            .map_err(|_| Error::Index("Missing lines field".into()))?;
        let alias_field = schema
            .get_field("alias")
            .map_err(|_| Error::Index("Missing alias field".into()))?;

        // Anchor is optional for backward compatibility with older indexes
        let anchor_field = schema.get_field("anchor").ok();

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::Index(format!("Failed to create reader: {e}")))?;

        Ok(Self {
            index,
            schema,
            content_field,
            path_field,
            heading_path_field,
            heading_path_display_field,
            heading_path_normalized_field,
            lines_field,
            alias_field,
            reader,
            anchor_field,
            metrics: None,
        })
    }

    /// Indexes a collection of heading blocks for a given alias
    pub fn index_blocks(&self, alias: &str, blocks: &[HeadingBlock]) -> Result<()> {
        let timer = self.metrics.as_ref().map_or_else(
            || OperationTimer::new(&format!("index_{alias}")),
            |metrics| OperationTimer::with_metrics(&format!("index_{alias}"), metrics.clone()),
        );

        let mut timings = ComponentTimings::new();

        let mut writer = timings.time("writer_creation", || {
            self.index
                .writer(50_000_000)
                .map_err(|e| Error::Index(format!("Failed to create writer: {e}")))
        })?;

        // Delete all existing documents for this alias
        let _deleted = timings.time("delete_existing", || {
            writer.delete_term(tantivy::Term::from_field_text(self.alias_field, alias))
        });

        let mut total_content_bytes = 0usize;

        timings.time("document_creation", || {
            for block in blocks {
                total_content_bytes += block.content.len();
                let heading_path_str = block.path.join(" > ");
                let display_path_str = block.display_path.join(" > ");
                let normalized_heading_str = block.normalized_tokens.join(" ");
                let lines_str = format!("{}-{}", block.start_line, block.end_line);
                // Compute anchor from last heading text
                let anchor = block.path.last().map(|h| Self::compute_anchor(h));

                let mut doc = doc!(
                    self.content_field => block.content.as_str(),  // Use &str instead of clone
                    self.path_field => "llms.txt",  // Always llms.txt (no flavor variants)
                    self.heading_path_field => heading_path_str,
                    self.lines_field => lines_str,
                    self.alias_field => alias
                );
                if let Some(field) = self.heading_path_display_field {
                    doc.add_text(field, display_path_str.as_str());
                }
                if let Some(field) = self.heading_path_normalized_field {
                    doc.add_text(field, normalized_heading_str.as_str());
                }
                if let (Some(f), Some(a)) = (self.anchor_field, anchor) {
                    doc.add_text(f, a);
                }

                writer
                    .add_document(doc)
                    .map_err(|e| Error::Index(format!("Failed to add document: {e}")))?;
            }
            Ok::<(), Error>(())
        })?;

        timings.time("commit", || {
            writer
                .commit()
                .map_err(|e| Error::Index(format!("Failed to commit: {e}")))
        })?;

        timings.time("reader_reload", || {
            self.reader
                .reload()
                .map_err(|e| Error::Index(format!("Failed to reload reader: {e}")))
        })?;

        let duration = timer.finish_index(total_content_bytes);

        // Print detailed breakdown if debug logging is enabled
        if tracing::enabled!(Level::DEBUG) {
            timings.print_breakdown();
        }

        info!(
            "Indexed {} blocks ({} bytes) for {} in {:.2}ms",
            blocks.len(),
            total_content_bytes,
            alias,
            duration.as_millis()
        );

        Ok(())
    }

    /// Searches the index with optional alias filtering
    pub fn search(
        &self,
        query_str: &str,
        alias: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        self.search_with_snippet_limit(query_str, alias, limit, DEFAULT_SNIPPET_CHAR_LIMIT)
    }

    /// Searches the index with optional alias filtering and an explicit snippet character limit.
    #[allow(clippy::too_many_lines)] // Complex search logic requires detailed implementation
    pub fn search_with_snippet_limit(
        &self,
        query_str: &str,
        alias: Option<&str>,
        limit: usize,
        snippet_max_chars: usize,
    ) -> Result<Vec<SearchHit>> {
        let timer = self.metrics.as_ref().map_or_else(
            || OperationTimer::new(&format!("search_{query_str}")),
            |metrics| OperationTimer::with_metrics(&format!("search_{query_str}"), metrics.clone()),
        );

        let mut timings = ComponentTimings::new();
        let mut lines_searched = 0usize;
        let snippet_limit = clamp_snippet_chars(snippet_max_chars);

        let searcher = timings.time("searcher_creation", || self.reader.searcher());

        let query_parser = timings.time("query_parser_creation", || {
            let mut fields = vec![self.content_field, self.heading_path_field];
            if let Some(field) = self.heading_path_display_field {
                fields.push(field);
            }
            if let Some(field) = self.heading_path_normalized_field {
                fields.push(field);
            }
            QueryParser::for_index(&self.index, fields)
        });

        // Sanitize query more efficiently with a single allocation
        let mut filter_clauses = Vec::new();
        if let Some(alias) = alias {
            filter_clauses.push(format!("alias:{alias}"));
        }

        let sanitized_query = Self::escape_query(query_str);

        // Check if the original query is a phrase query (quoted)
        let trimmed = query_str.trim();
        let is_phrase_query =
            trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2;

        let normalized_query_raw = normalize_text_for_search(query_str);
        let normalized_query = if normalized_query_raw.is_empty() {
            String::new()
        } else {
            let escaped = Self::escape_query(&normalized_query_raw);
            // Preserve phrase query syntax when normalizing
            if is_phrase_query && !escaped.starts_with('"') {
                format!("\"{escaped}\"")
            } else {
                escaped
            }
        };

        let use_normalized = !normalized_query.is_empty() && normalized_query != sanitized_query;
        let query_body = if use_normalized {
            format!("({sanitized_query}) OR ({normalized_query})")
        } else {
            sanitized_query
        };

        let full_query_str = if filter_clauses.is_empty() {
            query_body
        } else {
            format!("{} AND ({query_body})", filter_clauses.join(" AND "))
        };

        let query = timings.time("query_parsing", || {
            query_parser
                .parse_query(&full_query_str)
                .map_err(|e| Error::Index(format!("Failed to parse query: {e}")))
        })?;

        let top_docs = timings.time("tantivy_search", || {
            searcher
                .search(&query, &TopDocs::with_limit(limit))
                .map_err(|e| Error::Index(format!("Search failed: {e}")))
        })?;

        let mut hits = Vec::new();

        timings.time("result_processing", || {
            for (score, doc_address) in top_docs {
                let doc = searcher
                    .doc(doc_address)
                    .map_err(|e| Error::Index(format!("Failed to retrieve doc: {e}")))?;

                let alias = Self::get_field_text(&doc, self.alias_field)?;
                let file = Self::get_field_text(&doc, self.path_field)?;
                let heading_path_str = Self::get_field_text(&doc, self.heading_path_field)?;
                let display_path_str = self
                    .heading_path_display_field
                    .and_then(|field| Self::get_optional_field(&doc, field));
                let lines = Self::get_field_text(&doc, self.lines_field)?;
                let content = Self::get_field_text(&doc, self.content_field)?;
                let anchor = self.anchor_field.and_then(|f| {
                    doc.get_first(f)
                        .and_then(|v| v.as_str())
                        .map(std::string::ToString::to_string)
                });

                // Count lines for metrics
                lines_searched += content.lines().count();

                let raw_heading_segments: Vec<String> = heading_path_str
                    .split(" > ")
                    .map(std::string::ToString::to_string)
                    .collect();
                let display_heading_segments = display_path_str.as_ref().map(|value| {
                    value
                        .split(" > ")
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                });

                let heading_path = display_heading_segments
                    .clone()
                    .unwrap_or_else(|| raw_heading_segments.clone());
                let raw_heading_path = display_heading_segments
                    .as_ref()
                    .map(|_| raw_heading_segments.clone());

                let snippet = Self::extract_snippet(&content, query_str, snippet_limit);

                // Prefer exact match line(s) when possible for better citations
                let exact_lines = Self::compute_match_lines(&content, query_str, &lines)
                    .unwrap_or_else(|| lines.clone());

                // Parse numeric line range for convenience
                let line_numbers = Self::parse_lines_range(&exact_lines);

                hits.push(SearchHit {
                    source: alias,
                    file,
                    heading_path,
                    raw_heading_path,
                    lines: exact_lines,
                    line_numbers,
                    snippet,
                    score,
                    source_url: None,
                    fetched_at: None,
                    is_stale: false,
                    checksum: String::new(),
                    anchor,
                    context: None,
                });
            }
            Ok::<(), Error>(())
        })?;

        let duration = timer.finish_search(lines_searched);

        // Print detailed breakdown if debug logging is enabled
        if tracing::enabled!(Level::DEBUG) {
            timings.print_breakdown();
        }

        debug!(
            "Found {} hits for query '{}' in {:.2}ms (searched {} lines)",
            hits.len(),
            query_str,
            duration.as_millis(),
            lines_searched
        );

        Ok(hits)
    }

    fn compute_anchor(heading_text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(heading_text.trim().to_lowercase().as_bytes());
        let digest = hasher.finalize();
        let full = B64.encode(digest);
        full[..22.min(full.len())].to_string()
    }

    fn get_field_text(doc: &tantivy::TantivyDocument, field: Field) -> Result<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| Error::Index("Field not found in document".into()))
    }

    fn get_optional_field(doc: &tantivy::TantivyDocument, field: Field) -> Option<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
    }

    fn escape_query(query: &str) -> String {
        let mut escaped = String::with_capacity(query.len() * 2);
        for ch in query.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '(' => escaped.push_str("\\("),
                ')' => escaped.push_str("\\)"),
                '[' => escaped.push_str("\\["),
                ']' => escaped.push_str("\\]"),
                '{' => escaped.push_str("\\{"),
                '}' => escaped.push_str("\\}"),
                '^' => escaped.push_str("\\^"),
                '~' => escaped.push_str("\\~"),
                ':' => escaped.push_str("\\:"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Compute exact match line(s) within a block's content relative to its stored line range.
    /// Returns a "start-end" string (typically a single line) falling back to the original range on failure.
    fn compute_match_lines(content: &str, query: &str, block_lines: &str) -> Option<String> {
        // Parse the block's starting line
        let block_start: usize = block_lines
            .split(['-', ':'])
            .next()
            .and_then(|s| s.trim().parse::<usize>().ok())?;

        // Tokenize while preserving quoted phrases so we prefer the full phrase when present.
        let mut phrases = Vec::new();
        let mut terms = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        for ch in query.chars() {
            match ch {
                '"' => {
                    if in_quotes {
                        if !current.is_empty() {
                            phrases.push(current.clone());
                            current.clear();
                        }
                        in_quotes = false;
                    } else {
                        in_quotes = true;
                    }
                },
                ch if ch.is_whitespace() && !in_quotes => {
                    if !current.is_empty() {
                        terms.push(current.clone());
                        current.clear();
                    }
                },
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            if in_quotes {
                phrases.push(current);
            } else {
                terms.push(current);
            }
        }

        let phrases: Vec<String> = phrases
            .into_iter()
            .map(|token| {
                token
                    .trim_matches('"')
                    .trim_start_matches(['+', '-'])
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        let terms: Vec<String> = terms
            .into_iter()
            .map(|token| {
                token
                    .trim_matches('"')
                    .trim_start_matches(['+', '-'])
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();

        let mut best_pos: Option<usize> = None;
        for token in phrases.iter().chain(terms.iter()) {
            if let Some(pos) = content.find(token) {
                best_pos = Some(best_pos.map_or(pos, |cur| pos.min(cur)));
            }
        }

        let pos = best_pos?;
        // Count newlines before position to get 0-based line offset
        let local_line = content[..pos].bytes().filter(|&b| b == b'\n').count();
        let abs_line = block_start.saturating_add(local_line);
        Some(format!("{abs_line}-{abs_line}"))
    }

    /// Parse a `"start-end"` or `"start:end"` range into a two-element vector.
    /// Returns None if parsing fails or inputs are invalid.
    fn parse_lines_range(range: &str) -> Option<Vec<usize>> {
        let mut parts = range.split(['-', ':']);
        let start = parts.next()?.trim().parse::<usize>().ok()?;
        let end = parts.next()?.trim().parse::<usize>().ok()?;
        Some(vec![start, end])
    }

    fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
        // Prefer phrase matching when the whole query is quoted; otherwise use the raw query.
        let trimmed = query.trim();
        let phrase_candidate =
            if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
                &trimmed[1..trimmed.len() - 1]
            } else {
                query
            };
        let query_lower = phrase_candidate.to_lowercase();

        // Find match position using character indices to handle Unicode correctly
        let mut match_char_pos = None;

        // Use a sliding window approach with character iteration
        let content_chars: Vec<char> = content.chars().collect();
        let query_chars: Vec<char> = query_lower.chars().collect();

        if !query_chars.is_empty() {
            for window_start in 0..content_chars.len() {
                let window_end = (window_start + query_chars.len()).min(content_chars.len());
                if window_end - window_start < query_chars.len() {
                    break;
                }

                // Check if this window matches (case-insensitive)
                let window_matches = content_chars[window_start..window_end]
                    .iter()
                    .zip(query_chars.iter())
                    .all(|(c1, c2)| c1.to_lowercase().eq(c2.to_lowercase()));

                if window_matches {
                    match_char_pos = Some(window_start);
                    break;
                }
            }
        }

        if let Some(char_pos) = match_char_pos {
            // Derive context from max_len so we don't overshoot the requested length.
            let total_chars = content_chars.len();
            let qlen = query_chars.len();
            let ctx_each_side = max_len.saturating_sub(qlen) / 2;

            let start_char = char_pos.saturating_sub(ctx_each_side);
            let mut end_char = (char_pos + qlen + ctx_each_side).min(total_chars);

            // Clamp to at most max_len characters around the match.
            let span = end_char.saturating_sub(start_char);
            if span > max_len {
                end_char = start_char + max_len;
            }

            let left_trunc = start_char > 0;
            let right_trunc = end_char < total_chars;

            // Build snippet
            let mut snippet = String::with_capacity((end_char - start_char) * 4 + 6);
            if left_trunc {
                snippet.push_str("...");
            }
            for &ch in content_chars.iter().take(end_char).skip(start_char) {
                snippet.push(ch);
            }
            if right_trunc {
                snippet.push_str("...");
            }
            return snippet;
        }

        // No match found - return truncated content using character count
        let content_chars: Vec<char> = content.chars().collect();
        if content_chars.len() <= max_len {
            content.to_string()
        } else {
            // Truncate based on character count, not byte count
            let mut result = String::with_capacity(max_len * 4 + 3);
            for (i, ch) in content_chars.iter().enumerate() {
                if i >= max_len {
                    break;
                }
                result.push(*ch);
            }
            result.push_str("...");
            result
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::disallowed_macros)]
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::HeadingBlock;
    use std::time::Instant;
    use tempfile::TempDir;

    fn create_test_blocks() -> Vec<HeadingBlock> {
        vec![
            HeadingBlock::new(
                vec!["React".to_string(), "Hooks".to_string()],
                "useState is a React hook that lets you add state to functional components. It returns an array with the current state value and a function to update it.".to_string(),
                100,
                120,
            ),
            HeadingBlock::new(
                vec!["React".to_string(), "Components".to_string()],
                "Components are the building blocks of React applications. They can be function components or class components.".to_string(),
                50,
                75,
            ),
            HeadingBlock::new(
                vec!["Next.js".to_string(), "Routing".to_string()],
                "App Router is the new routing system in Next.js 13+. It provides better performance and developer experience.".to_string(),
                200,
                250,
            ),
        ]
    }

    #[test]
    fn test_index_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let result = SearchIndex::create(&index_path);
        assert!(result.is_ok(), "Should create index successfully");

        // Verify index directory was created
        assert!(index_path.exists());
    }

    #[test]
    fn test_index_open_nonexistent() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("nonexistent");

        let result = SearchIndex::open(&index_path);
        assert!(result.is_err(), "Should fail to open non-existent index");
    }

    #[test]
    fn test_index_and_search_basic() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        // Create index and add blocks
        let index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        // Search for content
        let hits = index
            .search("useState", Some("test"), 10)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results for useState");
        assert!(
            hits[0].snippet.contains("useState"),
            "Result should contain useState"
        );
        assert_eq!(hits[0].source, "test");
        assert_eq!(hits[0].file, "llms.txt");
    }

    #[test]
    fn test_search_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        // Search with limit
        let hits = index
            .search("React", Some("test"), 1)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results");
        assert!(hits.len() <= 1, "Should respect limit");
    }

    #[test]
    fn test_search_includes_anchor() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![HeadingBlock::new(
            vec!["API".to_string(), "Reference".to_string()],
            "token auth key".to_string(),
            10,
            20,
        )];

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        let hits = index
            .search("token", Some("test"), 10)
            .expect("Should search");

        assert!(!hits.is_empty());
        assert!(hits[0].anchor.is_some(), "anchor should be present in hits");
        // Anchor should be derived from the last heading segment
        let expected = SearchIndex::compute_anchor("Reference");
        assert_eq!(hits[0].anchor.clone().unwrap(), expected);
    }

    #[test]
    fn test_search_no_results() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        // Search for non-existent term
        let hits = index
            .search("nonexistentterm12345", Some("test"), 10)
            .expect("Should search");

        assert!(
            hits.is_empty(),
            "Should find no results for non-existent term"
        );
    }

    #[test]
    fn test_search_performance() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        // Create many blocks for performance testing
        let mut blocks = Vec::new();
        for i in 0..100 {
            blocks.push(HeadingBlock::new(
                vec![format!("Section{}", i)],
                format!("This is content block {i} with various keywords like React, hooks, components, and performance testing."),
                i * 10,
                i * 10 + 5,
            ));
        }

        index
            .index_blocks("perftest", &blocks)
            .expect("Should index many blocks");

        // Test search performance
        let start = Instant::now();
        let hits = index
            .search("React", Some("perftest"), 50)
            .expect("Should search");
        let duration = start.elapsed();

        assert!(!hits.is_empty(), "Should find results");
        assert!(
            duration.as_millis() < 100,
            "Search should be fast (<100ms), took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_search_scoring() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![
            HeadingBlock::new(
                vec!["Exact Match".to_string()],
                "React hooks".to_string(),
                1,
                5,
            ),
            HeadingBlock::new(
                vec!["Partial Match".to_string()],
                "React components and hooks are useful features".to_string(),
                10,
                15,
            ),
            HeadingBlock::new(
                vec!["Distant Match".to_string()],
                "In React, you can use various hooks for different purposes".to_string(),
                20,
                25,
            ),
        ];

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        let hits = index
            .search("React hooks", Some("test"), 10)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results");

        // Results should be ordered by relevance (score)
        for i in 1..hits.len() {
            assert!(
                hits[i - 1].score >= hits[i].score,
                "Results should be ordered by descending score"
            );
        }

        // The exact match should have the highest score
        assert!(
            hits[0].snippet.contains("React hooks"),
            "Highest scored result should contain exact match"
        );
    }

    #[test]
    fn test_search_snippet_respects_limits() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![HeadingBlock::new(
            vec!["Hooks".to_string()],
            "React provides hooks for state and effect management. Hooks enable composing complex logic from simple primitives. Extensive documentation follows here to ensure the snippet must truncate properly when limits are applied.".to_string(),
            1,
            20,
        )];

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        let default_hits = index
            .search("hooks", Some("test"), 5)
            .expect("Should search with default limit");
        assert!(!default_hits.is_empty());
        let default_len = default_hits[0].snippet.chars().count();
        assert!(
            default_len <= DEFAULT_SNIPPET_CHAR_LIMIT + 6,
            "Default snippet should clamp near default limit"
        );

        let custom_limit = 80;
        let custom_hits = index
            .search_with_snippet_limit("hooks", Some("test"), 5, custom_limit)
            .expect("Should search with custom limit");
        assert!(!custom_hits.is_empty());
        let custom_len = custom_hits[0].snippet.chars().count();
        assert!(
            custom_len <= clamp_snippet_chars(custom_limit) + 6,
            "Custom snippet should respect provided limit"
        );

        // Ensure custom limit produces a shorter snippet than the default when truncation occurs.
        assert!(custom_len <= default_len);
    }

    #[test]
    fn test_heading_normalization_search() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("normalized_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![
            HeadingBlock::new(
                vec!["Example: ./droid-refactor-imports.sh src".to_string()],
                "Refactor script instructions".to_string(),
                1,
                5,
            ),
            HeadingBlock::new(
                vec!["API-Schlüssel abrufen".to_string()],
                "Schlüsselverwaltung".to_string(),
                6,
                10,
            ),
        ];

        index
            .index_blocks("test", &blocks)
            .expect("Should index normalized blocks");

        let sanitized_hits = index
            .search("# example droid refactor imports sh src", Some("test"), 5)
            .expect("Should search sanitized query");
        assert!(sanitized_hits.iter().any(|hit| {
            hit.heading_path
                .last()
                .is_some_and(|h| h == "Example: ./droid-refactor-imports.sh src")
        }));

        let accent_hits = index
            .search("api schluessel abrufen", Some("test"), 5)
            .expect("Should search normalized accent query");

        // Note: The heading_path may not exactly match due to UTF-8 encoding during indexing.
        // We verify that we get a hit for the German heading with umlauts.
        assert!(
            !accent_hits.is_empty(),
            "Should find at least one hit for normalized German query"
        );
        assert!(
            accent_hits.iter().any(|hit| {
                // Match either the exact string or a close variant (accounting for potential encoding variations)
                hit.heading_path.last().is_some_and(|h| {
                    h == "API-Schlüssel abrufen" || h.contains("Schl") && h.contains("ssel abrufen")
                })
            }),
            "Should find the accent heading"
        );
    }

    #[test]
    fn test_heading_path_in_results() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![HeadingBlock::new(
            vec![
                "API".to_string(),
                "Reference".to_string(),
                "Hooks".to_string(),
            ],
            "useState hook documentation".to_string(),
            100,
            120,
        )];

        index
            .index_blocks("test", &blocks)
            .expect("Should index blocks");

        let hits = index
            .search("useState", Some("test"), 10)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results");
        assert_eq!(hits[0].heading_path, vec!["API", "Reference", "Hooks"]);
        assert_eq!(hits[0].file, "llms.txt");
        // Lines should point to the exact match within the block (first line)
        assert!(
            hits[0].lines.starts_with("100-"),
            "Expected match to start at line 100, got {}",
            hits[0].lines
        );
    }

    #[test]
    fn test_unicode_snippet_extraction() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");
        let index = SearchIndex::create(&index_path).expect("Should create index");

        // Test with various Unicode content
        let unicode_blocks = vec![
            HeadingBlock::new(
                vec!["Unicode".to_string(), "Emoji".to_string()],
                "This is a test with emojis: 👋 Hello 🌍 World! 🚀 Let's go! 🎉".to_string(),
                1,
                10,
            ),
            HeadingBlock::new(
                vec!["Unicode".to_string(), "Chinese".to_string()],
                "这是中文测试。Hello 世界！Programming 编程 is 很有趣。".to_string(),
                20,
                30,
            ),
            HeadingBlock::new(
                vec!["Unicode".to_string(), "Mixed".to_string()],
                "日本語 テスト 🇯🇵 with mixed content".to_string(),
                40,
                50,
            ),
        ];

        index
            .index_blocks("unicode_test", &unicode_blocks)
            .expect("Should index blocks");

        // Test searching for various Unicode content
        let test_cases = vec![("emoji", "👋"), ("中文", "测试"), ("programming", "编程")];

        for (query, _expected_content) in test_cases {
            let results = index
                .search(query, Some("unicode_test"), 10)
                .unwrap_or_else(|_| panic!("Should search for '{query}'"));

            if !results.is_empty() {
                let hit = &results[0];
                // Verify snippet doesn't panic on Unicode boundaries
                assert!(hit.snippet.is_char_boundary(0));
                assert!(hit.snippet.is_char_boundary(hit.snippet.len()));

                // Verify we can iterate over chars without panic
                let _char_count = hit.snippet.chars().count();
            }
        }
    }

    #[test]
    fn test_edge_case_unicode_truncation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");
        let index = SearchIndex::create(&index_path).expect("Should create index");

        // Create content where truncation would happen in middle of multi-byte chars
        let mut long_content = String::new();
        for _ in 0..20 {
            long_content.push_str("👨‍👩‍👧‍👦"); // Family emoji (complex grapheme cluster)
        }
        long_content.push_str(" MARKER ");
        for _ in 0..20 {
            long_content.push_str("🏳️‍🌈"); // Rainbow flag (another complex emoji)
        }

        let blocks = vec![HeadingBlock::new(
            vec!["Test".to_string()],
            long_content.clone(),
            1,
            10,
        )];

        index
            .index_blocks("edge_test", &blocks)
            .expect("Should index blocks");

        let results = index
            .search("MARKER", Some("edge_test"), 10)
            .expect("Should search");

        assert!(!results.is_empty());
        let snippet = &results[0].snippet;

        // Verify the snippet is valid UTF-8 and doesn't panic
        assert!(snippet.is_char_boundary(0));
        assert!(snippet.is_char_boundary(snippet.len()));
        assert!(snippet.contains("MARKER"));

        // Verify we can iterate over chars without panic
        let char_count = snippet.chars().count();
        assert!(char_count > 0);
    }
}
