//! Content assembler for generating llms-full.txt documents.
//!
//! Assembles cached web pages into a single document following Bun's llms-full.txt
//! format, with section markers and line range tracking for navigation.
//!
//! ## Format
//!
//! Each page is formatted as:
//! ```markdown
//! # Page Title
//! Source: https://example.com/page
//!
//! Page content here...
//! ```
//!
//! Pages are separated by two blank lines.
//!
//! ## Example
//!
//! ```rust
//! use blz_core::generate::ContentAssembler;
//! use blz_core::page_cache::PageCacheEntry;
//!
//! let pages = vec![
//!     PageCacheEntry::new(
//!         "https://example.com/docs".to_string(),
//!         "Hello world".to_string(),
//!     ).with_title(Some("Getting Started".to_string())),
//! ];
//!
//! let result = ContentAssembler::assemble(&pages);
//! assert!(result.content.contains("# Getting Started"));
//! assert!(result.content.contains("Source: https://example.com/docs"));
//! ```

use crate::generate::GenerateStats;
use crate::page_cache::{PageCacheEntry, PageId};

/// Entry in the line map showing which page owns which lines.
///
/// Maps a range of lines in the assembled document back to the
/// source page, enabling navigation and citation.
#[derive(Debug, Clone)]
pub struct LineMapEntry {
    /// Page identifier (derived from URL).
    pub page_id: PageId,
    /// Source URL.
    pub url: String,
    /// Line range in format "start-end" (1-indexed).
    pub line_range: String,
    /// Page title.
    pub title: Option<String>,
}

/// Result of content assembly.
///
/// Contains the assembled markdown document along with metadata
/// for navigation and statistics.
#[derive(Debug, Clone)]
pub struct AssemblyResult {
    /// The assembled markdown content.
    pub content: String,
    /// Map of line ranges to pages.
    pub line_map: Vec<LineMapEntry>,
    /// Statistics about the assembly.
    pub stats: GenerateStats,
}

/// Assembles cached pages into llms-full.txt format.
///
/// Combines multiple [`PageCacheEntry`] items into a single markdown
/// document with section markers and line tracking.
pub struct ContentAssembler;

