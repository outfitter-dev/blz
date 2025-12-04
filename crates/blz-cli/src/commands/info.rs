//! Command to display detailed information about a cached source

use anyhow::{Context, Result};
use blz_core::{HeadingFilterStats, Storage};
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::output::OutputFormat;
use crate::utils::count_headings;

/// Detailed information about a source
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    /// Source alias
    pub alias: String,
    /// Source URL
    pub url: String,
    /// Variant (llms, llms-full, or custom)
    pub variant: String,
    /// Additional aliases for this source
    pub aliases: Vec<String>,
    /// Total number of lines in the document
    pub lines: usize,
    /// Total number of headings in the document
    pub headings: usize,
    /// Size in bytes
    pub size_bytes: u64,
    /// Last updated timestamp (ISO 8601)
    pub last_updated: Option<String>,
    /// `ETag` for conditional fetching
    pub etag: Option<String>,
    /// SHA256 checksum
    pub checksum: Option<String>,
    /// Path to cached source directory
    pub cache_path: PathBuf,
    /// Language filtering statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_stats: Option<HeadingFilterStats>,
}

/// Execute the info command
pub async fn execute_info(alias: &str, format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if !storage.exists(&canonical) {
        anyhow::bail!("Source '{alias}' not found. Run `blz list` to see available sources.");
    }

    let llms = storage
        .load_llms_json(&canonical)
        .with_context(|| format!("Failed to load metadata for '{canonical}'"))?;
    let metadata = llms.metadata.clone();

    let llms_file = storage.llms_txt_path(&canonical)?;

    // Read file stats
    let file_metadata = fs::metadata(&llms_file)
        .with_context(|| format!("Failed to read source file for '{canonical}'"))?;

    let size_bytes = file_metadata.len();
    let lines = llms.line_index.total_lines;
    let headings = count_headings(&llms.toc);

    let cache_path = llms_file.parent().map(PathBuf::from).unwrap_or_default();

    let info = SourceInfo {
        alias: canonical,
        url: metadata.url.clone(),
        variant: format!("{:?}", metadata.variant),
        aliases: metadata.aliases.clone(),
        lines,
        headings,
        size_bytes,
        last_updated: Some(metadata.fetched_at.to_rfc3339()),
        etag: metadata.etag.clone(),
        checksum: Some(metadata.sha256),
        cache_path,
        filter_stats: llms.filter_stats,
    };

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&info)
                .context("Failed to serialize source info to JSON")?;
            println!("{json}");
        },
        OutputFormat::Jsonl => {
            let json =
                serde_json::to_string(&info).context("Failed to serialize source info to JSONL")?;
            println!("{json}");
        },
        OutputFormat::Text => {
            print_text_info(&info);
        },
        OutputFormat::Raw => {
            // Raw format: just key facts, no formatting
            println!("{}", info.url);
        },
    }

    Ok(())
}

fn print_text_info(info: &SourceInfo) {
    println!("Source: {}", info.alias);
    println!("URL: {}", info.url);
    println!("Variant: {}", info.variant);

    if !info.aliases.is_empty() {
        println!("Aliases: {}", info.aliases.join(", "));
    }

    println!("Lines: {}", format_number(info.lines));
    println!("Headings: {}", format_number(info.headings));
    println!("Size: {}", format_bytes(info.size_bytes));

    if let Some(updated) = &info.last_updated {
        println!("Last Updated: {updated}");
    }

    if let Some(etag) = &info.etag {
        println!("ETag: {etag}");
    }

    if let Some(checksum) = &info.checksum {
        println!("Checksum: {checksum}");
    }

    println!("Cache Location: {}", info.cache_path.display());

    // Display language filtering information
    println!();
    if let Some(stats) = &info.filter_stats {
        println!("Language Filtering:");
        let status_text = if stats.enabled {
            "enabled".green()
        } else {
            "disabled".yellow()
        };
        println!("  Status: {status_text}");

        if stats.enabled && stats.headings_rejected > 0 {
            let percentage = percentage(stats.headings_rejected, stats.headings_total);
            println!(
                "  Filtered: {} headings ({percentage:.1}%)",
                format_number(stats.headings_rejected)
            );
            println!("  Reason: {}", stats.reason);
        }
    } else {
        println!(
            "Language Filtering: {} (added before filtering feature)",
            "unknown".yellow()
        );
    }
}

