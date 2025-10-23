use crate::{Error, LlmsJson, Result, Source, SourceDescriptor, profile};
use chrono::Utc;
use directories::{BaseDirs, ProjectDirs};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Maximum allowed alias length to match CLI constraints
const MAX_ALIAS_LEN: usize = 64;

/// Local filesystem storage for cached llms.txt documentation
pub struct Storage {
    root_dir: PathBuf,
    config_dir: PathBuf,
}

impl Storage {
    fn sanitize_variant_file_name(name: &str) -> String {
        // Only allow a conservative set of filename characters to avoid
        // accidentally writing outside the tool directory or producing
        // surprising paths. Anything else becomes an underscore so that the
        // resulting filename stays predictable and safe to use across
        // platforms.
        let mut sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        // Collapse any ".." segments that could be introduced either by the
        // caller or by the substitution above. This keeps the path rooted at
        // the alias directory even if callers pass traversal attempts.
        while sanitized.contains("..") {
            sanitized = sanitized.replace("..", "_");
        }

        if sanitized.is_empty() {
            "llms.txt".to_string()
        } else {
            sanitized
        }
    }

    // Storage uses consistent filenames regardless of source URL:
    // - llms.txt for content (even if fetched from llms-full.txt)
    // - llms.json for parsed data
    // - metadata.json for source metadata

    /// Creates a new storage instance with the default root directory
    pub fn new() -> Result<Self> {
        // Test/dev override: allow BLZ_DATA_DIR to set the root directory explicitly
        if let Ok(dir) = std::env::var("BLZ_DATA_DIR") {
            let root = PathBuf::from(dir);
            let config_dir = Self::default_config_dir()?;
            return Self::with_paths(root, config_dir);
        }

        // Use XDG_DATA_HOME if explicitly set
        let root_dir = if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            let trimmed = xdg.trim();
            if trimmed.is_empty() {
                Self::fallback_data_dir()?
            } else {
                PathBuf::from(trimmed).join(profile::app_dir_slug())
            }
        } else {
            Self::fallback_data_dir()?
        };

        // Check for migration from old cache directory
        Self::check_and_migrate_old_cache(&root_dir);

