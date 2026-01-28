//! Health check command - comprehensive cache and source diagnostics

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::commands::sync::generated::{is_generated_source, load_generate_manifest};
use crate::output::OutputFormat;
use crate::utils::staleness::{self, DEFAULT_STALE_AFTER_DAYS};

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Check passed with no issues.
    Healthy,
    /// Check passed with warnings.
    Warning,
    /// Check failed with an error.
    Error,
}

// ============================================================
// Individual Source Health Types (for generated source tracking)
// ============================================================

/// Type of documentation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceHealthType {
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
    pub source_type: SourceHealthType,
    /// Total line count in the document.
    pub line_count: usize,
    /// Health status.
    pub status: HealthStatus,
    /// Human-readable status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Number of failed pages (for generated sources).
    pub failed_pages: usize,
    /// Whether a native llms-full.txt is available for upgrade.
    pub upgrade_available: bool,
}

impl SourceHealthEntry {
    /// Create a new source health entry.
    #[must_use]
    pub const fn new(alias: String, source_type: SourceHealthType) -> Self {
        Self {
            alias,
            source_type,
            line_count: 0,
            status: HealthStatus::Healthy,
            status_message: None,
            failed_pages: 0,
            upgrade_available: false,
        }
    }

    /// Set the line count.
    #[must_use]
    pub const fn with_line_count(mut self, count: usize) -> Self {
        self.line_count = count;
        self
    }

    /// Set the status with optional message.
    #[must_use]
    pub fn with_status(mut self, status: HealthStatus, message: Option<String>) -> Self {
        self.status = status;
        self.status_message = message;
        self
    }

    /// Set the failed pages count.
    #[must_use]
    pub const fn with_failed_pages(mut self, count: usize) -> Self {
        self.failed_pages = count;
        self
    }

    /// Set upgrade availability.
    ///
    /// Note: Currently unused in production code as checking upgrade availability
    /// requires async probing. Will be used when `--check-upgrades` flag is added.
    #[must_use]
    #[allow(dead_code)]
    pub const fn with_upgrade_available(mut self, available: bool) -> Self {
        self.upgrade_available = available;
        self
    }
}

/// Generate recommendations based on source health entries.
///
/// Returns actionable recommendations for sources that have:
/// - Failed pages that should be retried
/// - Upgrade availability to native source
#[must_use]
pub fn generate_source_health_recommendations(sources: &[SourceHealthEntry]) -> Vec<String> {
    let mut recommendations = Vec::new();

    for source in sources {
        // Recommend upgrade for generated sources with native available
        if source.source_type == SourceHealthType::Generated && source.upgrade_available {
            recommendations.push(format!(
                "Upgrade '{}' to native source (blz sync {} --upgrade)",
                source.alias, source.alias
            ));
        }

        // Recommend retry for sources with failed pages
        if source.failed_pages > 0 {
            let page_word = if source.failed_pages == 1 {
                "page"
            } else {
                "pages"
            };
            recommendations.push(format!(
                "Retry {} failed {} in '{}' (blz sync {})",
                source.failed_pages, page_word, source.alias, source.alias
            ));
        }
    }

    recommendations
}

/// Execute the doctor command.
///
/// # Errors
///
/// Returns an error if health checks, fixes, or output serialization fails.
pub async fn execute(format: OutputFormat, fix: bool) -> Result<()> {
    let storage = Storage::new()?;
    let mut report = run_health_checks(&storage)?;

    if fix {
        apply_fixes(&storage, &mut report).await?;
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        },
        OutputFormat::Jsonl => {
            for check in &report.checks {
                let json = serde_json::to_string(check)?;
                println!("{json}");
            }
        },
        OutputFormat::Text | OutputFormat::Raw => {
            print_text_report(&report, fix);
        },
    }

    // Exit with error code if there are errors
    if report.overall_status == HealthStatus::Error {
        std::process::exit(1);
    }

    Ok(())
}