impl ContentAssembler {
    /// Assemble pages into a single document.
    ///
    /// Each page is formatted with:
    /// - Title as H1 heading
    /// - Source URL on next line
    /// - Blank line
    /// - Page content
    /// - Two blank lines between pages
    ///
    /// Line numbers are 1-indexed to match standard text editor conventions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::generate::ContentAssembler;
    /// use blz_core::page_cache::PageCacheEntry;
    ///
    /// let pages = vec![
    ///     PageCacheEntry::new(
    ///         "https://example.com/intro".to_string(),
    ///         "Welcome to the docs.".to_string(),
    ///     ).with_title(Some("Introduction".to_string())),
    /// ];
    ///
    /// let result = ContentAssembler::assemble(&pages);
    /// assert_eq!(result.stats.successful_pages, 1);
    /// assert_eq!(result.line_map.len(), 1);
    /// ```
    #[must_use]
    pub fn assemble(pages: &[PageCacheEntry]) -> AssemblyResult {
        if pages.is_empty() {
            return AssemblyResult {
                content: String::new(),
                line_map: Vec::new(),
                stats: GenerateStats::default(),
            };
        }

        let mut content = String::new();
        let mut line_map = Vec::with_capacity(pages.len());
        let mut current_line: usize = 1;

        for (idx, page) in pages.iter().enumerate() {
            // Add separator between pages (2 blank lines)
            // Three newlines create two blank lines:
            // \n ends previous content, \n creates blank line 1, \n creates blank line 2
            if idx > 0 {
                content.push_str("\n\n\n");
                current_line += 2;
            }

            let start_line = current_line;

            // Format the section
            let section = Self::format_section(page);
            content.push_str(&section);

            // Calculate lines in this section:
            // Header (1) + Source (1) + blank (1) + content lines
            let content_lines = page.markdown.lines().count();
            let section_lines = 3 + content_lines;
            let end_line = start_line + section_lines - 1;

            line_map.push(LineMapEntry {
                page_id: page.id.clone(),
                url: page.url.clone(),
                line_range: format!("{start_line}-{end_line}"),
                title: page.title.clone(),
            });

            current_line = end_line + 1;
        }

        // Calculate total lines (end line of last page)
        let total_lines = line_map
            .last()
            .and_then(|entry| {
                entry
                    .line_range
                    .split('-')
                    .nth(1)
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .unwrap_or(0);

        let stats = GenerateStats {
            total_pages: pages.len(),
            successful_pages: pages.len(),
            failed_pages: 0,
            total_lines,
        };

        AssemblyResult {
            content,
            line_map,
            stats,
        }
    }

    /// Format a single page section.
    ///
    /// Returns the formatted section including:
    /// - H1 heading with title (or "Untitled")
    /// - Source URL line
    /// - Blank line
    /// - Page content
    fn format_section(page: &PageCacheEntry) -> String {
        format!(
            "# {}\nSource: {}\n\n{}",
            page.title.as_deref().unwrap_or("Untitled"),
            page.url,
            page.markdown
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_page(url: &str, title: &str, content: &str) -> PageCacheEntry {
        PageCacheEntry::new(url.to_string(), content.to_string())
            .with_title(Some(title.to_string()))
    }

    #[test]
    fn test_assemble_single_page() {
        let pages = vec![create_test_page(
            "https://example.com/getting-started",
            "Getting Started",
            "Welcome to the guide.\n\nThis is the first step.",
        )];

        let result = ContentAssembler::assemble(&pages);

        assert!(result.content.contains("# Getting Started"));
        assert!(
            result
                .content
                .contains("Source: https://example.com/getting-started")
        );
        assert!(result.content.contains("Welcome to the guide."));
        assert_eq!(result.line_map.len(), 1);
    }

    #[test]
    fn test_assemble_multiple_pages() {
        let pages = vec![
            create_test_page("https://example.com/page1", "Page One", "Content 1"),
            create_test_page("https://example.com/page2", "Page Two", "Content 2"),
        ];

        let result = ContentAssembler::assemble(&pages);

        assert!(result.content.contains("# Page One"));
        assert!(result.content.contains("# Page Two"));
        assert_eq!(result.line_map.len(), 2);
    }

    #[test]
    fn test_line_map_ranges() {
        let pages = vec![
            create_test_page(
                "https://example.com/page1",
                "Page One",
                "Line 1\nLine 2\nLine 3",
            ),
            create_test_page("https://example.com/page2", "Page Two", "Line A\nLine B"),
        ];

        let result = ContentAssembler::assemble(&pages);

        // First page: header (1) + source (1) + blank (1) + content (3) = lines 1-6
        assert_eq!(result.line_map[0].line_range, "1-6");

        // Gap: 2 blank lines (lines 7-8)
        // Second page: header (1) + source (1) + blank (1) + content (2) = lines 9-13
        assert_eq!(result.line_map[1].line_range, "9-13");
    }

    #[test]
    fn test_stats_calculation() {
        let pages = vec![
            create_test_page("https://example.com/page1", "Page One", "Line 1\nLine 2"),
            create_test_page("https://example.com/page2", "Page Two", "Line A"),
        ];

        let result = ContentAssembler::assemble(&pages);

        assert_eq!(result.stats.successful_pages, 2);
        assert!(result.stats.total_lines > 0);
        // Page 1: 3 + 2 = 5 lines (1-5)
        // Gap: 2 lines (6-7)
        // Page 2: 3 + 1 = 4 lines (8-11)
        assert_eq!(result.stats.total_lines, 11);
    }

    #[test]
    fn test_untitled_page() {
        let page = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "Content here".to_string(),
        );
        // No title set

        let result = ContentAssembler::assemble(&[page]);

        assert!(result.content.contains("# Untitled"));
    }

    #[test]
    fn test_empty_pages() {
        let result = ContentAssembler::assemble(&[]);

        assert!(result.content.is_empty());
        assert!(result.line_map.is_empty());
        assert_eq!(result.stats.successful_pages, 0);
        assert_eq!(result.stats.total_lines, 0);
    }

    #[test]
    fn test_page_separation() {
        let pages = vec![
            create_test_page("https://example.com/page1", "Page One", "Content 1"),
            create_test_page("https://example.com/page2", "Page Two", "Content 2"),
        ];

        let result = ContentAssembler::assemble(&pages);

        // Pages should be separated by two blank lines
        assert!(result.content.contains("Content 1\n\n\n# Page Two"));
    }

    #[test]
    fn test_line_map_has_page_ids() {
        let pages = vec![create_test_page(
            "https://example.com/page",
            "Test",
            "Content",
        )];

        let result = ContentAssembler::assemble(&pages);

        assert!(result.line_map[0].page_id.as_str().starts_with("pg_"));
        assert_eq!(result.line_map[0].url, "https://example.com/page");
        assert_eq!(result.line_map[0].title, Some("Test".to_string()));
    }

    #[test]
    fn test_format_section() {
        let page = create_test_page(
            "https://example.com/docs",
            "Documentation",
            "Line 1\nLine 2",
        );

        let section = ContentAssembler::format_section(&page);

        assert_eq!(
            section,
            "# Documentation\nSource: https://example.com/docs\n\nLine 1\nLine 2"
        );
    }

    #[test]
    fn test_three_pages_line_ranges() {
        let pages = vec![
            create_test_page("https://example.com/a", "Page A", "A1\nA2"),
            create_test_page("https://example.com/b", "Page B", "B1"),
            create_test_page("https://example.com/c", "Page C", "C1\nC2\nC3"),
        ];

        let result = ContentAssembler::assemble(&pages);

        // Page A: header(1) + source(1) + blank(1) + content(2) = 5 lines (1-5)
        assert_eq!(result.line_map[0].line_range, "1-5");
        // Gap: 2 lines (6-7)
        // Page B: header(1) + source(1) + blank(1) + content(1) = 4 lines (8-11)
        assert_eq!(result.line_map[1].line_range, "8-11");
        // Gap: 2 lines (12-13)
        // Page C: header(1) + source(1) + blank(1) + content(3) = 6 lines (14-19)
        assert_eq!(result.line_map[2].line_range, "14-19");
    }

    #[test]
    fn test_page_with_empty_content() {
        let page = PageCacheEntry::new("https://example.com/empty".to_string(), String::new())
            .with_title(Some("Empty".to_string()));

        let result = ContentAssembler::assemble(&[page]);

        // Empty content means 0 content lines
        // header(1) + source(1) + blank(1) + content(0) = 3 lines
        assert_eq!(result.line_map[0].line_range, "1-3");
        assert_eq!(result.stats.total_lines, 3);
    }

    #[test]
    fn test_content_format_matches_spec() {
        let page = create_test_page(
            "https://hono.dev/docs/getting-started",
            "Getting Started",
            "Content from getting started page...",
        );

        let result = ContentAssembler::assemble(&[page]);

        // Should match the format from the spec:
        // # Getting Started
        // Source: https://hono.dev/docs/getting-started
        //
        // Content from getting started page...
        let expected_lines = [
            "# Getting Started",
            "Source: https://hono.dev/docs/getting-started",
            "",
            "Content from getting started page...",
        ];

        let actual_lines: Vec<&str> = result.content.lines().collect();
        assert_eq!(actual_lines, expected_lines);
    }

    #[test]
    fn test_multiline_content_preserved() {
        let content = "First paragraph.\n\nSecond paragraph.\n\n- List item 1\n- List item 2";
        let page = create_test_page("https://example.com/multi", "Multiline", content);

        let result = ContentAssembler::assemble(&[page]);

        // Verify the content is preserved exactly
        assert!(result.content.contains(content));
        // Content has 6 lines
        assert_eq!(result.line_map[0].line_range, "1-9"); // 3 header lines + 6 content
    }
}
