//! Health check command - comprehensive cache and source diagnostics

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::output::OutputFormat;
use crate::utils::staleness::{self, DEFAULT_STALE_AFTER_DAYS};

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub recommendations: Vec<String>,
    pub cache_info: CacheInfo,
    pub source_health: SourceHealth,
}

#[derive(Debug, Serialize)]
pub struct CacheInfo {
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub total_size_bytes: u64,
    pub total_sources: usize,
    pub total_files: usize,
}

#[derive(Debug, Serialize)]
pub struct SourceHealth {
    pub total: usize,
    pub healthy: usize,
    pub stale: usize,
    pub corrupted: usize,
    pub stale_sources: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub fixable: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Error,
}

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
            "Run `blz update --all` to refresh {stale_count} stale sources"
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
        println!("  Updating stale sources...");
        let metrics = blz_core::PerformanceMetrics::default();
        for alias in &report.source_health.stale_sources {
            match crate::commands::update::execute(alias, metrics.clone(), true).await {
                Ok(()) => println!("    ✓ Updated {alias}"),
                Err(e) => eprintln!("    ✗ Failed to update {alias}: {e}"),
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
            let path = entry.path();

            if path.is_dir() {
                total += calculate_cache_size(&path)?;
            } else if path.is_file() {
                total += path.metadata()?.len();
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
            let path = entry.path();

            if path.is_dir() {
                count += count_files_recursive(&path)?;
            } else {
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

    // Source health
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
