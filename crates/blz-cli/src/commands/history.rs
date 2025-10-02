use anyhow::Result;
use colored::Colorize;
use serde_json::json;

use crate::output::OutputFormat;
use crate::utils::history_log;
use crate::utils::preferences::{self, CliPreferences};

pub fn show(prefs: &CliPreferences, limit: usize, format: OutputFormat) -> Result<()> {
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