fn run_health_checks(storage: &Storage) -> Result<HealthReport> {
    let cache_dir = storage.root_dir();
    let config_dir = storage.config_dir();

    let mut checks = Vec::new();
    let mut recommendations = Vec::new();

    checks.push(directory_check("Cache Directory", cache_dir)?);
    checks.push(directory_check("Config Directory", config_dir)?);

    let (disk_check, disk_recommendation, total_size) = disk_usage_check(cache_dir)?;
    checks.push(disk_check);
    if let Some(rec) = disk_recommendation {
        recommendations.push(rec);
    }

    let sources = storage.list_sources();
    let (sources_check, source_recommendations, source_health) =
        source_health_check(storage, &sources)?;
    checks.push(sources_check);
    recommendations.extend(source_recommendations);

    let (index_check, index_recommendation) = index_health_check(storage, &sources);
    checks.push(index_check);
    if let Some(rec) = index_recommendation {
        recommendations.push(rec);
    }

    // Collect individual source health entries
    let source_entries = collect_source_health_entries(storage, &sources);

    // Add recommendations for generated sources
    let source_entry_recs = generate_source_health_recommendations(&source_entries);
    recommendations.extend(source_entry_recs);

    let total_files = count_files_recursive(&cache_dir.to_path_buf())?;
    let overall_status = compute_overall_status(&checks);

    Ok(HealthReport {
        overall_status,
        checks,
        recommendations,
        cache_info: CacheInfo {
            cache_dir: cache_dir.to_path_buf(),
            config_dir: config_dir.to_path_buf(),
            total_size_bytes: total_size,
            total_sources: sources.len(),
            total_files,
        },
        source_health,
        source_entries,
    })
}

fn directory_check(name: &str, path: &Path) -> Result<HealthCheck> {
    let exists = path.exists();
    let writable = exists && !path.metadata()?.permissions().readonly();
    let status = if writable {
        HealthStatus::Healthy
    } else {
        HealthStatus::Error
    };
    let message = if writable {
        format!("{name} exists and is writable: {}", path.display())
    } else if exists {
        format!("{name} exists but is read-only: {}", path.display())
    } else {
        format!("{name} missing: {}", path.display())
    };

    Ok(HealthCheck {
        name: name.to_string(),
        status,
        message,
        fixable: false,
    })
}

fn disk_usage_check(cache_dir: &Path) -> Result<(HealthCheck, Option<String>, u64)> {
    const WARN_THRESHOLD_MB: u64 = 1_000; // 1 GB practical limit
    let total_size = calculate_cache_size(&cache_dir.to_path_buf())?;
    let total_size_mb = total_size / 1_048_576;
    let disk_space_ok = total_size_mb < WARN_THRESHOLD_MB;

    let message = format!("Cache size: {}", format_megabytes(total_size));
    let check = HealthCheck {
        name: "Disk Usage".to_string(),
        status: if disk_space_ok {
            HealthStatus::Healthy
        } else {
            HealthStatus::Warning
        },
        message,
        fixable: !disk_space_ok,
    };

    let recommendation = (!disk_space_ok).then_some(
        "Consider running `blz clear` or removing unused sources to free up space".to_string(),
    );

    Ok((check, recommendation, total_size))
}

fn source_health_check(
    storage: &Storage,
    aliases: &[String],
) -> Result<(HealthCheck, Vec<String>, SourceHealth)> {
    let mut stale_sources = Vec::new();
    let mut corrupted_count: usize = 0;

    for alias in aliases {
        if let Some(metadata) = storage.load_source_metadata(alias)? {
            if staleness::is_stale(metadata.fetched_at, DEFAULT_STALE_AFTER_DAYS) {
                stale_sources.push(alias.clone());
            }

            if let Ok(llms_path) = storage.llms_txt_path(alias) {
                if !llms_path.exists() {
                    corrupted_count += 1;
                }
            }
        }
    }

    let stale_count = stale_sources.len();
    let status = match (stale_count, corrupted_count) {
        (0, 0) => HealthStatus::Healthy,
        (_, c) if c > 0 => HealthStatus::Error,
        _ => HealthStatus::Warning,
    };

    let message = format!(
        "{total} total sources: {stale_count} stale (>{DEFAULT_STALE_AFTER_DAYS} days), {corrupted_count} corrupted",
        total = aliases.len()
    );

    let mut recommendations = Vec::new();
    if stale_count > 0 {
        recommendations.push(format!(
            "Run `blz refresh --all` (deprecated alias: `blz update --all`) to refresh {stale_count} stale sources"
        ));
    }
    if corrupted_count > 0 {
        recommendations.push("Remove and re-add corrupted sources".to_string());
    }

    let healthy = aliases.len().saturating_sub(stale_count + corrupted_count);

    let health = SourceHealth {
        total: aliases.len(),
        healthy,
        stale: stale_count,
        corrupted: corrupted_count,
        stale_sources,
    };

    let check = HealthCheck {
        name: "Source Integrity".to_string(),
        status,
        message,
        fixable: stale_count > 0,
    };

    Ok((check, recommendations, health))
}

