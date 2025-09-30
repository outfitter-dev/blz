use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use blz_core::config::Config;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item, table, value};
use tracing::warn;

use crate::utils::preferences;
use crate::utils::store;

const ADD_KEY: &str = "add";
const PREFER_FULL_KEY: &str = "prefer_full";

#[derive(Debug, Clone, Copy)]
pub enum PreferenceScope {
    Global,
    Local,
    Project,
}

fn parse_env_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub fn effective_prefer_llms_full() -> bool {
    if let Some(local) = local_prefer_llms_full() {
        return local;
    }

    if let Ok(raw) = std::env::var("BLZ_PREFER_LLMS_FULL") {
        if let Some(value) = parse_env_bool(&raw) {
            return value;
        }
    }

    match Config::load() {
        Ok(cfg) => cfg.defaults.prefer_llms_full,
        Err(err) => {
            warn!(error = %err, "failed to load blz config; using defaults");
            false
        },
    }
}

pub fn get_prefer_llms_full(scope: PreferenceScope) -> Option<bool> {
    match scope {
        PreferenceScope::Global => read_prefer_full_from_path(&global_config_path()),
        PreferenceScope::Project => read_prefer_full_from_path(&project_config_path()),
        PreferenceScope::Local => local_prefer_llms_full(),
    }
}

pub fn set_prefer_llms_full(scope: PreferenceScope, value: bool) -> Result<()> {
    match scope {
        PreferenceScope::Global => {
            set_prefer_full_in_config(&global_config_path(), value)?;
        },
        PreferenceScope::Project => {
            let path = project_config_path();
            set_prefer_full_in_config(&path, value)?;
        },
        PreferenceScope::Local => {
            set_local_prefer_llms_full(value)?;
        },
    }
    Ok(())
}

fn global_config_path() -> PathBuf {
    store::global_config_dir().join("config.toml")
}

fn project_config_path() -> PathBuf {
    if let Ok(file) = std::env::var("BLZ_CONFIG") {
        let trimmed = file.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if !path.as_os_str().is_empty() {
                return path;
            }
        }
    }

    if let Ok(dir) = std::env::var("BLZ_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).join("config.toml");
        }
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".blz")
        .join("config.toml")
}

fn read_prefer_full_from_path(path: &Path) -> Option<bool> {
    let contents = fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&contents).ok()?;
    value.get("defaults")?.get("prefer_llms_full")?.as_bool()
}

fn set_prefer_full_in_config(path: &Path, flag: bool) -> Result<()> {
    let mut doc = if path.exists() {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        contents
            .parse::<DocumentMut>()
            .unwrap_or_else(|_| DocumentMut::new())
    } else {
        DocumentMut::new()
    };

    if !doc.as_table().contains_key("defaults") || !doc["defaults"].is_table() {
        doc["defaults"] = table();
    }

    let defaults_table = doc
        .get_mut("defaults")
        .and_then(Item::as_table_mut)
        .context("defaults table missing")?;

    defaults_table["prefer_llms_full"] = value(flag);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut serialized = doc.to_string();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn local_prefer_llms_full() -> Option<bool> {
    let key = preferences::local_scope_key()?;
    let store = store::load_store();
    let record = store.scopes.get(&key)?;
    extract_prefer_full(&record.user_settings)
}

fn set_local_prefer_llms_full(value: bool) -> Result<()> {
    let key = preferences::local_scope_key()
        .ok_or_else(|| anyhow::anyhow!("unable to determine local scope"))?;
    let mut store = store::load_store();
    let record = store.scopes.entry(key).or_default();

    let mut settings = record
        .user_settings
        .as_object()
        .map_or_else(Map::new, Map::clone);

    let mut add_map = settings
        .remove(ADD_KEY)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    add_map.insert(PREFER_FULL_KEY.to_string(), Value::Bool(value));
    settings.insert(ADD_KEY.to_string(), Value::Object(add_map));

    record.user_settings = Value::Object(settings);
    store::save_store(&store)?;
    Ok(())
}

fn extract_prefer_full(settings: &Value) -> Option<bool> {
    let add = settings.as_object()?.get(ADD_KEY)?.as_object()?;
    add.get(PREFER_FULL_KEY)?.as_bool()
}

#[cfg(test)]
#[allow(unsafe_code, clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use anyhow::Result;
    use blz_core::config::Config;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn write_config(path: &Path, config: &Config) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(config)?;
        fs::write(path, content)?;
        Ok(())
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn new(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var_os(key),
            }
        }

        fn set<S: AsRef<std::ffi::OsStr>>(&self, value: S) {
            unsafe {
                std::env::set_var(self.key, value);
            }
        }

        fn remove(&self) {
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                unsafe {
                    std::env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn new() -> Result<Self> {
            Ok(Self {
                original: std::env::current_dir()?,
            })
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    // Tests for prefer_llms_full configuration removed - feature is now always enabled via FORCE_PREFER_FULL

    #[test]
    fn project_config_path_prefers_explicit_file() -> Result<()> {
        let _guard = crate::utils::test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let config_dir = tempdir()?;
        let config_file_dir = tempdir()?;
        let config_file = config_file_dir.path().join("custom-config.toml");

        let blz_config_guard = EnvVarGuard::new("BLZ_CONFIG");
        blz_config_guard.set(config_file.as_os_str());
        let blz_config_dir_guard = EnvVarGuard::new("BLZ_CONFIG_DIR");
        blz_config_dir_guard.set(config_dir.path());

        assert_eq!(project_config_path(), config_file);

        Ok(())
    }
}
