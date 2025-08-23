use crate::profiling::{ComponentTimings, OperationTimer, PerformanceMetrics};
use crate::{Error, HeadingBlock, Result, SearchHit};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexReader};
use tracing::{debug, info, Level};

pub struct SearchIndex {
    index: Index,
    #[allow(dead_code)]
    schema: Schema,
    content_field: Field,
    path_field: Field,
    heading_path_field: Field,
    lines_field: Field,
    alias_field: Field,
    reader: IndexReader,
    metrics: Option<PerformanceMetrics>,
}

impl SearchIndex {
    /// Enable performance metrics collection
    pub fn with_metrics(mut self, metrics: PerformanceMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Get the performance metrics instance
    pub const fn metrics(&self) -> Option<&PerformanceMetrics> {
        self.metrics.as_ref()
    }
    pub fn create(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let heading_path_field = schema_builder.add_text_field("heading_path", TEXT | STORED);
        let lines_field = schema_builder.add_text_field("lines", STRING | STORED);
        let alias_field = schema_builder.add_text_field("alias", STRING | STORED);

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
            lines_field,
            alias_field,
            reader,
            metrics: None,
        })
    }

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
        let lines_field = schema
            .get_field("lines")
            .map_err(|_| Error::Index("Missing lines field".into()))?;
        let alias_field = schema
            .get_field("alias")
            .map_err(|_| Error::Index("Missing alias field".into()))?;

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
            lines_field,
            alias_field,
            reader,
            metrics: None,
        })
    }

    pub fn index_blocks(
        &mut self,
        alias: &str,
        file_path: &str,
        blocks: &[HeadingBlock],
    ) -> Result<()> {
        let timer = if let Some(metrics) = &self.metrics {
            OperationTimer::with_metrics(&format!("index_{alias}"), metrics.clone())
        } else {
            OperationTimer::new(&format!("index_{alias}"))
        };

        let mut timings = ComponentTimings::new();

        let mut writer = timings.time("writer_creation", || {
            self.index
                .writer(50_000_000)
                .map_err(|e| Error::Index(format!("Failed to create writer: {e}")))
        })?;

        let _deleted = timings.time("delete_existing", || {
            writer.delete_term(tantivy::Term::from_field_text(self.alias_field, alias))
        });

        let mut total_content_bytes = 0usize;

        timings.time("document_creation", || {
            for block in blocks {
                total_content_bytes += block.content.len();
                let heading_path_str = block.path.join(" > ");
                let lines_str = format!("{}-{}", block.start_line, block.end_line);

                let doc = doc!(
                    self.content_field => block.content.as_str(),  // Use &str instead of clone
                    self.path_field => file_path,
                    self.heading_path_field => heading_path_str,
                    self.lines_field => lines_str,
                    self.alias_field => alias
                );

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

    pub fn search(
        &self,
        query_str: &str,
        alias: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let timer = if let Some(metrics) = &self.metrics {
            OperationTimer::with_metrics(&format!("search_{query_str}"), metrics.clone())
        } else {
            OperationTimer::new(&format!("search_{query_str}"))
        };

        let mut timings = ComponentTimings::new();
        let mut lines_searched = 0usize;

        let searcher = timings.time("searcher_creation", || self.reader.searcher());

        let query_parser = timings.time("query_parser_creation", || {
            QueryParser::for_index(
                &self.index,
                vec![self.content_field, self.heading_path_field],
            )
        });

        // Sanitize query more efficiently with a single allocation
        let needs_escaping = query_str.chars().any(|c| {
            matches!(
                c,
                '\\' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '~'
            )
        });

        let full_query_str = if needs_escaping {
            // Only allocate if we need to escape characters
            let mut sanitized = String::with_capacity(query_str.len() * 2);

            for ch in query_str.chars() {
                match ch {
                    '\\' => sanitized.push_str("\\\\"),
                    '"' => sanitized.push_str("\\\""),
                    '(' => sanitized.push_str("\\("),
                    ')' => sanitized.push_str("\\)"),
                    '[' => sanitized.push_str("\\["),
                    ']' => sanitized.push_str("\\]"),
                    '{' => sanitized.push_str("\\{"),
                    '}' => sanitized.push_str("\\}"),
                    '^' => sanitized.push_str("\\^"),
                    '~' => sanitized.push_str("\\~"),
                    _ => sanitized.push(ch),
                }
            }

            if let Some(alias) = alias {
                // Alias is internally controlled, no need to sanitize
                format!("alias:{alias} AND ({sanitized})")
            } else {
                sanitized
            }
        } else {
            // No escaping needed, minimize allocations
            if let Some(alias) = alias {
                format!("alias:{alias} AND ({query_str})")
            } else {
                query_str.to_string()
            }
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

                let alias = self.get_field_text(&doc, self.alias_field)?;
                let file = self.get_field_text(&doc, self.path_field)?;
                let heading_path_str = self.get_field_text(&doc, self.heading_path_field)?;
                let lines = self.get_field_text(&doc, self.lines_field)?;
                let content = self.get_field_text(&doc, self.content_field)?;

                // Count lines for metrics
                lines_searched += content.lines().count();

                let heading_path: Vec<String> = heading_path_str
                    .split(" > ")
                    .map(std::string::ToString::to_string)
                    .collect();

                let snippet = self.extract_snippet(&content, query_str, 100);

                hits.push(SearchHit {
                    alias,
                    file,
                    heading_path,
                    lines,
                    snippet,
                    score,
                    source_url: None,
                    checksum: String::new(),
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

    fn get_field_text(&self, doc: &tantivy::TantivyDocument, field: Field) -> Result<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| Error::Index("Field not found in document".into()))
    }

    fn extract_snippet(&self, content: &str, query: &str, max_len: usize) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            // Find safe UTF-8 boundaries around the match
            let context_before = 50;
            let context_after = 50;

            // Calculate byte positions
            let byte_start = pos.saturating_sub(context_before);
            let byte_end = (pos + query.len() + context_after).min(content.len());

            // Find the nearest character boundary for start
            let start = if byte_start == 0 {
                0
            } else {
                // Find the start of the character at or before byte_start
                content
                    .char_indices()
                    .take_while(|(i, _)| *i <= byte_start)
                    .last()
                    .map_or(0, |(i, _)| i)
            };

            // Find the nearest character boundary for end
            let end = content
                .char_indices()
                .find(|(i, _)| *i >= byte_end)
                .map_or(content.len(), |(i, _)| i);

            let mut snippet = String::with_capacity(end - start + 6);
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&content[start..end]);
            if end < content.len() {
                snippet.push_str("...");
            }

            return snippet;
        }

        // No match found - return truncated content
        if content.len() <= max_len {
            content.to_string()
        } else {
            // Find safe UTF-8 boundary for truncation
            let boundary = content
                .char_indices()
                .take_while(|(i, _)| *i < max_len)
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8());

            if boundary == 0 {
                // Edge case: first character is longer than max_len
                String::from("...")
            } else {
                format!("{}...", &content[..boundary])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeadingBlock;
    use std::time::Instant;
    use tempfile::TempDir;

    fn create_test_blocks() -> Vec<HeadingBlock> {
        vec![
            HeadingBlock {
                path: vec!["React".to_string(), "Hooks".to_string()],
                content: "useState is a React hook that lets you add state to functional components. It returns an array with the current state value and a function to update it.".to_string(),
                start_line: 100,
                end_line: 120,
            },
            HeadingBlock {
                path: vec!["React".to_string(), "Components".to_string()],
                content: "Components are the building blocks of React applications. They can be function components or class components.".to_string(),
                start_line: 50,
                end_line: 75,
            },
            HeadingBlock {
                path: vec!["Next.js".to_string(), "Routing".to_string()],
                content: "App Router is the new routing system in Next.js 13+. It provides better performance and developer experience.".to_string(),
                start_line: 200,
                end_line: 250,
            },
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
        let mut index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", "test.md", &blocks)
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
        assert_eq!(hits[0].alias, "test");
        assert_eq!(hits[0].file, "test.md");
    }

    #[test]
    fn test_search_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let mut index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", "test.md", &blocks)
            .expect("Should index blocks");

        // Search with limit
        let hits = index
            .search("React", Some("test"), 1)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results");
        assert!(hits.len() <= 1, "Should respect limit");
    }

    #[test]
    fn test_search_no_results() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let mut index = SearchIndex::create(&index_path).expect("Should create index");
        let blocks = create_test_blocks();

        index
            .index_blocks("test", "test.md", &blocks)
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

        let mut index = SearchIndex::create(&index_path).expect("Should create index");

        // Create many blocks for performance testing
        let mut blocks = Vec::new();
        for i in 0..100 {
            blocks.push(HeadingBlock {
                path: vec![format!("Section{}", i)],
                content: format!("This is content block {i} with various keywords like React, hooks, components, and performance testing."),
                start_line: i * 10,
                end_line: i * 10 + 5,
            });
        }

        index
            .index_blocks("perftest", "large.md", &blocks)
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

        let mut index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![
            HeadingBlock {
                path: vec!["Exact Match".to_string()],
                content: "React hooks".to_string(),
                start_line: 1,
                end_line: 5,
            },
            HeadingBlock {
                path: vec!["Partial Match".to_string()],
                content: "React components and hooks are useful features".to_string(),
                start_line: 10,
                end_line: 15,
            },
            HeadingBlock {
                path: vec!["Distant Match".to_string()],
                content: "In React, you can use various hooks for different purposes".to_string(),
                start_line: 20,
                end_line: 25,
            },
        ];

        index
            .index_blocks("test", "test.md", &blocks)
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
    fn test_heading_path_in_results() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");

        let mut index = SearchIndex::create(&index_path).expect("Should create index");

        let blocks = vec![HeadingBlock {
            path: vec![
                "API".to_string(),
                "Reference".to_string(),
                "Hooks".to_string(),
            ],
            content: "useState hook documentation".to_string(),
            start_line: 100,
            end_line: 120,
        }];

        index
            .index_blocks("test", "api.md", &blocks)
            .expect("Should index blocks");

        let hits = index
            .search("useState", Some("test"), 10)
            .expect("Should search");

        assert!(!hits.is_empty(), "Should find results");
        assert_eq!(hits[0].heading_path, vec!["API", "Reference", "Hooks"]);
        assert_eq!(hits[0].file, "api.md");
        assert_eq!(hits[0].lines, "100-120");
    }

    #[test]
    fn test_unicode_snippet_extraction() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index");
        let mut index = SearchIndex::create(&index_path).expect("Should create index");

        // Test with various Unicode content
        let unicode_blocks = vec![
            HeadingBlock {
                path: vec!["Unicode".to_string(), "Emoji".to_string()],
                content: "This is a test with emojis: 👋 Hello 🌍 World! 🚀 Let's go! 🎉"
                    .to_string(),
                start_line: 1,
                end_line: 10,
            },
            HeadingBlock {
                path: vec!["Unicode".to_string(), "Chinese".to_string()],
                content: "这是中文测试。Hello 世界！Programming 编程 is 很有趣。".to_string(),
                start_line: 20,
                end_line: 30,
            },
            HeadingBlock {
                path: vec!["Unicode".to_string(), "Mixed".to_string()],
                content: "日本語 テスト 🇯🇵 with mixed content".to_string(),
                start_line: 40,
                end_line: 50,
            },
        ];

        index
            .index_blocks("unicode_test", "test.md", &unicode_blocks)
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
        let mut index = SearchIndex::create(&index_path).expect("Should create index");

        // Create content where truncation would happen in middle of multi-byte chars
        let mut long_content = String::new();
        for _ in 0..20 {
            long_content.push_str("👨‍👩‍👧‍👦"); // Family emoji (complex grapheme cluster)
        }
        long_content.push_str(" MARKER ");
        for _ in 0..20 {
            long_content.push_str("🏳️‍🌈"); // Rainbow flag (another complex emoji)
        }

        let blocks = vec![HeadingBlock {
            path: vec!["Test".to_string()],
            content: long_content.clone(),
            start_line: 1,
            end_line: 10,
        }];

        index
            .index_blocks("edge_test", "test.md", &blocks)
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