fn index_health_check(storage: &Storage, aliases: &[String]) -> (HealthCheck, Option<String>) {
    let mut missing_indices = 0usize;
    for alias in aliases {
        if let Ok(index_dir) = storage.index_dir(alias) {
            if !index_dir.exists() {
                missing_indices += 1;
            }
        }
    }

    let status = if missing_indices == 0 {
        HealthStatus::Healthy
    } else {
        HealthStatus::Warning
    };
    let message = if missing_indices == 0 {
        format!("All {} sources have search indices", aliases.len())
    } else {
        format!("{missing_indices} sources missing search indices")
    };

    let recommendation = (missing_indices > 0)
        .then_some("Re-index sources with missing indices by updating them".to_string());

    let check = HealthCheck {
        name: "Search Indices".to_string(),
        status,
        message,
        fixable: missing_indices > 0,
    };

    (check, recommendation)
}

fn compute_overall_status(checks: &[HealthCheck]) -> HealthStatus {
    if checks.iter().any(|c| c.status == HealthStatus::Error) {
        HealthStatus::Error
    } else if checks.iter().any(|c| c.status == HealthStatus::Warning) {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    }
}

/// Collect health information for each individual source.
///
/// For generated sources, this includes failed page counts from the manifest.
fn collect_source_health_entries(storage: &Storage, aliases: &[String]) -> Vec<SourceHealthEntry> {
    let mut entries = Vec::new();

    for alias in aliases {
        let is_generated = is_generated_source(storage, alias);
        let source_type = if is_generated {
            SourceHealthType::Generated
        } else {
            SourceHealthType::Native
        };

        let mut entry = SourceHealthEntry::new(alias.clone(), source_type);

        // Get line count from llms.json
        if let Ok(llms_json) = storage.load_llms_json(alias) {
            entry = entry.with_line_count(llms_json.line_index.total_lines);
        }

        // For generated sources, get failed page count from manifest
        if is_generated {
            if let Ok(manifest) = load_generate_manifest(storage, alias) {
                let failed_count = manifest.failed.len();
                entry = entry.with_failed_pages(failed_count);

                if failed_count > 0 {
                    let page_word = if failed_count == 1 { "page" } else { "pages" };
                    entry = entry.with_status(
                        HealthStatus::Warning,
                        Some(format!("{failed_count} failed {page_word}")),
                    );
                }
            }
        }

        entries.push(entry);
    }

    entries
}

fn format_megabytes(bytes: u64) -> String {
    const MB: u64 = 1_048_576;
    let whole = bytes / MB;
    let fraction = ((bytes % MB) * 100) / MB;
    format!("{whole}.{fraction:02} MB")
}

async fn apply_fixes(storage: &Storage, report: &mut HealthReport) -> Result<()> {
    println!("{}", "Applying automatic fixes...".bold());

    // Fix 1: Update stale sources
    if !report.source_health.stale_sources.is_empty() {
        println!("  Refreshing stale sources...");
        let metrics = blz_core::PerformanceMetrics::default();
        for alias in &report.source_health.stale_sources {
            match crate::commands::refresh::execute(
                alias,
                metrics.clone(),
                true,
                false,
                None,
                false,
            )
            .await
            {
                Ok(()) => println!("    ✓ Refreshed {alias}"),
                Err(e) => eprintln!("    ✗ Failed to refresh {alias}: {e}"),
            }
        }
    }

    // Re-run checks to update report
    *report = run_health_checks(storage)?;

    Ok(())
}

fn calculate_cache_size(dir: &PathBuf) -> Result<u64> {
    let mut total = 0u64;

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();

            if file_type.is_dir() {
                total += calculate_cache_size(&path)?;
            } else if file_type.is_file() {
                total += entry.metadata()?.len();
            }
        }
    }

    Ok(total)
}

fn count_files_recursive(dir: &PathBuf) -> Result<usize> {
    let mut count = 0;

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();

            if file_type.is_dir() {
                count += count_files_recursive(&path)?;
            } else if file_type.is_file() {
                count += 1;
            }
        }
    }

    Ok(count)
}