fn percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        let result = (count as f64 / total as f64) * 100.0;
        result
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];

    // Find appropriate unit
    let mut unit_index = 0;
    let mut divisor = 1u64;

    while bytes >= divisor * 1024 && unit_index < UNITS.len() - 1 {
        divisor *= 1024;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} {}", UNITS[unit_index])
    } else {
        // Use f64 for fractional display
        #[allow(clippy::cast_precision_loss)]
        let size_f64 = bytes as f64 / divisor as f64;
        format!("{size_f64:.1} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::output::OutputFormat;
    use crate::utils::test_support;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn new(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var_os(key),
            }
        }

        fn set<S: AsRef<std::ffi::OsStr>>(&self, value: S) {
            // SAFETY: Environment mutations are synchronised via env_mutex() to avoid
            // concurrent access across tests, and `self.key` is a static str while
            // `value` implements AsRef<OsStr>, satisfying set_var requirements.
            unsafe {
                std::env::set_var(self.key, value);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: Protected by env_mutex(); see set().
            unsafe {
                if let Some(value) = self.original.clone() {
                    std::env::set_var(self.key, value);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123_456), "123,456");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_258_291), "1.2 MB");
    }

    #[test]
    fn test_execute_info_returns_context_for_invalid_metadata() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        let temp = TempDir::new().expect("create temp dir");
        let data_dir = temp.path().join("data");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&data_dir).expect("create data dir");
        fs::create_dir_all(&config_dir).expect("create config dir");

        let data_dir_str = data_dir.to_string_lossy().to_string();
        let config_dir_str = config_dir.to_string_lossy().to_string();
        let error = {
            let env_lock = test_support::env_mutex()
                .lock()
                .expect("env mutex poisoned");
            let data_guard = EnvGuard::new("BLZ_DATA_DIR");
            data_guard.set(&data_dir_str);
            let config_guard = EnvGuard::new("BLZ_GLOBAL_CONFIG_DIR");
            config_guard.set(&config_dir_str);

            let storage = Storage::new().expect("initialize storage");
            storage
                .ensure_tool_dir("demo")
                .expect("create alias directory");
            let llms_path = storage
                .llms_json_path("demo")
                .expect("resolve llms.json path");
            fs::write(&llms_path, "{ invalid json").expect("write malformed llms.json");

            let error = runtime
                .block_on(execute_info("demo", OutputFormat::Json))
                .expect_err("expected invalid metadata to error");

            drop(config_guard);
            drop(data_guard);
            drop(env_lock);

            error
        };
        let message = error.to_string();

        assert!(
            message.contains("Failed to load metadata for 'demo'"),
            "missing context in error: {message}"
        );
        let chain_messages: Vec<String> = error
            .chain()
            .map(std::string::ToString::to_string)
            .collect();
        assert!(
            chain_messages.len() >= 2,
            "Expected error chain with context and source, got: {chain_messages:?}"
        );
        assert!(
            chain_messages
                .iter()
                .any(|m| m.contains("Failed to parse llms.json")),
            "missing parse failure detail: {chain_messages:?}"
        );
    }

    #[test]
    fn test_percentage() {
        assert!((percentage(0, 100) - 0.0).abs() < f64::EPSILON);
        assert!((percentage(50, 100) - 50.0).abs() < f64::EPSILON);
        assert!((percentage(100, 100) - 100.0).abs() < f64::EPSILON);
        assert!((percentage(33, 100) - 33.0).abs() < f64::EPSILON);
        assert!((percentage(0, 0) - 0.0).abs() < f64::EPSILON); // Edge case: no total
    }

    #[test]
    fn test_percentage_precision() {
        let result = percentage(1, 3);
        assert!((result - 33.333_333).abs() < 0.001);
    }
}
