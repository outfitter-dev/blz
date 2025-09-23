//! List command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;

use crate::output::OutputFormat;
use crate::utils::formatting::get_alias_color;

/// Execute the list command to show all cached sources
pub async fn execute(format: OutputFormat, status: bool, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        match format {
            OutputFormat::Json => {
                println!("[]");
            },
            OutputFormat::Jsonl => {
                // No output when empty
            },
            OutputFormat::Text => {
                if !quiet {
                    println!("No sources found. Use 'blz add' to add sources.");
                }
            },
        }
        return Ok(());
    }

    let mut source_info = Vec::new();

    for source in &sources {
        if let Ok(llms_json) = storage.load_llms_json(source) {
            if status {
                // Include health/status details
                source_info.push(serde_json::json!({
                    "alias": source,
                    "source": source, // canonical handle (same as alias for now)
                    "url": llms_json.source.url,
                    "fetchedAt": llms_json.source.fetched_at,
                    "lines": llms_json.line_index.total_lines,
                    "sha256": llms_json.source.sha256,
                    "etag": llms_json.source.etag,
                    "lastModified": llms_json.source.last_modified,
                    "aliases": llms_json.source.aliases,
                }));
            } else {
                source_info.push(serde_json::json!({
                    "alias": source,
                    "source": source, // canonical handle (same as alias for now)
                    "url": llms_json.source.url,
                    "fetchedAt": llms_json.source.fetched_at,
                    "lines": llms_json.line_index.total_lines,
                    "sha256": llms_json.source.sha256,
                    "aliases": llms_json.source.aliases
                }));
            }
        }
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&source_info)?;
            println!("{json}");
        },
        OutputFormat::Jsonl => {
            for info in source_info {
                println!("{}", serde_json::to_string(&info)?);
            }
        },
        OutputFormat::Text => {
            display_sources_text(&sources, &storage, status, quiet);
        },
    }

    Ok(())
}

fn display_sources_text(sources: &[String], storage: &Storage, status: bool, quiet: bool) {
    if !quiet {
        println!("\nCached sources:\n");
    }

    for (i, source) in sources.iter().enumerate() {
        if let Ok(llms_json) = storage.load_llms_json(source) {
            let source_colored = get_alias_color(source, i);

            println!(
                "  {} {}",
                source_colored,
                llms_json.source.url.bright_black()
            );
            println!(
                "    Fetched: {}",
                llms_json.source.fetched_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("    Lines: {}", llms_json.line_index.total_lines);
            if status {
                if let Some(etag) = &llms_json.source.etag {
                    println!("    ETag: {etag}");
                }
                if let Some(lm) = &llms_json.source.last_modified {
                    println!("    Last-Modified: {lm}");
                }
                println!("    Checksum: {}", llms_json.source.sha256);
            }
            if !quiet {
                println!();
            }
        }
    }
}
