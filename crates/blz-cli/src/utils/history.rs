use anyhow::Result;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub ts: DateTime<Utc>,
    pub query: String,
    pub source: Option<String>,
    pub limit: usize,
    pub page: usize,
    pub top: Option<u8>,
    pub output_format: String,
    pub output_modifiers: Vec<String>,
    pub total_results: usize,
}

fn history_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "outfitter", "blz").map(|pd| pd.config_dir().join("history.json"))
}

pub fn append_history_entry(entry: &SearchHistoryEntry) -> Result<()> {
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let line = serde_json::to_string(entry)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

pub fn read_last_search() -> Result<Option<SearchHistoryEntry>> {
    if let Some(path) = history_path() {
        if !path.exists() {
            return Ok(None);
        }

        let file = OpenOptions::new().read(true).open(&path)?;
        let file_size = file.metadata()?.len();

        // For small files, read normally to avoid complexity
        if file_size <= 8192 {
            let reader = BufReader::new(file);
            let mut last: Option<String> = None;
            for l in reader.lines().map_while(Result::ok) {
                if !l.trim().is_empty() {
                    last = Some(l);
                }
            }
            if let Some(line) = last {
                let entry: SearchHistoryEntry = serde_json::from_str(&line)?;
                return Ok(Some(entry));
            }
        } else {
            // For larger files, read only the last 4KB to find the last entry
            use std::io::{Read, Seek, SeekFrom};
            let mut file = file;
            let read_size = std::cmp::min(4096, file_size);
            let seek_pos = file_size.saturating_sub(read_size);

            file.seek(SeekFrom::Start(seek_pos))?;
            let mut buffer = Vec::with_capacity(usize::try_from(read_size).unwrap_or(4096));
            file.read_to_end(&mut buffer)?;

            let content = String::from_utf8_lossy(&buffer);
            let mut last: Option<String> = None;

            for line in content.lines() {
                if !line.trim().is_empty() {
                    last = Some(line.to_string());
                }
            }

            if let Some(line) = last {
                let entry: SearchHistoryEntry = serde_json::from_str(&line)?;
                return Ok(Some(entry));
            }
        }
    }
    Ok(None)
}
