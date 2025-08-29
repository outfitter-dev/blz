use crate::{Error, LlmsJson, Result, Source};
use chrono::Utc;
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub struct Storage {
    root_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("dev", "outfitter", "blz")
            .ok_or_else(|| Error::Storage("Failed to determine project directories".into()))?;

        let root_dir = project_dirs.data_dir().to_path_buf();

        // Check for migration from old cache directory
        Self::check_and_migrate_old_cache(&root_dir)?;

        Self::with_root(root_dir)
    }

    pub fn with_root(root_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root_dir)
            .map_err(|e| Error::Storage(format!("Failed to create root directory: {e}")))?;

        Ok(Self { root_dir })
    }

    pub fn tool_dir(&self, alias: &str) -> Result<PathBuf> {
        // Validate alias to prevent directory traversal attacks
        Self::validate_alias(alias)?;
        Ok(self.root_dir.join(alias))
    }

    pub fn ensure_tool_dir(&self, alias: &str) -> Result<PathBuf> {
        let dir = self.tool_dir(alias)?;
        fs::create_dir_all(&dir)
            .map_err(|e| Error::Storage(format!("Failed to create tool directory: {e}")))?;
        Ok(dir)
    }

    /// Validate that an alias is safe to use as a directory name
    fn validate_alias(alias: &str) -> Result<()> {
        // Check for empty alias
        if alias.is_empty() {
            return Err(Error::Storage("Alias cannot be empty".into()));
        }

        // Check for path traversal attempts
        if alias.contains("..") || alias.contains('/') || alias.contains('\\') {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': contains path traversal characters"
            )));
        }

        // Check for special filesystem characters
        if alias.starts_with('.') || alias.contains('\0') {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': contains invalid filesystem characters"
            )));
        }

        // Check for reserved names on Windows
        #[cfg(target_os = "windows")]
        {
            const RESERVED_NAMES: &[&str] = &[
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ];

            let upper_alias = alias.to_uppercase();
            if RESERVED_NAMES.contains(&upper_alias.as_str()) {
                return Err(Error::Storage(format!(
                    "Invalid alias '{}': reserved name on Windows",
                    alias
                )));
            }
        }

        // Check length (reasonable limit for filesystem compatibility)
        if alias.len() > 255 {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': exceeds maximum length of 255 characters"
            )));
        }

        // Only allow alphanumeric, dash, underscore for safety
        if !alias
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': only alphanumeric characters, dashes, and underscores are allowed"
            )));
        }

        Ok(())
    }

    pub fn llms_txt_path(&self, alias: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(alias)?.join("llms.txt"))
    }

    pub fn llms_json_path(&self, alias: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(alias)?.join("llms.json"))
    }

    pub fn index_dir(&self, alias: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(alias)?.join(".index"))
    }

    pub fn archive_dir(&self, alias: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(alias)?.join(".archive"))
    }

    pub fn metadata_path(&self, alias: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(alias)?.join("metadata.json"))
    }

    pub fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()> {
        self.ensure_tool_dir(alias)?;
        let path = self.llms_txt_path(alias)?;

        // Write to temporary file first for atomic operation
        let tmp_path = path.with_extension("txt.tmp");
        fs::write(&tmp_path, content)
            .map_err(|e| Error::Storage(format!("Failed to write llms.txt: {e}")))?;

        // Atomically rename temp file to final location
        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit llms.txt: {e}")))?;

        debug!("Saved llms.txt for {}", alias);
        Ok(())
    }

    pub fn load_llms_txt(&self, alias: &str) -> Result<String> {
        let path = self.llms_txt_path(alias)?;
        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.txt: {e}")))
    }

    pub fn save_llms_json(&self, alias: &str, data: &LlmsJson) -> Result<()> {
        self.ensure_tool_dir(alias)?;
        let path = self.llms_json_path(alias)?;
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| Error::Storage(format!("Failed to serialize JSON: {e}")))?;

        // Write to temporary file first for atomic operation
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|e| Error::Storage(format!("Failed to write llms.json: {e}")))?;

        // Atomically rename temp file to final location
        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit llms.json: {e}")))?;

        debug!("Saved llms.json for {}", alias);
        Ok(())
    }

    pub fn load_llms_json(&self, alias: &str) -> Result<LlmsJson> {
        let path = self.llms_json_path(alias)?;
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.json: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse JSON: {e}")))
    }

    pub fn save_source_metadata(&self, alias: &str, source: &Source) -> Result<()> {
        self.ensure_tool_dir(alias)?;
        let path = self.metadata_path(alias)?;
        let json = serde_json::to_string_pretty(source)
            .map_err(|e| Error::Storage(format!("Failed to serialize metadata: {e}")))?;

        // Write to a temp file first to ensure atomicity
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| Error::Storage(format!("Failed to write temp metadata: {e}")))?;

        // Atomically rename temp file to final path
        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to persist metadata: {e}")))?;

        debug!("Saved metadata for {}", alias);
        Ok(())
    }

    pub fn load_source_metadata(&self, alias: &str) -> Result<Option<Source>> {
        let path = self.metadata_path(alias)?;
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read metadata: {e}")))?;
        let source = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse metadata: {e}")))?;
        Ok(Some(source))
    }

    pub fn exists(&self, alias: &str) -> bool {
        self.llms_json_path(alias)
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    pub fn list_sources(&self) -> Vec<String> {
        let mut sources = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if !name.starts_with('.') && self.exists(name) {
                            sources.push(name.to_string());
                        }
                    }
                }
            }
        }

        sources.sort();
        sources
    }

    pub fn archive(&self, alias: &str) -> Result<()> {
        let archive_dir = self.archive_dir(alias)?;
        fs::create_dir_all(&archive_dir)
            .map_err(|e| Error::Storage(format!("Failed to create archive directory: {e}")))?;

        // Include seconds for uniqueness and clearer chronology
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");

        let llms_txt = self.llms_txt_path(alias)?;
        if llms_txt.exists() {
            let archive_path = archive_dir.join(format!("{timestamp}-llms.txt"));
            fs::copy(&llms_txt, &archive_path)
                .map_err(|e| Error::Storage(format!("Failed to archive llms.txt: {e}")))?;
        }

        let llms_json = self.llms_json_path(alias)?;
        if llms_json.exists() {
            let archive_path = archive_dir.join(format!("{timestamp}-llms.json"));
            fs::copy(&llms_json, &archive_path)
                .map_err(|e| Error::Storage(format!("Failed to archive llms.json: {e}")))?;
        }

        info!("Archived {} at {}", alias, timestamp);
        Ok(())
    }

    /// Check for old cache directory and migrate if needed
    fn check_and_migrate_old_cache(new_root: &Path) -> Result<()> {
        // Try to find the old cache directory
        let old_project_dirs = ProjectDirs::from("dev", "outfitter", "cache");

        if let Some(old_dirs) = old_project_dirs {
            let old_root = old_dirs.data_dir();

            // Check if old directory exists and has content
            if old_root.exists() && old_root.is_dir() {
                // Check if there's actually content to migrate (look for llms.json files)
                let has_content = fs::read_dir(old_root)
                    .map(|entries| {
                        entries
                            .filter_map(std::result::Result::ok)
                            .any(|entry| {
                                let path = entry.path();
                                if !path.is_dir() {
                                    return false;
                                }
                                let has_llms_json = path.join("llms.json").exists();
                                let has_llms_txt = path.join("llms.txt").exists();
                                let has_metadata = path.join("metadata.json").exists();
                                has_llms_json || has_llms_txt || has_metadata
                            })
                    })
                    .unwrap_or(false);
                if has_content {
                    // Check if new directory already exists with content
                    if new_root.exists()
                        && fs::read_dir(new_root)
                            .map(|mut e| e.next().is_some())
                            .unwrap_or(false)
                    {
                        // New directory already has content, just log a warning
                        warn!(
                            "Found old cache at {} but new cache at {} already exists. \
                             Manual migration may be needed if you want to preserve old data.",
                            old_root.display(),
                            new_root.display()
                        );
                    } else {
                        // Attempt migration
                        info!(
                            "Migrating cache from old location {} to new location {}",
                            old_root.display(),
                            new_root.display()
                        );

                        if let Err(e) = Self::migrate_directory(old_root, new_root) {
                            // Log warning but don't fail - let the user continue with fresh cache
                            warn!(
                                "Could not automatically migrate cache: {}. \
                                 Starting with fresh cache at {}. \
                                 To manually migrate, copy contents from {} to {}",
                                e,
                                new_root.display(),
                                old_root.display(),
                                new_root.display()
                            );
                        } else {
                            info!("Successfully migrated cache to new location");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Recursively copy directory contents from old to new location
    fn migrate_directory(from: &Path, to: &Path) -> Result<()> {
        // Create target directory if it doesn't exist
        fs::create_dir_all(to)
            .map_err(|e| Error::Storage(format!("Failed to create migration target: {e}")))?;

        // Copy all entries
        for entry in fs::read_dir(from)
            .map_err(|e| Error::Storage(format!("Failed to read migration source: {e}")))?
        {
            let entry = entry
                .map_err(|e| Error::Storage(format!("Failed to read directory entry: {e}")))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let target_path = to.join(&file_name);

            if path.is_dir() {
                // Recursively copy subdirectory
                Self::migrate_directory(&path, &target_path)?;
            } else {
                // Copy file
                fs::copy(&path, &target_path).map_err(|e| {
                    Error::Storage(format!("Failed to copy file during migration: {e}"))
                })?;
            }
        }

        Ok(())
    }
}

// Note: Default is not implemented as Storage::new() can fail.
// Use Storage::new() directly and handle the Result.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileInfo, LineIndex, Source, TocEntry};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = Storage::with_root(temp_dir.path().to_path_buf())
            .expect("Failed to create test storage");
        (storage, temp_dir)
    }

    fn create_test_llms_json(alias: &str) -> LlmsJson {
        LlmsJson {
            alias: alias.to_string(),
            source: Source {
                url: format!("https://example.com/{alias}/llms.txt"),
                etag: Some("abc123".to_string()),
                last_modified: None,
                fetched_at: Utc::now(),
                sha256: "deadbeef".to_string(),
            },
            toc: vec![TocEntry {
                heading_path: vec!["Getting Started".to_string()],
                lines: "1-50".to_string(),
                children: vec![],
            }],
            files: vec![FileInfo {
                path: "llms.txt".to_string(),
                sha256: "deadbeef".to_string(),
            }],
            line_index: LineIndex {
                total_lines: 100,
                byte_offsets: false,
            },
            diagnostics: vec![],
        }
    }

    #[test]
    fn test_storage_creation_with_root() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = Storage::with_root(temp_dir.path().to_path_buf());

        assert!(storage.is_ok());
        let _storage = storage.unwrap();

        // Verify root directory was created
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_tool_directory_paths() {
        let (storage, _temp_dir) = create_test_storage();

        let tool_dir = storage.tool_dir("react").expect("Should get tool dir");
        let llms_txt_path = storage
            .llms_txt_path("react")
            .expect("Should get llms.txt path");
        let llms_json_path = storage
            .llms_json_path("react")
            .expect("Should get llms.json path");
        let index_dir = storage.index_dir("react").expect("Should get index dir");
        let archive_dir = storage
            .archive_dir("react")
            .expect("Should get archive dir");

        assert!(tool_dir.ends_with("react"));
        assert!(llms_txt_path.ends_with("react/llms.txt"));
        assert!(llms_json_path.ends_with("react/llms.json"));
        assert!(index_dir.ends_with("react/.index"));
        assert!(archive_dir.ends_with("react/.archive"));
    }

    #[test]
    fn test_invalid_alias_validation() {
        let (storage, _temp_dir) = create_test_storage();

        // Test path traversal attempts
        assert!(storage.tool_dir("../etc").is_err());
        assert!(storage.tool_dir("../../passwd").is_err());
        assert!(storage.tool_dir("test/../../../etc").is_err());

        // Test invalid characters
        assert!(storage.tool_dir(".hidden").is_err());
        assert!(storage.tool_dir("test\0null").is_err());
        assert!(storage.tool_dir("test/slash").is_err());
        assert!(storage.tool_dir("test\\backslash").is_err());

        // Test empty alias
        assert!(storage.tool_dir("").is_err());

        // Test valid aliases
        assert!(storage.tool_dir("react").is_ok());
        assert!(storage.tool_dir("my-tool").is_ok());
        assert!(storage.tool_dir("tool_123").is_ok());
    }

    #[test]
    fn test_ensure_tool_directory() {
        let (storage, _temp_dir) = create_test_storage();

        let tool_dir = storage
            .ensure_tool_dir("react")
            .expect("Should create tool dir");
        assert!(tool_dir.exists());

        // Should be idempotent
        let tool_dir2 = storage
            .ensure_tool_dir("react")
            .expect("Should not fail on existing dir");
        assert_eq!(tool_dir, tool_dir2);
    }

    #[test]
    fn test_save_and_load_llms_txt() {
        let (storage, _temp_dir) = create_test_storage();

        let content = "# React Documentation\n\nThis is the React documentation...";

        // Save content
        storage
            .save_llms_txt("react", content)
            .expect("Should save llms.txt");

        // Verify file exists
        assert!(
            storage
                .llms_txt_path("react")
                .expect("Should get path")
                .exists()
        );

        // Load content
        let loaded_content = storage
            .load_llms_txt("react")
            .expect("Should load llms.txt");
        assert_eq!(content, loaded_content);
    }

    #[test]
    fn test_save_and_load_llms_json() {
        let (storage, _temp_dir) = create_test_storage();

        let llms_json = create_test_llms_json("react");

        // Save JSON
        storage
            .save_llms_json("react", &llms_json)
            .expect("Should save llms.json");

        // Verify file exists
        assert!(
            storage
                .llms_json_path("react")
                .expect("Should get path")
                .exists()
        );

        // Load JSON
        let loaded_json = storage
            .load_llms_json("react")
            .expect("Should load llms.json");
        assert_eq!(llms_json.alias, loaded_json.alias);
        assert_eq!(llms_json.source.url, loaded_json.source.url);
        assert_eq!(
            llms_json.line_index.total_lines,
            loaded_json.line_index.total_lines
        );
    }

    #[test]
    fn test_source_exists() {
        let (storage, _temp_dir) = create_test_storage();

        // Initially should not exist
        assert!(!storage.exists("react"));

        // After saving llms.json, should exist
        let llms_json = create_test_llms_json("react");
        storage
            .save_llms_json("react", &llms_json)
            .expect("Should save");

        assert!(storage.exists("react"));
    }

    #[test]
    fn test_list_sources_empty() {
        let (storage, _temp_dir) = create_test_storage();

        let sources = storage.list_sources();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_list_sources_with_data() {
        let (storage, _temp_dir) = create_test_storage();

        // Add multiple sources
        let aliases = ["react", "nextjs", "rust"];
        for &alias in &aliases {
            let llms_json = create_test_llms_json(alias);
            storage
                .save_llms_json(alias, &llms_json)
                .expect("Should save");
        }

        let sources = storage.list_sources();
        assert_eq!(sources.len(), 3);

        // Should be sorted
        assert_eq!(sources, vec!["nextjs", "react", "rust"]);
    }

    #[test]
    fn test_list_sources_ignores_hidden_dirs() {
        let (storage, temp_dir) = create_test_storage();

        // Create a hidden directory
        let hidden_dir = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden_dir).expect("Should create hidden dir");

        // Create a regular source
        let llms_json = create_test_llms_json("react");
        storage
            .save_llms_json("react", &llms_json)
            .expect("Should save");

        let sources = storage.list_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], "react");
    }

    #[test]
    fn test_list_sources_requires_llms_json() {
        let (storage, _temp_dir) = create_test_storage();

        // Create tool directory without llms.json
        storage
            .ensure_tool_dir("incomplete")
            .expect("Should create dir");

        // Save only llms.txt (no llms.json)
        storage
            .save_llms_txt("incomplete", "# Test content")
            .expect("Should save txt");

        // Create another source with complete data
        let llms_json = create_test_llms_json("complete");
        storage
            .save_llms_json("complete", &llms_json)
            .expect("Should save json");

        let sources = storage.list_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], "complete");
    }

    #[test]
    fn test_archive_functionality() {
        let (storage, _temp_dir) = create_test_storage();

        // Create source data
        let content = "# Test content";
        let llms_json = create_test_llms_json("test");

        storage
            .save_llms_txt("test", content)
            .expect("Should save txt");
        storage
            .save_llms_json("test", &llms_json)
            .expect("Should save json");

        // Archive the source
        storage.archive("test").expect("Should archive");

        // Verify archive directory exists
        let archive_dir = storage.archive_dir("test").expect("Should get archive dir");
        assert!(archive_dir.exists());

        // Verify archived files exist (names contain timestamp)
        let archive_entries: Vec<_> = fs::read_dir(&archive_dir)
            .expect("Should read archive dir")
            .collect::<std::result::Result<Vec<_>, std::io::Error>>()
            .expect("Should collect entries");

        assert_eq!(archive_entries.len(), 2); // llms.txt and llms.json

        // Verify archived files have correct names
        let mut has_txt = false;
        let mut has_json = false;
        for entry in archive_entries {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains("llms.txt") {
                has_txt = true;
            }
            if name.contains("llms.json") {
                has_json = true;
            }
        }

        assert!(has_txt, "Should have archived llms.txt");
        assert!(has_json, "Should have archived llms.json");
    }

    #[test]
    fn test_archive_missing_files() {
        let (storage, _temp_dir) = create_test_storage();

        // Archive non-existent source - should not fail
        let result = storage.archive("nonexistent");
        assert!(result.is_ok());

        // Archive directory should still be created
        let archive_dir = storage
            .archive_dir("nonexistent")
            .expect("Should get archive dir");
        assert!(archive_dir.exists());
    }

    #[test]
    fn test_load_missing_files_returns_error() {
        let (storage, _temp_dir) = create_test_storage();

        let result = storage.load_llms_txt("nonexistent");
        assert!(result.is_err());

        let result = storage.load_llms_json("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let (storage, _temp_dir) = create_test_storage();

        let original = create_test_llms_json("test");

        // Save and load
        storage
            .save_llms_json("test", &original)
            .expect("Should save");
        let loaded = storage.load_llms_json("test").expect("Should load");

        // Verify all fields are preserved
        assert_eq!(original.alias, loaded.alias);
        assert_eq!(original.source.url, loaded.source.url);
        assert_eq!(original.source.sha256, loaded.source.sha256);
        assert_eq!(original.toc.len(), loaded.toc.len());
        assert_eq!(original.files.len(), loaded.files.len());
        assert_eq!(
            original.line_index.total_lines,
            loaded.line_index.total_lines
        );
        assert_eq!(original.diagnostics.len(), loaded.diagnostics.len());
    }
}
