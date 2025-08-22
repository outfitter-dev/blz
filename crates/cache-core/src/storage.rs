use crate::{Error, LlmsJson, Result, Source};
use chrono::Utc;
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};
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