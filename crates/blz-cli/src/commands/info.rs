//! Command to display detailed information about a cached source

use anyhow::{Context, Result};
use blz_core::Storage;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::output::OutputFormat;

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
}

/// Execute the info command
pub async fn execute_info(alias: &str, format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if !storage.exists(&canonical) {
        anyhow::bail!(
            "Source '{}' not found. Run `blz list` to see available sources.",
            alias
        );
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

    let cache_path = llms_file.parent().map(PathBuf::from).unwrap_or_default();

    let info = SourceInfo {
        alias: canonical,
        url: metadata.url.clone(),
        variant: format!("{:?}", metadata.variant),
        aliases: metadata.aliases.clone(),
        lines,
        size_bytes,
        last_updated: Some(metadata.fetched_at.to_rfc3339()),
        etag: metadata.etag.clone(),
        checksum: Some(metadata.sha256),
        cache_path,
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
mod tests {
    use super::*;

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
}
