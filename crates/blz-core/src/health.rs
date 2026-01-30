//! Health check types for diagnostics and source health monitoring.
//!
//! These types are shared between the CLI and MCP server to provide
//! consistent health reporting across interfaces.

use serde::Serialize;
use std::path::PathBuf;

/// Overall health report for the cache.
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    /// Overall status derived from all checks.
    pub overall_status: HealthStatus,
    /// Individual check results.
    pub checks: Vec<HealthCheck>,
    /// Suggested remediation steps.
    pub recommendations: Vec<String>,
    /// Cache directory metadata and sizes.
    pub cache_info: CacheInfo,
    /// Aggregate source health statistics.
    pub source_health: SourceHealth,
    /// Per-source health entries (includes generated source details).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub source_entries: Vec<SourceHealthEntry>,
}

/// Cache directory metadata and sizes.
#[derive(Debug, Clone, Serialize)]
pub struct CacheInfo {
    /// Root cache directory.
    pub cache_dir: PathBuf,
    /// Configuration directory.
    pub config_dir: PathBuf,
    /// Total size of cached files in bytes.
    pub total_size_bytes: u64,
    /// Number of cached sources.
    pub total_sources: usize,
    /// Number of cached files.
    pub total_files: usize,
}

/// Aggregate source health statistics.
#[derive(Debug, Clone, Serialize)]
pub struct SourceHealth {
    /// Total sources inspected.
    pub total: usize,
    /// Sources with no issues.
    pub healthy: usize,
    /// Sources that are stale.
    pub stale: usize,
    /// Sources with corrupted caches.
    pub corrupted: usize,
    /// Aliases of stale sources.
    pub stale_sources: Vec<String>,
}

/// Individual health check result.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    /// Human-friendly check name.
    pub name: String,
    /// Status of the check.
    pub status: HealthStatus,
    /// Message describing the result.
    pub message: String,
    /// Whether the issue can be auto-fixed.
    pub fixable: bool,
}

/// Health status for checks and sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Check passed with no issues.
    Healthy,
    /// Check passed with warnings.
    Warning,
    /// Check failed with an error.
    Error,
}

/// Kind of documentation source (native vs generated).
///
/// This is distinct from `crate::types::SourceType` which describes
/// how the source is fetched (remote URL vs local file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Native llms.txt/llms-full.txt source.
    Native,
    /// Generated via Firecrawl scraping.
    Generated,
}

/// Health status for an individual source.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthEntry {
    /// Source alias.
    pub alias: String,
    /// Type of source (native or generated).
    pub source_type: SourceKind,
    /// Total line count in the document.
    pub line_count: usize,
    /// Whether the source is stale.
    pub is_stale: bool,
    /// Days since last update (if stale).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_days: Option<u64>,
    /// Number of failed pages (for generated sources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_pages: Option<usize>,
    /// Whether a native llms-full.txt is available for upgrade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_available: Option<bool>,
}
