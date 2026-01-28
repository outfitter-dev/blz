//! Manifest types for generated sources.
//!
//! The [`GenerateManifest`] tracks metadata about generated sources including:
//! - Schema version for future migrations
//! - Discovery information (how URLs were found)
//! - Page metadata with line ranges
//! - Generation statistics
//! - Optional backup information

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::page_cache::{BackupInfo, FailedPage, PageId};

/// The current schema version for `GenerateManifest`.
///
/// Bump this when making breaking changes to the manifest structure
/// to enable migration logic.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Type of documentation source.
///
/// Distinguishes between sources generated from web scraping
/// and native llms.txt files from upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeneratedSourceType {
    /// Generated from web scraping via Firecrawl.
    Generated,
    /// Native llms.txt/llms-full.txt from upstream.
    Native,
}

/// Discovery information about how URLs were found.
///
/// Tracks the original input and which discovery methods
/// yielded URLs for scraping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryInfo {
    /// Original input (domain or URL).
    pub input: String,
    /// URL to llms.txt index (if found).
    pub index_url: Option<String>,
    /// URL to sitemap.xml (if found).
    pub sitemap_url: Option<String>,
    /// Count of URLs from each source.
    pub url_sources: UrlSourceCounts,
}

/// Counts of URLs from different discovery sources.
///
/// Used to track how many URLs came from each discovery method.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlSourceCounts {
    /// URLs discovered from llms.txt links section.
    pub llms_txt: usize,
    /// URLs discovered from sitemap.xml.
    pub sitemap: usize,
    /// URLs discovered via crawling.
    pub crawl: usize,
}

/// Metadata about a page in the generated document.
///
/// Tracks which page contributed to which section of the
/// assembled llms.txt output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMeta {
    /// Page identifier (derived from URL).
    pub id: PageId,
    /// Source URL.
    pub url: String,
    /// Page title (from HTML or first heading).
    pub title: Option<String>,
    /// Line range in assembled document (e.g., "1-50").
    pub line_range: String,
}

/// Statistics about the generation.
///
/// Provides summary statistics for the generation operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateStats {
    /// Total pages discovered (successful + failed).
    pub total_pages: usize,
    /// Pages successfully scraped.
    pub successful_pages: usize,
    /// Pages that failed to scrape.
    pub failed_pages: usize,
    /// Total lines in assembled document.
    pub total_lines: usize,
}

/// Manifest for a generated source.
///
/// Contains all metadata about how a source was generated,
/// enabling incremental updates and schema migrations.
///
/// ## Schema Versioning
///
/// The `schema_version` field enables future migrations when
/// the manifest structure changes. Check [`SCHEMA_VERSION`]
/// for the current version.
///
/// ## Example
///
/// ```rust
/// use blz_core::generate::{GenerateManifest, DiscoveryInfo, UrlSourceCounts};
///
/// let discovery = DiscoveryInfo {
///     input: "example.com".to_string(),
///     index_url: None,
///     sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
///     url_sources: UrlSourceCounts { llms_txt: 0, sitemap: 10, crawl: 0 },
/// };
///
/// let manifest = GenerateManifest::new(
///     discovery,
///     vec![],
///     vec![],
///     "1.2.0".to_string(),
/// );
///
/// assert_eq!(manifest.schema_version, "1.0.0");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateManifest {
    /// Schema version for migrations.
    pub schema_version: String,
    /// Type of source.
    #[serde(rename = "type")]
    pub source_type: GeneratedSourceType,
    /// When the document was generated.
    pub generated_at: DateTime<Utc>,
    /// How URLs were discovered.
    pub discovery: DiscoveryInfo,
    /// Metadata about each page.
    pub pages: Vec<PageMeta>,
    /// Pages that failed to scrape.
    pub failed_pages: Vec<FailedPage>,
    /// Generation statistics.
    pub stats: GenerateStats,
    /// Firecrawl CLI version used.
    pub firecrawl_version: String,
    /// Backup info if pages were backed up before this generation.
    pub backup: Option<BackupInfo>,
}