        let config_dir = Self::default_config_dir()?;
        Self::with_paths(root_dir, config_dir)
    }

    /// Fallback data directory when `XDG_DATA_HOME` is not set
    fn fallback_data_dir() -> Result<PathBuf> {
        // Use ~/.blz/ for data (same location as config for non-XDG systems)
        let home = directories::BaseDirs::new()
            .ok_or_else(|| Error::Storage("Failed to determine home directory".into()))?;
        Ok(home.home_dir().join(profile::dot_dir_slug()))
    }

    /// Determine the default configuration directory honoring overrides
    fn default_config_dir() -> Result<PathBuf> {
        if let Ok(dir) = std::env::var("BLZ_CONFIG_DIR") {
            let trimmed = dir.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }

        if let Ok(dir) = std::env::var("BLZ_GLOBAL_CONFIG_DIR") {
            let trimmed = dir.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed).join(profile::app_dir_slug()));
            }
        }

        if let Some(base_dirs) = BaseDirs::new() {
            return Ok(base_dirs.home_dir().join(profile::dot_dir_slug()));
        }

        Err(Error::Storage(
            "Failed to determine configuration directory".into(),
        ))
    }

    /// Creates a new storage instance with a custom root directory
    pub fn with_root(root_dir: PathBuf) -> Result<Self> {
        let config_dir = root_dir.join("config");
        Self::with_paths(root_dir, config_dir)
    }

    /// Creates a new storage instance with explicit data and config directories
    pub fn with_paths(root_dir: PathBuf, config_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root_dir)
            .map_err(|e| Error::Storage(format!("Failed to create root directory: {e}")))?;
        fs::create_dir_all(&config_dir)
            .map_err(|e| Error::Storage(format!("Failed to create config directory: {e}")))?;

        Ok(Self {
            root_dir,
            config_dir,
        })
    }

    /// Returns the root data directory path
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns the root configuration directory path used for descriptors
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    fn descriptors_dir(&self) -> PathBuf {
        self.config_dir.join("sources")
    }

    /// Returns the path to the descriptor TOML for a source
    pub fn descriptor_path(&self, alias: &str) -> Result<PathBuf> {
        Self::validate_alias(alias)?;
        Ok(self.descriptors_dir().join(format!("{alias}.toml")))
    }

    /// Persist a descriptor to disk, creating parent directories if necessary
    pub fn save_descriptor(&self, descriptor: &SourceDescriptor) -> Result<()> {
        let path = self.descriptor_path(&descriptor.alias)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Storage(format!("Failed to create descriptor dir: {e}")))?;
        }

        let toml = toml::to_string_pretty(descriptor)
            .map_err(|e| Error::Storage(format!("Failed to serialize descriptor: {e}")))?;
        fs::write(&path, toml)
            .map_err(|e| Error::Storage(format!("Failed to write descriptor: {e}")))?;
        Ok(())
    }

    /// Load a descriptor if it exists
    pub fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>> {
        let path = self.descriptor_path(alias)?;
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read descriptor: {e}")))?;
        let descriptor = toml::from_str::<SourceDescriptor>(&contents)
            .map_err(|e| Error::Storage(format!("Failed to parse descriptor: {e}")))?;
        Ok(Some(descriptor))
    }

    /// Remove descriptor file for an alias if present
    pub fn remove_descriptor(&self, alias: &str) -> Result<()> {
        let path = self.descriptor_path(alias)?;
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to remove descriptor: {e}")))?;
        }
        Ok(())
    }

    /// Returns the directory path for a given alias
    pub fn tool_dir(&self, source: &str) -> Result<PathBuf> {
        // Validate alias to prevent directory traversal attacks
        Self::validate_alias(source)?;
        Ok(self.root_dir.join("sources").join(source))
    }

    /// Resolve the on-disk path for a specific flavored content file.
    fn variant_file_path(&self, source: &str, file_name: &str) -> Result<PathBuf> {
        let sanitized = Self::sanitize_variant_file_name(file_name);
        Ok(self.tool_dir(source)?.join(sanitized))
    }

    /// Ensures the directory for an alias exists and returns its path
    pub fn ensure_tool_dir(&self, source: &str) -> Result<PathBuf> {
        let dir = self.tool_dir(source)?;
        fs::create_dir_all(&dir)
            .map_err(|e| Error::Storage(format!("Failed to create tool directory: {e}")))?;
        Ok(dir)
    }

    /// Validate that an alias is safe to use as a directory name
    ///
    /// This validation is unified with CLI constraints to prevent inconsistencies
    /// between what the CLI accepts and what storage can handle.
    fn validate_alias(alias: &str) -> Result<()> {
        // Check for empty alias
        if alias.is_empty() {
            return Err(Error::Storage("Alias cannot be empty".into()));
        }

        // Disallow leading hyphen to avoid CLI parsing ambiguities
        if alias.starts_with('-') {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': cannot start with '-'"
            )));
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

        // Check length (keep consistent with CLI policy)
        if alias.len() > MAX_ALIAS_LEN {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': exceeds maximum length of {MAX_ALIAS_LEN} characters"
            )));
        }

        // Only allow ASCII alphanumeric, dash, underscore
        if !alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Storage(format!(
                "Invalid alias '{alias}': only [A-Za-z0-9_-] are allowed"
            )));
        }

        Ok(())
    }

    /// Returns the path to the llms.txt file for a source
    pub fn llms_txt_path(&self, source: &str) -> Result<PathBuf> {
        self.variant_file_path(source, "llms.txt")
    }

    /// Returns the path to the llms.json file for a source
    pub fn llms_json_path(&self, source: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(source)?.join("llms.json"))
    }

    /// Returns the path to the search index directory for a source
    pub fn index_dir(&self, source: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(source)?.join(".index"))
    }

    /// Returns the path to the archive directory for a source
    pub fn archive_dir(&self, source: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(source)?.join(".archive"))
    }

    /// Returns the path to the metadata file for a source
    pub fn metadata_path(&self, source: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(source)?.join("metadata.json"))
    }

    /// Returns the path to the anchors mapping file for a source
    pub fn anchors_map_path(&self, source: &str) -> Result<PathBuf> {
        Ok(self.tool_dir(source)?.join("anchors.json"))
    }

    /// Saves the llms.txt content for a source
    pub fn save_llms_txt(&self, source: &str, content: &str) -> Result<()> {
        self.ensure_tool_dir(source)?;
        let path = self.llms_txt_path(source)?;

        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, content)
            .map_err(|e| Error::Storage(format!("Failed to write llms.txt: {e}")))?;

        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to remove existing llms.txt: {e}")))?;
        }

        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit llms.txt: {e}")))?;

        debug!("Saved llms.txt for {}", source);
        Ok(())
    }

    /// Loads the llms.txt content for a source
    pub fn load_llms_txt(&self, source: &str) -> Result<String> {
        let path = self.llms_txt_path(source)?;
        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.txt: {e}")))
    }

    /// Saves the parsed llms.json data for a source
    pub fn save_llms_json(&self, source: &str, data: &LlmsJson) -> Result<()> {
        self.ensure_tool_dir(source)?;
        let path = self.llms_json_path(source)?;
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| Error::Storage(format!("Failed to serialize JSON: {e}")))?;

        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|e| Error::Storage(format!("Failed to write llms.json: {e}")))?;

        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to remove existing llms.json: {e}")))?;
        }
        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit llms.json: {e}")))?;

        debug!("Saved llms.json for {}", source);
        Ok(())
    }

    /// Loads the parsed llms.json data for a source
    pub fn load_llms_json(&self, source: &str) -> Result<LlmsJson> {
        let path = self.llms_json_path(source)?;
        if !path.exists() {
            return Err(Error::Storage(format!(
                "llms.json missing for source '{source}'"
            )));
        }
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.json: {e}")))?;

        // Try to detect old v0.4.x format
        if let Ok(raw_value) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(obj) = raw_value.as_object() {
                // Old format has "alias" field instead of "source"
                if obj.contains_key("alias")
                    || (obj.contains_key("source") && obj["source"].is_object())
                {
                    return Err(Error::Storage(format!(
                        "Incompatible cache format detected for source '{source}'.\n\n\
                         This cache was created with blz v0.4.x or earlier and is not compatible with v0.5.0+.\n\n\
                         To fix this, clear your cache:\n  \
                         blz clear --force\n\n\
                         Then re-add your sources."
                    )));
                }
            }
        }

        let data = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse llms.json: {e}")))?;
        Ok(data)
    }

    /// Saves source metadata for a source
    pub fn save_source_metadata(&self, source: &str, metadata: &Source) -> Result<()> {
        self.ensure_tool_dir(source)?;
        let path = self.metadata_path(source)?;
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| Error::Storage(format!("Failed to serialize metadata: {e}")))?;

        // Write to a temp file first to ensure atomicity
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| Error::Storage(format!("Failed to write temp metadata: {e}")))?;

        // Atomically rename temp file to final path (handle Windows overwrite)
        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to remove existing metadata: {e}")))?;
        }
        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to persist metadata: {e}")))?;

        debug!("Saved metadata for {}", source);
        Ok(())
    }

    /// Save anchors remap JSON for a source
    pub fn save_anchors_map(&self, source: &str, map: &crate::AnchorsMap) -> Result<()> {
        self.ensure_tool_dir(source)?;
        let path = self.anchors_map_path(source)?;
        let json = serde_json::to_string_pretty(map)
            .map_err(|e| Error::Storage(format!("Failed to serialize anchors map: {e}")))?;
        fs::write(&path, json)
            .map_err(|e| Error::Storage(format!("Failed to write anchors map: {e}")))?;
        Ok(())
    }

    /// Loads source metadata for a source if it exists
    pub fn load_source_metadata(&self, source: &str) -> Result<Option<Source>> {
        let path = self.metadata_path(source)?;
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read metadata: {e}")))?;
        let metadata = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse metadata: {e}")))?;
        Ok(Some(metadata))
    }

    /// Checks if a source exists in storage
    #[must_use]
    pub fn exists(&self, source: &str) -> bool {
        self.llms_json_path(source)
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    /// Lists all cached sources
    #[must_use]
    pub fn list_sources(&self) -> Vec<String> {
        let mut sources = Vec::new();
        let sources_dir = self.root_dir.join("sources");

        if let Ok(entries) = fs::read_dir(&sources_dir) {
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

    /// Clears the entire cache directory, removing all sources and their data.
    ///
    /// This is a destructive operation that cannot be undone. Use with caution.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be removed or recreated.
    pub fn clear_cache(&self) -> Result<()> {
        // Remove the entire root directory
        if self.root_dir.exists() {
            fs::remove_dir_all(&self.root_dir)
                .map_err(|e| Error::Storage(format!("Failed to remove cache directory: {e}")))?;
        }

        // Recreate empty root directory
        fs::create_dir_all(&self.root_dir)
            .map_err(|e| Error::Storage(format!("Failed to recreate cache directory: {e}")))?;

        Ok(())
    }

    /// Archives the current version of a source
    pub fn archive(&self, source: &str) -> Result<()> {
        let archive_dir = self.archive_dir(source)?;
        fs::create_dir_all(&archive_dir)
            .map_err(|e| Error::Storage(format!("Failed to create archive directory: {e}")))?;

        // Include seconds for uniqueness and clearer chronology
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");

        // Archive all llms*.json and llms*.txt files
        let dir = self.tool_dir(source)?;
        if dir.exists() {
            for entry in fs::read_dir(&dir)
                .map_err(|e| Error::Storage(format!("Failed to read dir for archive: {e}")))?
            {
                let entry =
                    entry.map_err(|e| Error::Storage(format!("Failed to read entry: {e}")))?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = entry.file_name();
                let name_str = name.to_string_lossy().to_lowercase();
                // Archive only llms*.json / llms*.txt (skip metadata/anchors)
                let is_json = std::path::Path::new(&name_str)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
                let is_txt = std::path::Path::new(&name_str)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
                let is_llms_artifact = (is_json || is_txt) && name_str.starts_with("llms");
                if is_llms_artifact {
                    let archive_path =
                        archive_dir.join(format!("{timestamp}-{}", name.to_string_lossy()));
                    fs::copy(&path, &archive_path).map_err(|e| {
                        Error::Storage(format!("Failed to archive {}: {e}", path.display()))
                    })?;
                }
            }
        }

        info!("Archived {} at {}", source, timestamp);
        Ok(())
    }

    /// Check for old cache directory and migrate if needed
    fn check_and_migrate_old_cache(new_root: &Path) {
        // Try to find the old cache directory
        let old_project_dirs = ProjectDirs::from("dev", "outfitter", "cache");

        if let Some(old_dirs) = old_project_dirs {
            let old_root = old_dirs.data_dir();

            // Check if old directory exists and has content
            if old_root.exists() && old_root.is_dir() {
                // Check if there's actually content to migrate (look for llms.json files)
                let has_content = fs::read_dir(old_root)
                    .map(|entries| {
                        entries.filter_map(std::result::Result::ok).any(|entry| {
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{FileInfo, LineIndex, Source, SourceVariant, TocEntry};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = Storage::with_root(temp_dir.path().to_path_buf())
            .expect("Failed to create test storage");
        (storage, temp_dir)
    }

    fn create_test_llms_json(source_name: &str) -> LlmsJson {
        LlmsJson {
            source: source_name.to_string(),
            metadata: Source {
                url: format!("https://example.com/{source_name}/llms.txt"),
                etag: Some("abc123".to_string()),
                last_modified: None,
                fetched_at: Utc::now(),
                sha256: "deadbeef".to_string(),
                variant: SourceVariant::Llms,
                aliases: Vec::new(),
                tags: Vec::new(),
                description: None,
                category: None,
                npm_aliases: Vec::new(),
                github_aliases: Vec::new(),
                origin: crate::types::SourceOrigin {
                    manifest: None,
                    source_type: Some(crate::types::SourceType::Remote {
                        url: format!("https://example.com/{source_name}/llms.txt"),
                    }),
                },
                filter_non_english: None,
            },
            toc: vec![TocEntry {
                heading_path: vec!["Getting Started".to_string()],
                heading_path_display: Some(vec!["Getting Started".to_string()]),
                heading_path_normalized: Some(vec!["getting started".to_string()]),
                lines: "1-50".to_string(),
                anchor: None,
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
            parse_meta: None,
            filter_stats: None,
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
        assert_eq!(llms_json.source, loaded_json.source);
        assert_eq!(llms_json.metadata.url, loaded_json.metadata.url);
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
        assert_eq!(original.source, loaded.source);
        assert_eq!(original.metadata.url, loaded.metadata.url);
        assert_eq!(original.metadata.sha256, loaded.metadata.sha256);
        assert_eq!(original.toc.len(), loaded.toc.len());
        assert_eq!(original.files.len(), loaded.files.len());
        assert_eq!(
            original.line_index.total_lines,
            loaded.line_index.total_lines
        );
        assert_eq!(original.diagnostics.len(), loaded.diagnostics.len());
    }
}
