use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde_json::json;

use crate::output::OutputFormat;
use crate::utils::history_log;
use crate::utils::preferences::{self, CliPreferences};

pub fn show(
    prefs: &CliPreferences,
    limit: usize,
    format: OutputFormat,
    clear: bool,
    clear_before: Option<&str>,
) -> Result<()> {
    // Handle clear operations
    if clear {
        history_log::clear_all()?;
        match format {
            OutputFormat::Text => {
                println!("{}", "All search history cleared.".green());
            },
            OutputFormat::Json | OutputFormat::Jsonl => {
                println!("{}", json!({"status": "ok", "cleared": "all"}));
            },
            OutputFormat::Raw => {
                // exit code communicates success
            },
        }
        return Ok(());
    }

    if let Some(date_str) = clear_before {
        let cutoff_date = parse_date(date_str)?;
        history_log::clear_before(&cutoff_date)?;
        match format {
            OutputFormat::Text => {
                println!(
                    "{}",
                    format!(
                        "Search history before {} cleared.",
                        cutoff_date.to_rfc3339()
                    )
                    .green()
                );
            },
            OutputFormat::Json | OutputFormat::Jsonl => {
                println!(
                    "{}",
                    json!({
                        "status": "ok",
                        "cleared": "before",
                        "cutoff": cutoff_date.to_rfc3339()
                    })
                );
            },
            OutputFormat::Raw => {
                // exit code communicates success
            },
        }
        return Ok(());
    }

    let limit = limit.max(1);
    let entries: Vec<_> = history_log::recent_for_active_scope(limit);
    match format {
        OutputFormat::Text => {
            render_text(prefs, &entries);
        },
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&json!(entries))?);
        },
        OutputFormat::Jsonl => {
            for entry in entries {
                println!("{}", serde_json::to_string(&entry)?);
            }
        },
        OutputFormat::Raw => {
            // Raw format: just print queries, one per line
            for entry in entries {
                println!("{}", entry.query);
            }
        },
    }
    Ok(())
}

fn render_text(prefs: &CliPreferences, entries: &[preferences::SearchHistoryEntry]) {
    let defaults = prefs.default_show_components();
    println!(
        "{} {}",
        "Default show:".bright_black(),
        preferences::format_show_components(&defaults)
    );
    println!(
        "{} {}",
        "Default snippet lines:".bright_black(),
        prefs.default_snippet_lines()
    );
    println!(
        "{} {}",
        "Default score precision:".bright_black(),
        prefs.default_score_precision()
    );
    println!();

    if entries.is_empty() {
        println!("{}", "No recent searches recorded.".bright_black());
        return;
    }

    for (idx, entry) in entries.iter().enumerate() {
        println!(
            "{} {}",
            format!("{}.", idx + 1).green(),
            entry.query.clone()
        );
        if let Some(alias) = &entry.source {
            println!("   {} {}", "alias:".bright_black(), alias);
        }
        println!("   {} {}", "format:".bright_black(), &entry.format);
        if !entry.show.is_empty() {
            println!("   {} {}", "show:".bright_black(), entry.show.join(", "));
        }
        println!(
            "   {} {} (score precision {})",
            "snippet lines:".bright_black(),
            entry.snippet_lines,
            entry.score_precision
        );
        println!("   {} {}", "timestamp:".bright_black(), entry.timestamp);
        println!();
    }
}

fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    // Try parsing as YYYY-MM-DD format first
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(naive_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?
            .and_utc());
    }

    // Try parsing as ISO 8601 / RFC3339 format
    chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| anyhow::anyhow!(
            "Invalid date format. Use YYYY-MM-DD or ISO 8601 (e.g., 2024-01-01 or 2024-01-01T00:00:00Z)"
        ))
}
