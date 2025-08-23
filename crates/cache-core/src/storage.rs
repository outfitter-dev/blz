use crate::{Error, LlmsJson, Result};
use chrono::Utc;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct Storage {
    root_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("dev", "outfitter", "cache")
            .ok_or_else(|| Error::Storage("Failed to determine project directories".into()))?;
        
        let root_dir = project_dirs.data_dir().to_path_buf();
        Self::with_root(root_dir)
    }
    
    pub fn with_root(root_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root_dir)
            .map_err(|e| Error::Storage(format!("Failed to create root directory: {}", e)))?;
        
        Ok(Self { root_dir })
    }
    
    pub fn tool_dir(&self, alias: &str) -> PathBuf {
        self.root_dir.join(alias)
    }
    
    pub fn ensure_tool_dir(&self, alias: &str) -> Result<PathBuf> {
        let dir = self.tool_dir(alias);
        fs::create_dir_all(&dir)
            .map_err(|e| Error::Storage(format!("Failed to create tool directory: {}", e)))?;
        Ok(dir)
    }
    
    pub fn llms_txt_path(&self, alias: &str) -> PathBuf {
        self.tool_dir(alias).join("llms.txt")
    }
    
    pub fn llms_json_path(&self, alias: &str) -> PathBuf {
        self.tool_dir(alias).join("llms.json")
    }
    
    pub fn index_dir(&self, alias: &str) -> PathBuf {
        self.tool_dir(alias).join(".index")
    }
    
    pub fn archive_dir(&self, alias: &str) -> PathBuf {
        self.tool_dir(alias).join(".archive")
    }
    
    pub fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()> {
        self.ensure_tool_dir(alias)?;
        let path = self.llms_txt_path(alias);
        fs::write(&path, content)
            .map_err(|e| Error::Storage(format!("Failed to write llms.txt: {}", e)))?;
        debug!("Saved llms.txt for {}", alias);
        Ok(())
    }
    
    pub fn load_llms_txt(&self, alias: &str) -> Result<String> {
        let path = self.llms_txt_path(alias);
        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.txt: {}", e)))
    }
    
    pub fn save_llms_json(&self, alias: &str, data: &LlmsJson) -> Result<()> {
        self.ensure_tool_dir(alias)?;
        let path = self.llms_json_path(alias);
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| Error::Storage(format!("Failed to serialize JSON: {}", e)))?;
        fs::write(&path, json)
            .map_err(|e| Error::Storage(format!("Failed to write llms.json: {}", e)))?;
        debug!("Saved llms.json for {}", alias);
        Ok(())
    }
    
    pub fn load_llms_json(&self, alias: &str) -> Result<LlmsJson> {
        let path = self.llms_json_path(alias);
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read llms.json: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse JSON: {}", e)))
    }
    
    pub fn exists(&self, alias: &str) -> bool {
        self.llms_json_path(alias).exists()
    }
    
    pub fn list_sources(&self) -> Result<Vec<String>> {
        let mut sources = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if !name.starts_with('.') && self.llms_json_path(name).exists() {
                            sources.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        sources.sort();
        Ok(sources)
    }
    
    pub fn archive(&self, alias: &str) -> Result<()> {
        let archive_dir = self.archive_dir(alias);
        fs::create_dir_all(&archive_dir)
            .map_err(|e| Error::Storage(format!("Failed to create archive directory: {}", e)))?;
        
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%MZ");
        
        let llms_txt = self.llms_txt_path(alias);
        if llms_txt.exists() {
            let archive_path = archive_dir.join(format!("{}-llms.txt", timestamp));
            fs::copy(&llms_txt, &archive_path)
                .map_err(|e| Error::Storage(format!("Failed to archive llms.txt: {}", e)))?;
        }
        
        let llms_json = self.llms_json_path(alias);
        if llms_json.exists() {
            let archive_path = archive_dir.join(format!("{}-llms.json", timestamp));
            fs::copy(&llms_json, &archive_path)
                .map_err(|e| Error::Storage(format!("Failed to archive llms.json: {}", e)))?;
        }
        
        info!("Archived {} at {}", alias, timestamp);
        Ok(())
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to create storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Source, FileInfo, LineIndex, TocEntry};
    use tempfile::TempDir;
    use std::fs;

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
                url: format!("https://example.com/{}/llms.txt", alias),
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
        
        let tool_dir = storage.tool_dir("react");
        let llms_txt_path = storage.llms_txt_path("react");
        let llms_json_path = storage.llms_json_path("react");
        let index_dir = storage.index_dir("react");
        let archive_dir = storage.archive_dir("react");
        
        assert!(tool_dir.ends_with("react"));
        assert!(llms_txt_path.ends_with("react/llms.txt"));
        assert!(llms_json_path.ends_with("react/llms.json"));
        assert!(index_dir.ends_with("react/.index"));
        assert!(archive_dir.ends_with("react/.archive"));
    }

    #[test]
    fn test_ensure_tool_directory() {
        let (storage, _temp_dir) = create_test_storage();
        
        let tool_dir = storage.ensure_tool_dir("react").expect("Should create tool dir");
        assert!(tool_dir.exists());
        
        // Should be idempotent
        let tool_dir2 = storage.ensure_tool_dir("react").expect("Should not fail on existing dir");
        assert_eq!(tool_dir, tool_dir2);
    }

    #[test]
    fn test_save_and_load_llms_txt() {
        let (storage, _temp_dir) = create_test_storage();
        
        let content = "# React Documentation\n\nThis is the React documentation...";
        
        // Save content
        storage.save_llms_txt("react", content).expect("Should save llms.txt");
        
        // Verify file exists
        assert!(storage.llms_txt_path("react").exists());
        
        // Load content
        let loaded_content = storage.load_llms_txt("react").expect("Should load llms.txt");
        assert_eq!(content, loaded_content);
    }

    #[test]
    fn test_save_and_load_llms_json() {
        let (storage, _temp_dir) = create_test_storage();
        
        let llms_json = create_test_llms_json("react");
        
        // Save JSON
        storage.save_llms_json("react", &llms_json).expect("Should save llms.json");
        
        // Verify file exists
        assert!(storage.llms_json_path("react").exists());
        
        // Load JSON
        let loaded_json = storage.load_llms_json("react").expect("Should load llms.json");
        assert_eq!(llms_json.alias, loaded_json.alias);
        assert_eq!(llms_json.source.url, loaded_json.source.url);
        assert_eq!(llms_json.line_index.total_lines, loaded_json.line_index.total_lines);
    }

    #[test]
    fn test_source_exists() {
        let (storage, _temp_dir) = create_test_storage();
        
        // Initially should not exist
        assert!(!storage.exists("react"));
        
        // After saving llms.json, should exist
        let llms_json = create_test_llms_json("react");
        storage.save_llms_json("react", &llms_json).expect("Should save");
        
        assert!(storage.exists("react"));
    }

    #[test]
    fn test_list_sources_empty() {
        let (storage, _temp_dir) = create_test_storage();
        
        let sources = storage.list_sources().expect("Should list sources");
        assert!(sources.is_empty());
    }

    #[test]
    fn test_list_sources_with_data() {
        let (storage, _temp_dir) = create_test_storage();
        
        // Add multiple sources
        let aliases = ["react", "nextjs", "rust"];
        for &alias in &aliases {
            let llms_json = create_test_llms_json(alias);
            storage.save_llms_json(alias, &llms_json).expect("Should save");
        }
        
        let sources = storage.list_sources().expect("Should list sources");
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
        storage.save_llms_json("react", &llms_json).expect("Should save");
        
        let sources = storage.list_sources().expect("Should list sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], "react");
    }

    #[test]
    fn test_list_sources_requires_llms_json() {
        let (storage, _temp_dir) = create_test_storage();
        
        // Create tool directory without llms.json
        storage.ensure_tool_dir("incomplete").expect("Should create dir");
        
        // Save only llms.txt (no llms.json)
        storage.save_llms_txt("incomplete", "# Test content").expect("Should save txt");
        
        // Create another source with complete data
        let llms_json = create_test_llms_json("complete");
        storage.save_llms_json("complete", &llms_json).expect("Should save json");
        
        let sources = storage.list_sources().expect("Should list sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], "complete");
    }

    #[test]
    fn test_archive_functionality() {
        let (storage, _temp_dir) = create_test_storage();
        
        // Create source data
        let content = "# Test content";
        let llms_json = create_test_llms_json("test");
        
        storage.save_llms_txt("test", content).expect("Should save txt");
        storage.save_llms_json("test", &llms_json).expect("Should save json");
        
        // Archive the source
        storage.archive("test").expect("Should archive");
        
        // Verify archive directory exists
        let archive_dir = storage.archive_dir("test");
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
        let archive_dir = storage.archive_dir("nonexistent");
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
        storage.save_llms_json("test", &original).expect("Should save");
        let loaded = storage.load_llms_json("test").expect("Should load");
        
        // Verify all fields are preserved
        assert_eq!(original.alias, loaded.alias);
        assert_eq!(original.source.url, loaded.source.url);
        assert_eq!(original.source.sha256, loaded.source.sha256);
        assert_eq!(original.toc.len(), loaded.toc.len());
        assert_eq!(original.files.len(), loaded.files.len());
        assert_eq!(original.line_index.total_lines, loaded.line_index.total_lines);
        assert_eq!(original.diagnostics.len(), loaded.diagnostics.len());
    }
}