fn print_text_report(report: &HealthReport, fix_applied: bool) {
    let status_icon = match report.overall_status {
        HealthStatus::Healthy => "✓".green(),
        HealthStatus::Warning => "⚠".yellow(),
        HealthStatus::Error => "✗".red(),
    };

    println!("\n{} {}", status_icon, "BLZ Health Check".bold());
    println!("{}", "=".repeat(50));

    // Cache info
    println!("\n{}", "Cache Information:".bold());
    println!("  Location: {}", report.cache_info.cache_dir.display());
    println!(
        "  Size: {}",
        format_megabytes(report.cache_info.total_size_bytes)
    );
    println!("  Sources: {}", report.cache_info.total_sources);
    println!("  Files: {}", report.cache_info.total_files);

    // Source health (aggregate)
    println!("\n{}", "Source Health:".bold());
    println!(
        "  {} healthy",
        report.source_health.healthy.to_string().green()
    );
    println!(
        "  {} stale",
        report.source_health.stale.to_string().yellow()
    );
    println!(
        "  {} corrupted",
        report.source_health.corrupted.to_string().red()
    );

    // Individual source entries (if any have notable status)
    if !report.source_entries.is_empty() {
        println!("\n{}", "Sources:".bold());
        for entry in &report.source_entries {
            let status_icon = match entry.status {
                HealthStatus::Healthy => "✓".green(),
                HealthStatus::Warning => "⚠".yellow(),
                HealthStatus::Error => "✗".red(),
            };

            let type_str = match entry.source_type {
                SourceHealthType::Native => "native",
                SourceHealthType::Generated => "generated",
            };

            // Format: ✓ react       native     15,230 lines   fresh
            let line_count_str = format_line_count(entry.line_count);
            let status_msg = entry.status_message.as_deref().unwrap_or(
                if entry.status == HealthStatus::Healthy {
                    "healthy"
                } else {
                    ""
                },
            );

            println!(
                "  {status_icon} {:<12} {:<10} {:>12}   {}",
                entry.alias, type_str, line_count_str, status_msg
            );

            // Show upgrade recommendation inline for generated sources
            if entry.upgrade_available {
                println!("    {} Native llms-full.txt now available!", "→".cyan());
            }
        }
    }

    // Checks
    println!("\n{}", "Health Checks:".bold());
    for check in &report.checks {
        let check_icon = match check.status {
            HealthStatus::Healthy => "✓".green(),
            HealthStatus::Warning => "⚠".yellow(),
            HealthStatus::Error => "✗".red(),
        };
        println!("  {check_icon} {}: {}", check.name, check.message);
    }

    // Recommendations
    if !report.recommendations.is_empty() {
        println!("\n{}", "Recommendations:".bold().yellow());
        for rec in &report.recommendations {
            println!("  • {rec}");
        }
    }

    if fix_applied {
        println!("\n{}", "✓ Automatic fixes applied".green().bold());
    } else if report.checks.iter().any(|c| c.fixable) {
        println!("\n{}", "Run with --fix to apply automatic fixes".cyan());
    }
}

