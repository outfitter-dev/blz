//! Cache statistics command implementation

use anyhow::Result;
use blz_core::Storage;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::output::OutputFormat;

/// Statistics for a single source
#[derive(Debug, Serialize)]
struct SourceStats {
    alias: String,
    size_bytes: u64,
    lines: usize,
    last_updated: String,
    age_hours: i64,
}

/// Overall cache statistics
#[derive(Debug, Serialize)]
struct CacheStats {
    total_sources: usize,
    total_size_bytes: u64,
    total_lines: usize,
    cache_location: String,
    sources: Vec<SourceStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oldest_source: Option<OldestSource>,
}

#[derive(Debug, Serialize)]
struct OldestSource {
    alias: String,
    age_days: i64,
}

/// Execute the stats command
pub fn execute(format: OutputFormat, limit: Option<usize>) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    let mut source_stats = Vec::new();
    let mut total_size = 0u64;
    let mut total_lines = 0usize;
    let mut oldest: Option<(String, DateTime<Utc>)> = None;

    for alias in &sources {
        // Get source metadata
        let Some(metadata) = storage.load_source_metadata(alias)? else {
            continue; // Skip sources without metadata
        };

        // Get file size
        let llms_path = storage.llms_txt_path(alias)?;
        let size = std::fs::metadata(&llms_path).map(|m| m.len()).unwrap_or(0);

        // Get line count from metadata
        let lines = match storage.load_llms_json(alias) {
            Ok(json) => json.line_index.total_lines,
            Err(_) => 0,
        };

        // Calculate age
        let age_hours = Utc::now()
            .signed_duration_since(metadata.fetched_at)
            .num_hours();

        // Track oldest source
        if let Some((_, oldest_time)) = &oldest {
            if metadata.fetched_at < *oldest_time {
                oldest = Some((alias.clone(), metadata.fetched_at));
            }
        } else {
            oldest = Some((alias.clone(), metadata.fetched_at));
        }

        source_stats.push(SourceStats {
            alias: alias.clone(),
            size_bytes: size,
            lines,
            last_updated: metadata.fetched_at.to_rfc3339(),
            age_hours,
        });

        total_size += size;
        total_lines += lines;
    }

    // Sort by size (largest first)
    source_stats.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    // Apply limit after sorting
    if let Some(limit_count) = limit {
        source_stats.truncate(limit_count);
    }

    let oldest_source = oldest.map(|(alias, time)| {
        let age_days = Utc::now().signed_duration_since(time).num_days();
        OldestSource { alias, age_days }
    });

    let cache_location = storage.root_dir().to_string_lossy().to_string();

    let stats = CacheStats {
        total_sources: source_stats.len(),
        total_size_bytes: total_size,
        total_lines,
        cache_location,
        sources: source_stats,
        oldest_source,
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        },
        OutputFormat::Jsonl => {
            println!("{}", serde_json::to_string(&stats)?);
        },
        OutputFormat::Text => {
            print_text_stats(&stats);
        },
        OutputFormat::Raw => {
            // Raw format: just list source names
            for source in &stats.sources {
                println!("{}", source.alias);
            }
        },
    }

    Ok(())
}

fn print_text_stats(stats: &CacheStats) {
    println!("BLZ Cache Statistics");
    println!("====================");
    println!("Total Sources: {}", stats.total_sources);
    println!("Total Size: {}", format_size(stats.total_size_bytes));
    println!("Total Lines: {}", format_number(stats.total_lines));
    println!("Cache Location: {}", stats.cache_location);

    if !stats.sources.is_empty() {
        println!("\nSources:");
        for source in &stats.sources {
            let age_str = if source.age_hours < 24 {
                format!("{} hours ago", source.age_hours)
            } else {
                let days = source.age_hours / 24;
                if days == 1 {
                    "1 day ago".to_string()
                } else {
                    format!("{days} days ago")
                }
            };

            println!(
                "  {} ({}, {} lines, updated {})",
                source.alias,
                format_size(source.size_bytes),
                format_number(source.lines),
                age_str
            );
        }
    }

    if let Some(oldest) = &stats.oldest_source {
        println!(
            "\nOldest Source: {} (updated {} days ago)",
            oldest.alias, oldest.age_days
        );
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        #[allow(clippy::cast_precision_loss)]
        let result = bytes as f64 / GB as f64;
        format!("{result:.1} GB")
    } else if bytes >= MB {
        #[allow(clippy::cast_precision_loss)]
        let result = bytes as f64 / MB as f64;
        format!("{result:.1} MB")
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} bytes")
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let chunks: Vec<&str> = bytes
        .rchunks(3)
        .rev()
        .filter_map(|chunk| std::str::from_utf8(chunk).ok())
        .collect();
    chunks.join(",")
}
