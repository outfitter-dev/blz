use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

const STORE_FILENAME: &str = "blz.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlzStore {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub scopes: HashMap<String, ScopeRecord>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScopeRecord {
    #[serde(default)]
    pub cli_preferences: Value,
    #[serde(default)]
    pub user_settings: Value,
    #[serde(default)]
    pub sources: HashMap<String, SourceOverrides>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SourceOverrides {
    #[serde(default)]
    pub preferred_flavor: Option<String>,
}

const fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

impl Default for BlzStore {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            scopes: HashMap::new(),
        }
    }
}

pub fn load_store() -> BlzStore {
    let path = store_path();
    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<BlzStore>(&bytes) {
            Ok(store) => {
                if store.schema_version == CURRENT_SCHEMA_VERSION {
                    store
                } else {
                    warn!(
                        "blz.json schema version {} unsupported; resetting store",
                        store.schema_version
                    );
                    BlzStore::default()
                }
            },
            Err(err) => {
                warn!("failed to parse blz.json at {}: {err}", path.display());
                BlzStore::default()
            },
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => BlzStore::default(),
        Err(err) => {
            warn!("failed to read blz.json at {}: {err}", path.display());
            BlzStore::default()
        },
    }
}

pub fn save_store(store: &BlzStore) -> std::io::Result<()> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(store)?;
    fs::write(path, data)
}

fn store_path() -> PathBuf {
    active_config_dir().join(STORE_FILENAME)
}

pub fn active_config_dir() -> PathBuf {
    if let Ok(file) = std::env::var("BLZ_CONFIG") {
        let trimmed = file.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if let Some(parent) = path.parent() {
                return parent.to_path_buf();
            }
        }
    }

    if let Ok(dir) = std::env::var("BLZ_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).to_path_buf();
        }
    }

    global_config_dir()
}

pub fn global_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BLZ_GLOBAL_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).to_path_buf();
        }
    }
    if let Some(project_dirs) = ProjectDirs::from("dev", "outfitter", "blz") {
        return project_dirs.config_dir().to_path_buf();
    }
    Path::new(".").to_path_buf()
}

#[cfg(test)]
#[allow(unsafe_code, clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::tempdir;

    fn with_temp_config_dir<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&Path) -> Result<R>,
    {
        let _guard = crate::utils::test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");
        let dir = tempdir()?;
        unsafe {
            std::env::set_var("BLZ_CONFIG_DIR", dir.path());
            std::env::remove_var("BLZ_CONFIG");
        }
        let result = f(dir.path());
        unsafe {
            std::env::remove_var("BLZ_CONFIG_DIR");
        }
        result
    }

    #[test]
    fn load_store_returns_default_when_missing() -> Result<()> {
        with_temp_config_dir(|_| {
            let store = load_store();
            assert_eq!(store.schema_version, CURRENT_SCHEMA_VERSION);
            assert!(store.scopes.is_empty());
            Ok(())
        })
    }

    #[test]
    fn load_store_returns_default_on_invalid_json() -> Result<()> {
        with_temp_config_dir(|dir| {
            let path = dir.join(STORE_FILENAME);
            fs::create_dir_all(dir)?;
            fs::write(&path, b"not-json")?;

            let store = load_store();
            assert_eq!(store.schema_version, CURRENT_SCHEMA_VERSION);
            assert!(store.scopes.is_empty());
            Ok(())
        })
    }

    #[test]
    fn load_store_resets_on_schema_mismatch() -> Result<()> {
        with_temp_config_dir(|dir| {
            let path = dir.join(STORE_FILENAME);
            fs::create_dir_all(dir)?;
            let bad = serde_json::json!({
                "schema_version": CURRENT_SCHEMA_VERSION + 1,
                "scopes": {
                    "local:/tmp": {
                        "cli_preferences": serde_json::json!({"default_show": ["url"]}),
                        "user_settings": serde_json::json!({})
                    }
                }
            });
            fs::write(&path, serde_json::to_vec(&bad)?)?;

            let store = load_store();
            assert_eq!(store.schema_version, CURRENT_SCHEMA_VERSION);
            assert!(store.scopes.is_empty());
            Ok(())
        })
    }

    #[test]
    fn save_store_creates_parent_directories() -> Result<()> {
        with_temp_config_dir(|dir| {
            let mut store = BlzStore::default();
            store
                .scopes
                .insert("local:/tmp".into(), ScopeRecord::default());
            save_store(&store)?;

            let path = dir.join(STORE_FILENAME);
            assert!(path.exists());

            let loaded = load_store();
            assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
            assert!(loaded.scopes.contains_key("local:/tmp"));
            Ok(())
        })
    }

    #[test]
    fn save_store_persists_source_overrides() -> Result<()> {
        with_temp_config_dir(|_| {
            let mut store = BlzStore::default();
            let mut record = ScopeRecord::default();
            record.sources.insert(
                "react".into(),
                SourceOverrides {
                    preferred_flavor: Some("llms-full".to_string()),
                },
            );
            store.scopes.insert("global".into(), record);
            save_store(&store)?;

            let loaded = load_store();
            let reloaded = loaded.scopes.get("global").expect("global scope present");
            let override_flavor = reloaded
                .sources
                .get("react")
                .and_then(|s| s.preferred_flavor.as_deref());
            assert_eq!(override_flavor, Some("llms-full"));
            Ok(())
        })
    }
}
