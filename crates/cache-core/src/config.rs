use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub defaults: DefaultsConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    pub refresh_hours: u32,
    pub max_archives: usize,
    pub fetch_enabled: bool,
    pub follow_links: FollowLinks,
    pub allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowLinks {
    None,
    FirstParty,
    Allowlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub root: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| Error::Config(format!("Failed to read config: {}", e)))?;
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))
        } else {
            Ok(Self::default())
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let parent = config_path.parent()
            .ok_or_else(|| Error::Config("Invalid config path".into()))?;
        
        fs::create_dir_all(parent)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(&config_path, content)
            .map_err(|e| Error::Config(format!("Failed to write config: {}", e)))?;
        
        Ok(())
    }
    
    fn config_path() -> Result<PathBuf> {
        let project_dirs = directories::ProjectDirs::from("dev", "outfitter", "cache")
            .ok_or_else(|| Error::Config("Failed to determine project directories".into()))?;
        
        Ok(project_dirs.config_dir().join("global.toml"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: DefaultsConfig {
                refresh_hours: 24,
                max_archives: 10,
                fetch_enabled: true,
                follow_links: FollowLinks::FirstParty,
                allowlist: Vec::new(),
            },
            paths: PathsConfig {
                root: directories::ProjectDirs::from("dev", "outfitter", "cache")
                    .map(|dirs| dirs.data_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("~/.outfitter/cache")),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub meta: ToolMeta,
    pub fetch: FetchConfig,
    pub index: IndexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    pub name: String,
    pub display_name: Option<String>,
    pub homepage: Option<String>,
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchConfig {
    pub refresh_hours: Option<u32>,
    pub follow_links: Option<FollowLinks>,
    pub allowlist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub max_heading_block_lines: Option<usize>,
}

impl ToolConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read tool config: {}", e)))?;
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse tool config: {}", e)))
    }
    
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize tool config: {}", e)))?;
        fs::write(path, content)
            .map_err(|e| Error::Config(format!("Failed to write tool config: {}", e)))?;
        Ok(())
    }
}