use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::cli::ShowComponent;
use crate::output::OutputFormat;
use crate::utils::store::{self, BlzStore};
use chrono::Utc;

const GLOBAL_SCOPE_KEY: &str = "global";

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliPreferences {
    #[serde(default)]
    default_show: Vec<String>,
    #[serde(default = "default_precision")]
    default_score_precision: u8,
    #[serde(default = "default_snippet")]
    default_snippet_lines: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub timestamp: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub format: String,
    pub show: Vec<String>,
    pub snippet_lines: u8,
    pub score_precision: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_results: Option<usize>,
}

const fn default_precision() -> u8 {
    1
}

const fn default_snippet() -> u8 {
    3
}

impl Default for CliPreferences {
    fn default() -> Self {
        Self {
            default_show: Vec::new(),
            default_score_precision: default_precision(),
            default_snippet_lines: default_snippet(),
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
}

pub fn load() -> CliPreferences {
    let store = store::load_store();
    load_from_store(&store)
}

fn load_from_store(store: &BlzStore) -> CliPreferences {
    let mut prefs = CliPreferences::default();
    for key in scope_chain() {
        if let Some(record) = store.scopes.get(&key) {
            if !record.cli_preferences.is_null() {
                match serde_json::from_value::<CliPreferences>(record.cli_preferences.clone()) {
                    Ok(scope_prefs) => {
                        prefs = sanitize(scope_prefs);
                    },
                    Err(err) => {
                        warn!("failed to deserialize CLI preferences for scope {key}: {err}");
                    },
                }
            }
        }
    }
    prefs
}

fn sanitize(mut prefs: CliPreferences) -> CliPreferences {
    prefs.default_score_precision = clamp_precision(prefs.default_score_precision);
    prefs.default_snippet_lines = clamp_snippet(prefs.default_snippet_lines);
    prefs
}

pub fn save(prefs: &CliPreferences) -> std::io::Result<()> {
    let mut store = store::load_store();
    let key = active_scope_key();
    let record = store.scopes.entry(key).or_default();
    record.cli_preferences = serde_json::to_value(prefs).unwrap_or(serde_json::Value::Null);
    store::save_store(&store)
}

/// Builder for search history entries to avoid too many function parameters
pub struct HistoryEntryBuilder<'a> {
    query: &'a str,
    alias: Option<&'a str>,
    format: OutputFormat,
    show: &'a [ShowComponent],
    snippet_lines: u8,
    score_precision: u8,
    pagination: PaginationInfo,
}

/// Pagination information for search history
#[derive(Debug, Clone, Copy, Default)]
pub struct PaginationInfo {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub total_pages: Option<usize>,
    pub total_results: Option<usize>,
}

impl<'a> HistoryEntryBuilder<'a> {
    pub fn new(
        query: &'a str,
        alias: Option<&'a str>,
        format: OutputFormat,
        show: &'a [ShowComponent],
    ) -> Self {
        Self {
            query,
            alias,
            format,
            show,
            snippet_lines: default_snippet(),
            score_precision: default_precision(),
            pagination: PaginationInfo::default(),
        }
    }

    pub const fn with_snippet_lines(mut self, lines: u8) -> Self {
        self.snippet_lines = lines;
        self
    }

    pub const fn with_score_precision(mut self, precision: u8) -> Self {
        self.score_precision = precision;
        self
    }

    pub const fn with_pagination(mut self, pagination: PaginationInfo) -> Self {
        self.pagination = pagination;
        self
    }

    pub fn build(self) -> SearchHistoryEntry {
        let timestamp = Utc::now().to_rfc3339();
        SearchHistoryEntry {
            timestamp,
            query: self.query.to_string(),
            source: self.alias.map(std::string::ToString::to_string),
            format: format_to_string(self.format),
            show: components_to_strings(self.show),
            snippet_lines: clamp_snippet(self.snippet_lines),
            score_precision: clamp_precision(self.score_precision),
            page: self.pagination.page,
            limit: self.pagination.limit,
            total_pages: self.pagination.total_pages,
            total_results: self.pagination.total_results,
        }
    }
}

pub fn format_to_string(format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => "text".to_string(),
        OutputFormat::Json => "json".to_string(),
        OutputFormat::Jsonl => "jsonl".to_string(),
    }
}

pub fn components_to_strings(components: &[ShowComponent]) -> Vec<String> {
    components
        .iter()
        .map(|component| component_to_str(*component))
        .map(str::to_string)
        .collect()
}

