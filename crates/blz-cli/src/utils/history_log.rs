use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::utils::preferences::{SearchHistoryEntry, active_scope_key};
use crate::utils::store;

use fs2::FileExt;

const HISTORY_FILENAME: &str = "history.jsonl";
const MAX_HISTORY_ENTRIES: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
struct HistoryRecord {
    scope: String,
    #[serde(flatten)]
    entry: SearchHistoryEntry,
}

/// Append a search history entry to the scoped history log.
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

/// Return recent history entries for the active scope.
pub fn recent_for_active_scope(limit: usize) -> Vec<SearchHistoryEntry> {
    recent_for_scope(&active_scope_key(), limit)
}

/// Return recent history entries for a specific scope.
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
    let tmp_path = path.with_extension("jsonl.tmp");

    // Acquire a persistent exclusive lock alongside the history file.
    let lock_path = path.with_extension("lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock_exclusive()?; // keep this handle alive until after rename

    let tmp = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)?;
    // removed tmp.lock_exclusive(); using separate `lock` file instead

    // Use buffered writing for better performance while targeting a temp file first
    let mut buf = BufWriter::new(tmp);
    for record in records {
        serde_json::to_writer(&mut buf, record).map_err(std::io::Error::other)?;
        buf.write_all(b"\n")?;
    }
    buf.flush()?;
    let file = buf.into_inner()?;
    file.sync_all()?;
    drop(file);
    match fs::rename(&tmp_path, &path) {
        Ok(()) => {},
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(&path)?;
            fs::rename(&tmp_path, &path)?;
        },
        Err(err) => {
            // Clean up temp file on failure to avoid accumulating stale files.
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        },
    }
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = OpenOptions::new().read(true).open(parent) {
            let _ = dir.sync_all();
        }
    }
    // `lock` is dropped here when it goes out of scope, releasing the exclusive lock
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

/// Clear all search history
pub fn clear_all() -> std::io::Result<()> {
    let path = history_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Clear search history before a specific date
pub fn clear_before(cutoff: &chrono::DateTime<chrono::Utc>) -> std::io::Result<()> {
    let mut records = load_all();

    // Filter out records before the cutoff date
    records.retain(|record| {
        chrono::DateTime::parse_from_rfc3339(&record.entry.timestamp).map_or(true, |timestamp| {
            timestamp.with_timezone(&chrono::Utc) >= *cutoff
        })
    });

    write_all(&records)
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
        // SAFETY: history tests hold the env mutex to ensure exclusive env access.
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
            source: Some("alias".to_string()),
            format: "text".to_string(),
            show: vec![],
            snippet_lines: 3,
            score_precision: 1,
            page: Some(1),
            limit: Some(10),
            total_pages: Some(1),
            total_results: Some(5),
            headings_only: false,
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
