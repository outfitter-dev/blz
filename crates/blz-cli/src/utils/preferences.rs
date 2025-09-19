use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::cli::ShowComponent;
use crate::output::OutputFormat;

const MAX_HISTORY_ENTRIES: usize = 50;
const PREFS_FILENAME: &str = "cli-preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub timestamp: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub format: String,
    pub show: Vec<String>,
    pub snippet_lines: u8,
    pub score_precision: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliPreferences {
    #[serde(default)]
    default_show: Vec<String>,
    #[serde(default = "default_precision")]
    default_score_precision: u8,
    #[serde(default = "default_snippet")]
    default_snippet_lines: u8,
    #[serde(default)]
    history: Vec<SearchHistoryEntry>,
}

fn default_precision() -> u8 {
    1
}

fn default_snippet() -> u8 {
    3
}

impl Default for CliPreferences {
    fn default() -> Self {
        Self {
            default_show: Vec::new(),
            default_score_precision: default_precision(),
            default_snippet_lines: default_snippet(),
            history: Vec::new(),
        }
    }
}

impl CliPreferences {
    pub fn default_show_components(&self) -> Vec<ShowComponent> {
        self.default_show
            .iter()
            .filter_map(|s| component_from_str(s))
            .collect()
    }

    pub fn set_default_show(&mut self, components: &[ShowComponent]) {
        self.default_show = components_to_strings(components);
    }

    pub fn default_score_precision(&self) -> u8 {
        clamp_precision(self.default_score_precision)
    }

    pub fn set_default_score_precision(&mut self, precision: u8) {
        self.default_score_precision = clamp_precision(precision);
    }

    pub fn default_snippet_lines(&self) -> u8 {
        clamp_snippet(self.default_snippet_lines)
    }

    pub fn set_default_snippet_lines(&mut self, lines: u8) {
        self.default_snippet_lines = clamp_snippet(lines);
    }

    pub fn record_history(&mut self, mut entry: SearchHistoryEntry) {
        entry.snippet_lines = clamp_snippet(entry.snippet_lines);
        entry.score_precision = clamp_precision(entry.score_precision);
        self.history.push(entry);
        if self.history.len() > MAX_HISTORY_ENTRIES {
            let excess = self.history.len() - MAX_HISTORY_ENTRIES;
            self.history.drain(0..excess);
        }
    }

    pub fn history(&self) -> &[SearchHistoryEntry] {
        &self.history
    }
}

pub fn load() -> CliPreferences {
    if let Some(path) = preferences_path() {
        if let Ok(data) = fs::read(&path) {
            match serde_json::from_slice::<CliPreferences>(&data) {
                Ok(mut prefs) => {
                    // sanitize
                    prefs.default_score_precision = clamp_precision(prefs.default_score_precision);
                    prefs.default_snippet_lines = clamp_snippet(prefs.default_snippet_lines);
                    prefs.history = prefs
                        .history
                        .into_iter()
                        .map(|mut entry| {
                            entry.snippet_lines = clamp_snippet(entry.snippet_lines);
                            entry.score_precision = clamp_precision(entry.score_precision);
                            entry
                        })
                        .collect();
                    return prefs;
                },
                Err(err) => warn!("failed to parse CLI preferences: {err}"),
            }
        }
    }
    CliPreferences::default()
}

pub fn save(prefs: &CliPreferences) -> std::io::Result<()> {
    if let Some(path) = preferences_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(prefs).unwrap_or_else(|_| "{}".to_string());
        fs::write(path, json)?;
    }
    Ok(())
}

pub fn preferences_path() -> Option<PathBuf> {
    if let Ok(dir) = env::var("BLZ_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Some(Path::new(trimmed).join(PREFS_FILENAME));
        }
    }

    if let Ok(file) = env::var("BLZ_CONFIG") {
        let path = PathBuf::from(file);
        if let Some(parent) = path.parent() {
            return Some(parent.join(PREFS_FILENAME));
        }
    }

    ProjectDirs::from("dev", "outfitter", "blz").map(|dirs| dirs.config_dir().join(PREFS_FILENAME))
}

pub fn components_to_strings(components: &[ShowComponent]) -> Vec<String> {
    components
        .iter()
        .map(component_to_str)
        .map(str::to_string)
        .collect()
}

pub fn component_to_str(component: &ShowComponent) -> &'static str {
    match component {
        ShowComponent::Url => "url",
        ShowComponent::Lines => "lines",
        ShowComponent::Anchor => "anchor",
        ShowComponent::Rank => "rank",
    }
}

pub fn component_from_str(s: &str) -> Option<ShowComponent> {
    match s.to_ascii_lowercase().as_str() {
        "url" => Some(ShowComponent::Url),
        "lines" => Some(ShowComponent::Lines),
        "anchor" => Some(ShowComponent::Anchor),
        "rank" => Some(ShowComponent::Rank),
        _ => None,
    }
}

pub fn parse_show_list(raw: &str) -> Vec<ShowComponent> {
    raw.split(',')
        .flat_map(|entry| entry.split_whitespace())
        .filter_map(|token| {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                None
            } else {
                component_from_str(trimmed)
            }
        })
        .collect()
}

pub fn format_show_components(components: &[ShowComponent]) -> String {
    components
        .iter()
        .map(component_to_str)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn make_history_entry(
    query: &str,
    alias: Option<&str>,
    format: OutputFormat,
    show: &[ShowComponent],
    snippet_lines: u8,
    score_precision: u8,
) -> SearchHistoryEntry {
    let timestamp = Utc::now().to_rfc3339();
    SearchHistoryEntry {
        timestamp,
        query: query.to_string(),
        alias: alias.map(|a| a.to_string()),
        format: format_to_string(format),
        show: components_to_strings(show),
        snippet_lines: clamp_snippet(snippet_lines),
        score_precision: clamp_precision(score_precision),
    }
}

pub fn format_to_string(format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => "text".to_string(),
        OutputFormat::Json => "json".to_string(),
        OutputFormat::Jsonl => "jsonl".to_string(),
    }
}

fn clamp_snippet(value: u8) -> u8 {
    value.clamp(1, 10)
}

fn clamp_precision(value: u8) -> u8 {
    value.min(4)
}

pub fn collect_show_components(url: bool, lines: bool, anchor: bool) -> Vec<ShowComponent> {
    let mut components = Vec::new();
    if url {
        components.push(ShowComponent::Url);
    }
    if lines {
        components.push(ShowComponent::Lines);
    }
    if anchor {
        components.push(ShowComponent::Anchor);
    }
    components
}