/// Format line count with thousands separator.
fn format_line_count(count: usize) -> String {
    if count == 0 {
        "0 lines".to_string()
    } else if count >= 1000 {
        let thousands = count / 1000;
        format!("{thousands},{:03} lines", count % 1000)
    } else {
        format!("{count} lines")
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;

    // --------------------------------------------------------
    // SourceHealthEntry Tests
    // --------------------------------------------------------

    #[test]
    fn test_source_health_entry_native() {
        let entry = SourceHealthEntry::new("react".to_string(), SourceHealthType::Native)
            .with_line_count(15230)
            .with_status(HealthStatus::Healthy, None);

        assert_eq!(entry.alias, "react");
        assert_eq!(entry.source_type, SourceHealthType::Native);
        assert_eq!(entry.line_count, 15230);
        assert_eq!(entry.status, HealthStatus::Healthy);
        assert!(!entry.upgrade_available);
        assert_eq!(entry.failed_pages, 0);
    }

    #[test]
    fn test_source_health_entry_generated_with_failures() {
        let entry = SourceHealthEntry::new("hono".to_string(), SourceHealthType::Generated)
            .with_line_count(12890)
            .with_failed_pages(1)
            .with_status(HealthStatus::Warning, Some("1 failed page".to_string()))
            .with_upgrade_available(true);

        assert_eq!(entry.alias, "hono");
        assert_eq!(entry.source_type, SourceHealthType::Generated);
        assert_eq!(entry.line_count, 12890);
        assert_eq!(entry.status, HealthStatus::Warning);
        assert_eq!(entry.failed_pages, 1);
        assert!(entry.upgrade_available);
        assert_eq!(entry.status_message, Some("1 failed page".to_string()));
    }

    // --------------------------------------------------------
    // Recommendation Generation Tests
    // --------------------------------------------------------

    #[test]
    fn test_recommendations_upgrade() {
        let sources = vec![
            SourceHealthEntry::new("hono".to_string(), SourceHealthType::Generated)
                .with_line_count(12890)
                .with_upgrade_available(true),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Upgrade"));
        assert!(recs[0].contains("hono"));
        assert!(recs[0].contains("--upgrade"));
    }

    #[test]
    fn test_recommendations_failed_pages() {
        let sources = vec![
            SourceHealthEntry::new("effect".to_string(), SourceHealthType::Generated)
                .with_line_count(9450)
                .with_failed_pages(2)
                .with_status(HealthStatus::Warning, Some("2 failed pages".to_string())),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Retry"));
        assert!(recs[0].contains("2"));
        assert!(recs[0].contains("failed"));
        assert!(recs[0].contains("effect"));
    }

    #[test]
    fn test_recommendations_single_failed_page() {
        let sources = vec![
            SourceHealthEntry::new("test".to_string(), SourceHealthType::Generated)
                .with_failed_pages(1),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("1 failed page")); // singular
    }

    #[test]
    fn test_recommendations_multiple_failed_pages() {
        let sources = vec![
            SourceHealthEntry::new("test".to_string(), SourceHealthType::Generated)
                .with_failed_pages(3),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("3 failed pages")); // plural
    }

    #[test]
    fn test_no_recommendations_healthy_native() {
        let sources = vec![
            SourceHealthEntry::new("react".to_string(), SourceHealthType::Native)
                .with_line_count(15230)
                .with_status(HealthStatus::Healthy, None),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert!(recs.is_empty());
    }

    #[test]
    fn test_no_recommendations_healthy_generated() {
        let sources = vec![
            SourceHealthEntry::new("test".to_string(), SourceHealthType::Generated)
                .with_status(HealthStatus::Healthy, None),
        ];

        let recs = generate_source_health_recommendations(&sources);

        assert!(recs.is_empty());
    }

    #[test]
    fn test_recommendations_combined() {
        let sources = vec![
            SourceHealthEntry::new("hono".to_string(), SourceHealthType::Generated)
                .with_upgrade_available(true)
                .with_failed_pages(1),
            SourceHealthEntry::new("react".to_string(), SourceHealthType::Native),
            SourceHealthEntry::new("effect".to_string(), SourceHealthType::Generated)
                .with_failed_pages(2),
        ];

        let recs = generate_source_health_recommendations(&sources);

        // hono: upgrade + retry, effect: retry
        assert_eq!(recs.len(), 3);
        assert!(
            recs.iter()
                .any(|r| r.contains("Upgrade") && r.contains("hono"))
        );
        assert!(
            recs.iter()
                .any(|r| r.contains("Retry") && r.contains("hono"))
        );
        assert!(
            recs.iter()
                .any(|r| r.contains("Retry") && r.contains("effect"))
        );
    }

    // --------------------------------------------------------
    // Format Line Count Tests
    // --------------------------------------------------------

    #[test]
    fn test_format_line_count_zero() {
        assert_eq!(format_line_count(0), "0 lines");
    }

    #[test]
    fn test_format_line_count_small() {
        assert_eq!(format_line_count(100), "100 lines");
        assert_eq!(format_line_count(999), "999 lines");
    }

    #[test]
    fn test_format_line_count_thousands() {
        assert_eq!(format_line_count(1000), "1,000 lines");
        assert_eq!(format_line_count(15230), "15,230 lines");
        assert_eq!(format_line_count(100000), "100,000 lines");
    }

    // --------------------------------------------------------
    // Serialization Tests
    // --------------------------------------------------------

    #[test]
    fn test_source_health_entry_json_serialization() {
        let entry = SourceHealthEntry::new("hono".to_string(), SourceHealthType::Generated)
            .with_line_count(12890)
            .with_failed_pages(1)
            .with_status(HealthStatus::Warning, Some("1 failed page".to_string()))
            .with_upgrade_available(true);

        let json = serde_json::to_value(&entry).unwrap();

        assert_eq!(json["alias"], "hono");
        assert_eq!(json["sourceType"], "generated");
        assert_eq!(json["lineCount"], 12890);
        assert_eq!(json["status"], "warning");
        assert_eq!(json["statusMessage"], "1 failed page");
        assert_eq!(json["failedPages"], 1);
        assert_eq!(json["upgradeAvailable"], true);
    }

    #[test]
    fn test_source_health_entry_json_skips_null_message() {
        let entry = SourceHealthEntry::new("react".to_string(), SourceHealthType::Native);

        let json = serde_json::to_value(&entry).unwrap();

        // statusMessage should not be present when None
        assert!(json.get("statusMessage").is_none());
    }

    #[test]
    fn test_source_health_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SourceHealthType::Native).unwrap(),
            "\"native\""
        );
        assert_eq!(
            serde_json::to_string(&SourceHealthType::Generated).unwrap(),
            "\"generated\""
        );
    }
}