pub const fn component_to_str(component: ShowComponent) -> &'static str {
    match component {
        ShowComponent::Url => "url",
        ShowComponent::Lines => "lines",
        ShowComponent::Anchor => "anchor",
        ShowComponent::Rank => "rank",
        ShowComponent::RawScore => "raw-score",
    }
}

pub fn component_from_str(s: &str) -> Option<ShowComponent> {
    match s.to_ascii_lowercase().as_str() {
        "url" => Some(ShowComponent::Url),
        "lines" => Some(ShowComponent::Lines),
        "anchor" => Some(ShowComponent::Anchor),
        "rank" => Some(ShowComponent::Rank),
        "raw-score" => Some(ShowComponent::RawScore),
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
        .map(|component| component_to_str(*component))
        .collect::<Vec<_>>()
        .join(", ")
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

#[allow(clippy::fn_params_excessive_bools)]
pub fn collect_show_components_extended(
    url: bool,
    lines: bool,
    anchor: bool,
    raw_score: bool,
) -> Vec<ShowComponent> {
    let mut components = collect_show_components(url, lines, anchor);
    if raw_score {
        components.push(ShowComponent::RawScore);
    }
    components
}

pub fn scope_chain() -> Vec<String> {
    let mut chain = vec![GLOBAL_SCOPE_KEY.to_string()];
    if let Some(project) = project_scope_key() {
        chain.push(project);
    }
    if let Some(local) = local_scope_key() {
        chain.push(local);
    }
    chain
}

pub fn active_scope_key() -> String {
    scope_chain()
        .into_iter()
        .last()
        .unwrap_or_else(|| GLOBAL_SCOPE_KEY.to_string())
}

pub fn project_scope_key() -> Option<String> {
    if let Ok(file) = env::var("BLZ_CONFIG") {
        let trimmed = file.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if let Some(parent) = path.parent() {
                return Some(format!("project:{}", canonicalize_path(parent)));
            }
        }
    }

    if let Ok(dir) = env::var("BLZ_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Some(format!("project:{}", canonicalize(trimmed)));
        }
    }

    None
}

pub fn local_scope_key() -> Option<String> {
    env::current_dir()
        .ok()
        .map(|dir| format!("local:{}", canonicalize_path(&dir)))
}

pub fn local_scope_path() -> Option<PathBuf> {
    env::current_dir()
        .ok()
        .map(|dir| PathBuf::from(canonicalize_path(&dir)))
}

fn canonicalize(value: &str) -> String {
    canonicalize_path(Path::new(value))
}

fn canonicalize_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn clamp_snippet(value: u8) -> u8 {
    value.clamp(1, 10)
}

fn clamp_precision(value: u8) -> u8 {
    value.min(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_entry_builder_populates_pagination_fields() {
        let entry = HistoryEntryBuilder::new(
            "search term",
            Some("docs"),
            OutputFormat::Text,
            &[ShowComponent::Url, ShowComponent::Rank],
        )
        .with_snippet_lines(12)
        .with_score_precision(6)
        .with_pagination(PaginationInfo {
            page: Some(3),
            limit: Some(25),
            total_pages: Some(5),
            total_results: Some(250),
        })
        .build();

        assert_eq!(entry.page, Some(3));
        assert_eq!(entry.limit, Some(25));
        assert_eq!(entry.total_pages, Some(5));
        assert_eq!(entry.total_results, Some(250));
        // Clamp behaviour remains in effect for existing fields
        assert_eq!(entry.snippet_lines, 10);
        assert_eq!(entry.score_precision, 4);
    }

    #[test]
    fn collect_show_components_extended_includes_raw_score() {
        let components = collect_show_components_extended(true, false, false, true);
        assert!(components.contains(&ShowComponent::Url));
        assert!(components.contains(&ShowComponent::RawScore));
        assert!(!components.contains(&ShowComponent::Lines));
        assert!(!components.contains(&ShowComponent::Anchor));
    }

    #[test]
    fn raw_score_round_trip_serialisation() {
        let components = vec![ShowComponent::RawScore];
        let strings = components_to_strings(&components);
        assert_eq!(strings, vec!["raw-score".to_string()]);

        let parsed = strings
            .iter()
            .filter_map(|s| component_from_str(s))
            .collect::<Vec<_>>();
        assert_eq!(parsed, components);
    }
}