impl GenerateManifest {
    /// Create a new manifest with current timestamp.
    ///
    /// Automatically calculates statistics from the provided pages
    /// and failed pages.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::generate::{GenerateManifest, DiscoveryInfo, UrlSourceCounts};
    ///
    /// let discovery = DiscoveryInfo {
    ///     input: "docs.rs".to_string(),
    ///     index_url: None,
    ///     sitemap_url: None,
    ///     url_sources: UrlSourceCounts::default(),
    /// };
    ///
    /// let manifest = GenerateManifest::new(
    ///     discovery,
    ///     vec![],
    ///     vec![],
    ///     "1.0.0".to_string(),
    /// );
    ///
    /// assert_eq!(manifest.stats.total_pages, 0);
    /// ```
    #[must_use]
    pub fn new(
        discovery: DiscoveryInfo,
        pages: Vec<PageMeta>,
        failed_pages: Vec<FailedPage>,
        firecrawl_version: String,
    ) -> Self {
        let stats = Self::calculate_stats(&pages, &failed_pages);

        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            source_type: GeneratedSourceType::Generated,
            generated_at: Utc::now(),
            discovery,
            pages,
            failed_pages,
            stats,
            firecrawl_version,
            backup: None,
        }
    }

    /// Create with backup info.
    ///
    /// Builder method to attach backup information to the manifest.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::generate::{GenerateManifest, DiscoveryInfo, UrlSourceCounts};
    /// use blz_core::page_cache::BackupInfo;
    /// use chrono::Utc;
    ///
    /// let discovery = DiscoveryInfo {
    ///     input: "example.com".to_string(),
    ///     index_url: None,
    ///     sitemap_url: None,
    ///     url_sources: UrlSourceCounts::default(),
    /// };
    ///
    /// let backup = BackupInfo {
    ///     backed_up_at: Utc::now(),
    ///     reason: "pre-upgrade".to_string(),
    ///     page_count: 50,
    ///     path: "pages.bak.20240115".to_string(),
    /// };
    ///
    /// let manifest = GenerateManifest::new(
    ///     discovery,
    ///     vec![],
    ///     vec![],
    ///     "1.0.0".to_string(),
    /// ).with_backup(backup);
    ///
    /// assert!(manifest.backup.is_some());
    /// ```
    #[must_use]
    pub fn with_backup(mut self, backup: BackupInfo) -> Self {
        self.backup = Some(backup);
        self
    }

    /// Calculate stats from pages and `failed_pages`.
    ///
    /// Parses line ranges to determine total lines in the assembled document.
    fn calculate_stats(pages: &[PageMeta], failed_pages: &[FailedPage]) -> GenerateStats {
        let successful_pages = pages.len();
        let failed_count = failed_pages.len();
        let total_pages = successful_pages + failed_count;

        // Calculate total lines by finding the maximum end line across all pages
        let total_lines = pages
            .iter()
            .filter_map(|p| {
                // Parse "start-end" format
                let parts: Vec<&str> = p.line_range.split('-').collect();
                if parts.len() == 2 {
                    parts[1].parse::<usize>().ok()
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        GenerateStats {
            total_pages,
            successful_pages,
            failed_pages: failed_count,
            total_lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_discovery() -> DiscoveryInfo {
        DiscoveryInfo {
            input: "hono.dev".to_string(),
            index_url: Some("https://hono.dev/llms.txt".to_string()),
            sitemap_url: Some("https://hono.dev/sitemap.xml".to_string()),
            url_sources: UrlSourceCounts {
                llms_txt: 45,
                sitemap: 12,
                crawl: 0,
            },
        }
    }

    fn create_test_page_meta() -> PageMeta {
        PageMeta {
            id: PageId::from_url("https://hono.dev/docs/getting-started"),
            url: "https://hono.dev/docs/getting-started".to_string(),
            title: Some("Getting Started".to_string()),
            line_range: "1-50".to_string(),
        }
    }

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(SCHEMA_VERSION, "1.0.0");
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = GenerateManifest::new(
            create_test_discovery(),
            vec![create_test_page_meta()],
            vec![],
            "1.2.0".to_string(),
        );

        assert_eq!(manifest.schema_version, "1.0.0");
        assert_eq!(manifest.source_type, GeneratedSourceType::Generated);
        assert_eq!(manifest.pages.len(), 1);
        assert!(manifest.backup.is_none());
    }

    #[test]
    fn test_manifest_with_backup() {
        let backup = BackupInfo {
            backed_up_at: Utc::now(),
            reason: "pre-upgrade".to_string(),
            page_count: 50,
            path: "pages.bak.20240115".to_string(),
        };

        let manifest =
            GenerateManifest::new(create_test_discovery(), vec![], vec![], "1.2.0".to_string())
                .with_backup(backup);

        assert!(manifest.backup.is_some());
        assert_eq!(
            manifest.backup.as_ref().map(|b| b.reason.as_str()),
            Some("pre-upgrade")
        );
    }

    #[test]
    fn test_stats_calculation() {
        let pages = vec![
            PageMeta {
                line_range: "1-50".to_string(),
                ..create_test_page_meta()
            },
            PageMeta {
                line_range: "51-100".to_string(),
                ..create_test_page_meta()
            },
        ];
        let failed = vec![FailedPage::new(
            "https://example.com/bad".to_string(),
            "timeout".to_string(),
        )];

        let manifest =
            GenerateManifest::new(create_test_discovery(), pages, failed, "1.2.0".to_string());

        assert_eq!(manifest.stats.total_pages, 3); // 2 successful + 1 failed
        assert_eq!(manifest.stats.successful_pages, 2);
        assert_eq!(manifest.stats.failed_pages, 1);
        assert_eq!(manifest.stats.total_lines, 100);
    }

    #[test]
    fn test_stats_calculation_empty() {
        let manifest =
            GenerateManifest::new(create_test_discovery(), vec![], vec![], "1.2.0".to_string());

        assert_eq!(manifest.stats.total_pages, 0);
        assert_eq!(manifest.stats.successful_pages, 0);
        assert_eq!(manifest.stats.failed_pages, 0);
        assert_eq!(manifest.stats.total_lines, 0);
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = GenerateManifest::new(
            create_test_discovery(),
            vec![create_test_page_meta()],
            vec![],
            "1.2.0".to_string(),
        );

        let json = serde_json::to_string_pretty(&manifest).expect("Should serialize");

        // Verify camelCase
        assert!(json.contains("schemaVersion"));
        assert!(json.contains("generatedAt"));
        assert!(json.contains("\"type\""));
        assert!(json.contains("urlSources"));
        assert!(json.contains("lineRange"));

        // Roundtrip
        let roundtrip: GenerateManifest = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(manifest.schema_version, roundtrip.schema_version);
        assert_eq!(manifest.pages.len(), roundtrip.pages.len());
    }

    #[test]
    fn test_source_type_serialization() {
        assert_eq!(
            serde_json::to_string(&GeneratedSourceType::Generated).expect("Should serialize"),
            "\"generated\""
        );
        assert_eq!(
            serde_json::to_string(&GeneratedSourceType::Native).expect("Should serialize"),
            "\"native\""
        );
    }

    #[test]
    fn test_source_type_deserialization() {
        let generated: GeneratedSourceType =
            serde_json::from_str("\"generated\"").expect("Should deserialize");
        assert_eq!(generated, GeneratedSourceType::Generated);

        let native: GeneratedSourceType =
            serde_json::from_str("\"native\"").expect("Should deserialize");
        assert_eq!(native, GeneratedSourceType::Native);
    }

    #[test]
    fn test_discovery_info_serialization() {
        let discovery = create_test_discovery();
        let json = serde_json::to_string_pretty(&discovery).expect("Should serialize");

        assert!(json.contains("indexUrl"));
        assert!(json.contains("sitemapUrl"));
        assert!(json.contains("urlSources"));
        assert!(json.contains("llmsTxt"));

        let roundtrip: DiscoveryInfo = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(discovery.input, roundtrip.input);
        assert_eq!(
            discovery.url_sources.llms_txt,
            roundtrip.url_sources.llms_txt
        );
    }

    #[test]
    fn test_page_meta_serialization() {
        let page = create_test_page_meta();
        let json = serde_json::to_string_pretty(&page).expect("Should serialize");

        assert!(json.contains("lineRange"));

        let roundtrip: PageMeta = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(page.url, roundtrip.url);
        assert_eq!(page.line_range, roundtrip.line_range);
    }

    #[test]
    fn test_url_source_counts_default() {
        let counts = UrlSourceCounts::default();
        assert_eq!(counts.llms_txt, 0);
        assert_eq!(counts.sitemap, 0);
        assert_eq!(counts.crawl, 0);
    }

    #[test]
    fn test_generate_stats_default() {
        let stats = GenerateStats::default();
        assert_eq!(stats.total_pages, 0);
        assert_eq!(stats.successful_pages, 0);
        assert_eq!(stats.failed_pages, 0);
        assert_eq!(stats.total_lines, 0);
    }

    #[test]
    fn test_manifest_firecrawl_version() {
        let manifest =
            GenerateManifest::new(create_test_discovery(), vec![], vec![], "1.5.3".to_string());
        assert_eq!(manifest.firecrawl_version, "1.5.3");
    }

    #[test]
    fn test_manifest_generated_at_is_recent() {
        let before = Utc::now();
        let manifest =
            GenerateManifest::new(create_test_discovery(), vec![], vec![], "1.0.0".to_string());
        let after = Utc::now();

        assert!(manifest.generated_at >= before);
        assert!(manifest.generated_at <= after);
    }

    #[test]
    fn test_stats_with_non_contiguous_line_ranges() {
        // Test that we correctly find max end line even with gaps
        let pages = vec![
            PageMeta {
                id: PageId::from_url("https://example.com/1"),
                url: "https://example.com/1".to_string(),
                title: None,
                line_range: "1-50".to_string(),
            },
            PageMeta {
                id: PageId::from_url("https://example.com/2"),
                url: "https://example.com/2".to_string(),
                title: None,
                line_range: "100-200".to_string(), // Gap between 50-100
            },
        ];

        let manifest =
            GenerateManifest::new(create_test_discovery(), pages, vec![], "1.0.0".to_string());

        assert_eq!(manifest.stats.total_lines, 200);
    }
}
