use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::utils::preferences::{SearchHistoryEntry, active_scope_key};
use crate::utils::store;

const HISTORY_FILENAME: &str = "history.jsonl";
const MAX_HISTORY_ENTRIES: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
struct HistoryRecord {
    scope: String,
    #[serde(flatten)]
    entry: SearchHistoryEntry,
}

pub fn append(entry: &SearchHistoryEntry) -> std::io::Result<()> {
    let scope = active_scope_key();
    let mut records = load_all();
    records.push(HistoryRecord {
        scope,
        entry: entry.clone(),
    });
    prune_records(&mut records);
    write_all(&records)
}

pub fn recent_for_active_scope(limit: usize) -> Vec<SearchHistoryEntry> {
    recent_for_scope(&active_scope_key(), limit)
}

pub fn recent_for_scope(scope: &str, limit: usize) -> Vec<SearchHistoryEntry> {
    let records = load_all();
    records
        .into_iter()
        .filter(|record| record.scope == scope)
        .map(|record| record.entry)
        .rev()
        .take(limit)
        .collect()
}

fn load_all() -> Vec<HistoryRecord> {
    let path = history_path();
    let file = match OpenOptions::new().read(true).open(&path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            warn!("failed to read history log at {}: {err}", path.display());
            return Vec::new();
        },
    };

    let reader = BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| match line {
            Ok(raw) if !raw.trim().is_empty() => {
                match serde_json::from_str::<HistoryRecord>(&raw) {
                    Ok(record) => Some(record),
                    Err(err) => {
                        warn!("failed to parse history record: {err}");
                        None
                    },
                }
            },
            _ => None,
        })
        .collect()
}

fn write_all(records: &[HistoryRecord]) -> std::io::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;
    for record in records {
        let line = serde_json::to_string(record).unwrap_or_else(|_| "{}".to_string());
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn prune_records(records: &mut Vec<HistoryRecord>) {
    let mut per_scope: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        per_scope.entry(record.scope.clone()).or_default().push(idx);
    }

    let mut indices_to_remove = Vec::new();
    for indices in per_scope.values() {
        if indices.len() > MAX_HISTORY_ENTRIES {
            indices_to_remove.extend_from_slice(&indices[..indices.len() - MAX_HISTORY_ENTRIES]);
        }
    }

    if indices_to_remove.is_empty() {
        return;
    }

    indices_to_remove.sort_unstable();
    indices_to_remove.dedup();
    for idx in indices_to_remove.into_iter().rev() {
        records.remove(idx);
    }
}

fn history_path() -> PathBuf {
    store::active_config_dir().join(HISTORY_FILENAME)
}

#[cfg(test)]
#[allow(unsafe_code, clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::utils::preferences::SearchHistoryEntry;
    use tempfile::tempdir;

    fn with_temp_history<F, R>(f: F) -> std::io::Result<R>
    where
        F: FnOnce() -> std::io::Result<R>,
    {
        let _guard = crate::utils::test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");
        let dir = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("BLZ_CONFIG_DIR", dir.path());
            std::env::remove_var("BLZ_CONFIG");
        }
        let result = f();
        unsafe {
            std::env::remove_var("BLZ_CONFIG_DIR");
        }
        result
    }

    fn sample_entry(query: &str) -> SearchHistoryEntry {
        SearchHistoryEntry {
            timestamp: "1970-01-01T00:00:00Z".to_string(),
            query: query.to_string(),
            alias: Some("alias".to_string()),
            format: "text".to_string(),
            show: vec![],
            snippet_lines: 3,
            score_precision: 1,
        }
    }

    #[test]
    fn append_writes_history_for_active_scope() -> std::io::Result<()> {
        with_temp_history(|| {
            let entry = sample_entry("first");
            append(&entry)?;

            let fetched = recent_for_active_scope(5);
            assert_eq!(fetched.len(), 1);
            assert_eq!(fetched[0].query, "first");
            Ok(())
        })
    }

    #[test]
    fn history_prunes_to_max_entries_per_scope() -> std::io::Result<()> {
        with_temp_history(|| {
            for idx in 0..60 {
                let entry = sample_entry(&format!("query-{idx}"));
                append(&entry)?;
            }

            let entries = recent_for_active_scope(100);
            assert_eq!(entries.len(), MAX_HISTORY_ENTRIES);
            assert_eq!(entries.first().unwrap().query, "query-59");
            assert_eq!(entries.last().unwrap().query, "query-10");
            Ok(())
        })
    }
}
