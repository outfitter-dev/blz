//! Configuration management for blz cache system.
//!
//! This module provides hierarchical configuration with global defaults and per-source overrides.
//! Configuration is stored in TOML format and supports environment variable overrides.
//!
//! ## Configuration Hierarchy
//!
//! 1. **Global config**: Platform-specific config directory (see `GlobalConfig` docs)
//! 2. **Per-source config**: `<source_dir>/settings.toml`
//! 3. **Environment variables**: `CACHE_*` prefix
//!
//! ## Examples
//!
//! ### Loading global configuration:
//!
//! ```rust
//! use blz_core::{Config, Result};
//!
//! // Load from default location or create with defaults
//! let config = Config::load()?;
//! println!("Cache root: {}", config.paths.root.display());
//! println!("Refresh interval: {} hours", config.defaults.refresh_hours);
//! # Ok::<(), blz_core::Error>(())
//! ```
//!
//! ### Working with tool-specific configuration:
//!
//! ```rust,no_run
//! use blz_core::{ToolConfig, ToolMeta, FetchConfig, IndexConfig};
//! use std::path::Path;
//!
//! let tool_config = ToolConfig {
//!     meta: ToolMeta {
//!         name: "react".to_string(),
//!         display_name: Some("React Documentation".to_string()),
//!         homepage: Some("https://react.dev".to_string()),
//!         repo: Some("https://github.com/facebook/react".to_string()),
//!     },
//!     fetch: FetchConfig {
//!         refresh_hours: Some(12), // Override global default
//!         follow_links: None,      // Use global default
//!         allowlist: None,         // Use global default
//!     },
//!     index: IndexConfig {
//!         max_heading_block_lines: Some(500),
//!         filter_non_english: None, // Use global default
//!     },
//! };
//!
//! // Save to file
//! tool_config.save(Path::new("react/settings.toml"))?;
//! # Ok::<(), blz_core::Error>(())
//! ```

use crate::{Error, Result, profile};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default value for `filter_non_english` setting.
///
/// Returns `true` to enable non-English content filtering by default,
/// maintaining backward compatibility with existing behavior.
const fn default_filter_non_english() -> bool {
    true
}

/// Global configuration for the blz cache system.
///
/// Contains default settings that apply to all sources unless overridden by per-source configuration.
/// Configuration is automatically loaded from the system config directory or created with sensible defaults.
///
/// ## File Location
///
/// The configuration file is stored at (searched in order):
/// - XDG: `$XDG_CONFIG_HOME/blz/config.toml` or `~/.config/blz/config.toml`
/// - Dotfile fallback: `~/.blz/config.toml`
///
/// A `config.local.toml` in the same directory overrides keys from `config.toml`.
///
/// ## Example Configuration File
///
/// ```toml
/// [defaults]
/// refresh_hours = 24
/// max_archives = 10
/// fetch_enabled = true
/// follow_links = "first_party"
/// allowlist = ["docs.rs", "developer.mozilla.org"]
///
/// [paths]
/// root = "/home/user/.outfitter/blz"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default settings for all sources
    pub defaults: DefaultsConfig,
    /// File system paths configuration
    pub paths: PathsConfig,
}

/// Default settings that apply to all sources unless overridden.
///
/// These settings control fetching behavior, caching policies, and link following rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// How often to refresh cached content (in hours).
    ///
    /// Sources are only re-fetched if they haven't been updated within this interval.
    /// Set to 0 to always fetch on access.
    pub refresh_hours: u32,

    /// Maximum number of archived versions to keep per source.
    ///
    /// When a source is updated, the previous version is archived. This setting
    /// controls how many historical versions to retain for diff generation.
    pub max_archives: usize,

    /// Whether fetching from remote sources is enabled.
    ///
    /// When disabled, only locally cached content is used. Useful for offline work
    /// or environments with restricted network access.
    pub fetch_enabled: bool,

    /// Policy for following links in llms.txt files.
    ///
    /// Controls whether and which external links should be followed when processing
    /// llms.txt files that contain references to other documentation sources.
    pub follow_links: FollowLinks,

    /// Domains allowed for link following.
    ///
    /// Only used when `follow_links` is set to `Allowlist`. Links to domains
    /// not in this list will be ignored.
    pub allowlist: Vec<String>,

    /// Default language filtering behavior.
    ///
    /// When `true`, non-English content is filtered during document processing.
    /// When `false`, all content is retained regardless of language.
    /// Defaults to `true` for backward compatibility.
    #[serde(default = "default_filter_non_english")]
    pub filter_non_english: bool,
}

/// Policy for following external links in llms.txt files.
///
/// This controls how the system handles links to other documentation sources
/// within llms.txt files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowLinks {
    /// Never follow external links.
    ///
    /// Only process the original llms.txt file, ignoring any links to other sources.
    None,

    /// Follow links to the same domain and its immediate subdomains.
    ///
    /// For example, if processing `docs.example.com/llms.txt`, links to
    /// `api.example.com/docs` or `example.com/guide` would be followed,
    /// but `other-site.com/docs` would be ignored.
    FirstParty,

    /// Only follow links to domains in the allowlist.
    ///
    /// Use the `allowlist` field in `DefaultsConfig` to specify which domains
    /// are permitted. This provides fine-grained control over which external
    /// sources are trusted.
    Allowlist,
}

/// File system paths configuration.
///
/// Defines where cached content, indices, and metadata are stored on the local filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Root directory for all cached content.
    ///
    /// Each source gets its own subdirectory under this root. The directory
    /// structure is: `root/<source_alias>/`
    ///
    /// Default locations:
    /// - Linux: `~/.local/share/blz`
    /// - macOS: `~/Library/Application Support/dev.outfitter.blz`
    /// - Windows: `%APPDATA%\outfitter\blz`
    pub root: PathBuf,
}

impl Config {
    /// Load configuration from the default location or create with defaults.
    ///
    /// This method attempts to load the configuration from the system config directory.
    /// If the file doesn't exist, it returns a configuration with sensible defaults.
    /// If the file exists but is malformed, it returns an error.
    ///
    /// # Returns
    ///
    /// Returns the loaded configuration or a default configuration if no file exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The config directory cannot be determined (unsupported platform)
    /// - The config file exists but cannot be read (permissions, I/O error)
    /// - The config file exists but contains invalid TOML syntax
    /// - The config file exists but contains invalid configuration values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blz_core::Config;
    ///
    /// // Load existing config or create with defaults
    /// let config = Config::load()?;
    ///
    /// if config.defaults.fetch_enabled {
    ///     println!("Fetching is enabled");
    /// }
    /// # Ok::<(), blz_core::Error>(())
    /// ```
    pub fn load() -> Result<Self> {
        // Determine base config path (BLZ_CONFIG/BLZ_CONFIG_DIR, XDG, dotfile), or use defaults
        let base_path = Self::existing_config_path()?;

        // Load base
        let mut base_value: toml::Value = if let Some(ref path) = base_path {
            let content = fs::read_to_string(path)
                .map_err(|e| Error::Config(format!("Failed to read config: {e}")))?;
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse config: {e}")))?
        } else {
            let default_str = toml::to_string(&Self::default())
                .map_err(|e| Error::Config(format!("Failed to init default config: {e}")))?;
            toml::from_str(&default_str)
                .map_err(|e| Error::Config(format!("Failed to init default config: {e}")))?
        };

        // Merge optional local override next to resolved base directory
        let base_dir = base_path.as_deref().map_or_else(
            || {
                Self::canonical_config_path().map_or_else(
                    |_| PathBuf::new(),
                    |p| p.parent().map(Path::to_path_buf).unwrap_or_default(),
                )
            },
            |bp| bp.parent().map(Path::to_path_buf).unwrap_or_default(),
        );

        let local_path = base_dir.join("config.local.toml");
        if local_path.exists() {
            let content = fs::read_to_string(&local_path)
                .map_err(|e| Error::Config(format!("Failed to read local config: {e}")))?;
            let local_value: toml::Value = toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse local config: {e}")))?;
            Self::merge_toml(&mut base_value, &local_value);
        }

        // Deserialize
        let mut config: Self = base_value
            .try_into()
            .map_err(|e| Error::Config(format!("Failed to materialize config: {e}")))?;

        // Apply env overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Save the configuration to the default location.
    ///
    /// This method serializes the configuration to TOML format and writes it to
    /// the system config directory. Parent directories are created if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The config directory cannot be determined (unsupported platform)
    /// - Parent directories cannot be created (permissions, disk space)
    /// - The configuration cannot be serialized to TOML
    /// - The file cannot be written (permissions, disk space, I/O error)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use blz_core::{Config, DefaultsConfig, PathsConfig, FollowLinks};
    /// use std::path::PathBuf;
    ///
    /// let mut config = Config::load()?;
    /// config.defaults.refresh_hours = 12; // Update refresh interval
    /// config.save()?; // Persist changes
    /// # Ok::<(), blz_core::Error>(())
    /// ```
    pub fn save(&self) -> Result<()> {
        let config_path = Self::save_target_path()?;
        let parent = config_path
            .parent()
            .ok_or_else(|| Error::Config("Invalid config path".into()))?;

        fs::create_dir_all(parent)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {e}")))?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {e}")))?;

        let tmp = parent.join("config.toml.tmp");
        fs::write(&tmp, &content)
            .map_err(|e| Error::Config(format!("Failed to write temp config: {e}")))?;
        // Best-effort atomic replace; on Windows, rename() replaces if target does not exist.
        // SAFETY: config.toml write is replaced in one step to avoid torn files.
        #[cfg(target_os = "windows")]
        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| Error::Config(format!("Failed to remove existing config: {e}")))?;
        }
        std::fs::rename(&tmp, &config_path)
            .map_err(|e| Error::Config(format!("Failed to replace config: {e}")))?;

        Ok(())
    }

    /// Get the path where the global configuration file is stored.
    ///
    /// Uses the system-appropriate config directory based on the platform:
    /// - Linux: `~/.config/blz/global.toml`
    /// - macOS: `~/Library/Application Support/dev.outfitter.blz/global.toml`
    /// - Windows: `%APPDATA%\outfitter\blz\global.toml`
    ///
    /// # Errors
    ///
    /// Returns an error if the system config directory cannot be determined,
    /// which may happen on unsupported platforms or in sandboxed environments.
    fn canonical_config_path() -> Result<PathBuf> {
        let xdg = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| directories::BaseDirs::new().map(|b| b.home_dir().join(".config")))
            .ok_or_else(|| Error::Config("Failed to determine XDG config directory".into()))?;
        Ok(xdg.join(profile::app_dir_slug()).join("config.toml"))
    }

    fn dotfile_config_path() -> Result<PathBuf> {
        let home = directories::BaseDirs::new()
            .map(|b| b.home_dir().to_path_buf())
            .ok_or_else(|| Error::Config("Failed to determine home directory".into()))?;
        Ok(home.join(profile::dot_dir_slug()).join("config.toml"))
    }

    fn existing_config_path() -> Result<Option<PathBuf>> {
        // 1) BLZ_CONFIG (file)
        if let Ok(explicit) = std::env::var("BLZ_CONFIG") {
            let explicit = explicit.trim();
            if !explicit.is_empty() {
                let p = PathBuf::from(explicit);
                if p.is_file() && p.exists() {
                    return Ok(Some(p));
                }
            }
        }

        // 2) BLZ_CONFIG_DIR (dir)
        if let Ok(dir) = std::env::var("BLZ_CONFIG_DIR") {
            let dir = dir.trim();
            if !dir.is_empty() {
                let p = PathBuf::from(dir).join("config.toml");
                if p.is_file() && p.exists() {
                    return Ok(Some(p));
                }
            }
        }

        // 3) XDG
        let xdg = Self::canonical_config_path()?;
        if xdg.exists() {
            return Ok(Some(xdg));
        }
        // 4) Dotfile
        let dot = Self::dotfile_config_path()?;
        if dot.exists() {
            return Ok(Some(dot));
        }
        Ok(None)
    }

    fn save_target_path() -> Result<PathBuf> {
        if let Some(existing) = Self::existing_config_path()? {
            return Ok(existing);
        }
        Self::canonical_config_path()
    }

    fn merge_toml(dst: &mut toml::Value, src: &toml::Value) {
        use toml::Value::Table;
        match (dst, src) {
            (Table(dst_tbl), Table(src_tbl)) => {
                for (k, v) in src_tbl {
                    match dst_tbl.get_mut(k) {
                        Some(dst_v) => Self::merge_toml(dst_v, v),
                        None => {
                            dst_tbl.insert(k.clone(), v.clone());
                        },
                    }
                }
            },
            (dst_v, src_v) => *dst_v = src_v.clone(),
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("BLZ_REFRESH_HOURS") {
            if let Ok(n) = v.parse::<u32>() {
                self.defaults.refresh_hours = n;
            }
        }
        if let Ok(v) = std::env::var("BLZ_MAX_ARCHIVES") {
            if let Ok(n) = v.parse::<usize>() {
                self.defaults.max_archives = n;
            }
        }
        if let Ok(v) = std::env::var("BLZ_FETCH_ENABLED") {
            let norm = v.to_ascii_lowercase();
            self.defaults.fetch_enabled = matches!(norm.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("BLZ_FOLLOW_LINKS") {
            match v.to_ascii_lowercase().as_str() {
                "none" => self.defaults.follow_links = FollowLinks::None,
                "first_party" | "firstparty" => {
                    self.defaults.follow_links = FollowLinks::FirstParty;
                },
                "allowlist" => self.defaults.follow_links = FollowLinks::Allowlist,
                _ => {},
            }
        }
        if let Ok(v) = std::env::var("BLZ_ALLOWLIST") {
            let list = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !list.is_empty() {
                self.defaults.allowlist = list;
            }
        }
        if let Ok(v) = std::env::var("BLZ_ROOT") {
            let p = PathBuf::from(v);
            if !p.as_os_str().is_empty() {
                self.paths.root = p;
            }
        }
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
                filter_non_english: true,
            },
            paths: PathsConfig {
                root: directories::ProjectDirs::from("dev", "outfitter", profile::app_dir_slug())
                    .map_or_else(
                        || {
                            // Expand home directory properly
                            directories::BaseDirs::new().map_or_else(
                                || PathBuf::from(".outfitter").join(profile::app_dir_slug()),
                                |base| {
                                    base.home_dir()
                                        .join(".outfitter")
                                        .join(profile::app_dir_slug())
                                },
                            )
                        },
                        |dirs| dirs.data_dir().to_path_buf(),
                    ),
            },
        }
    }
}

/// Per-source configuration that overrides global defaults.
///
/// Each documentation source can have its own configuration file (`settings.toml`)
/// that overrides the global configuration for that specific source. This allows
/// fine-grained control over fetching behavior, indexing parameters, and metadata.
///
/// ## File Location
///
/// Stored as `<cache_root>/<source_alias>/settings.toml`
///
/// ## Example Configuration File
///
/// ```toml
/// [meta]
/// name = "react"
/// display_name = "React Documentation"
/// homepage = "https://react.dev"
/// repo = "https://github.com/facebook/react"
///
/// [fetch]
/// refresh_hours = 12  # Override global default
/// follow_links = "first_party"
/// allowlist = ["reactjs.org", "react.dev"]
///
/// [index]
/// max_heading_block_lines = 500
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Metadata about the documentation source
    pub meta: ToolMeta,
    /// Fetching behavior overrides
    pub fetch: FetchConfig,
    /// Indexing parameter overrides
    pub index: IndexConfig,
}

/// Metadata about a documentation source.
///
/// This information is used for display purposes and to provide context
/// about the source of documentation being cached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    /// Unique identifier for this source (used as directory name).
    ///
    /// Should be a valid filename that uniquely identifies the source.
    /// Typically lowercase with hyphens (e.g., "react", "node-js", "rust-std").
    pub name: String,

    /// Human-readable display name for the source.
    ///
    /// Used in search results and UI displays. If not provided, the `name`
    /// field is used as fallback.
    pub display_name: Option<String>,

    /// Homepage URL for the documentation source.
    ///
    /// The main website or documentation portal for this source.
    /// Used for reference and linking back to the original documentation.
    pub homepage: Option<String>,

    /// Repository URL for the documentation source.
    ///
    /// Link to the source code repository, if available. Useful for
    /// understanding the project context and accessing source code.
    pub repo: Option<String>,
}

/// Per-source fetching behavior overrides.
///
/// These settings override the global defaults for fetching behavior.
/// Any `None` values will use the corresponding global default setting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchConfig {
    /// Override for refresh interval in hours.
    ///
    /// If `Some`, overrides the global `refresh_hours` setting for this source.
    /// If `None`, uses the global default.
    pub refresh_hours: Option<u32>,

    /// Override for link following policy.
    ///
    /// If `Some`, overrides the global `follow_links` setting for this source.
    /// If `None`, uses the global default.
    pub follow_links: Option<FollowLinks>,

    /// Override for allowed domains list.
    ///
    /// If `Some`, overrides the global `allowlist` setting for this source.
    /// If `None`, uses the global default. Only used when `follow_links` is `Allowlist`.
    pub allowlist: Option<Vec<String>>,
}

/// Per-source indexing parameter overrides.
///
/// These settings control how the documentation is processed and indexed
/// for this specific source, overriding global defaults where specified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Maximum lines to include in a single heading block.
    ///
    /// Controls how large sections are broken up during indexing. Larger values
    /// include more context but may reduce search precision. Smaller values
    /// provide more focused results but may split related content.
    ///
    /// If `None`, uses a sensible default based on content analysis.
    pub max_heading_block_lines: Option<usize>,

    /// Override language filtering for this source.
    ///
    /// If `Some(true)`, non-English content will be filtered regardless of global default.
    /// If `Some(false)`, all content will be retained regardless of global default.
    /// If `None`, uses the global `filter_non_english` setting.
    pub filter_non_english: Option<bool>,
}

impl ToolConfig {
    /// Load per-source configuration from a file.
    ///
    /// Loads and parses a TOML configuration file for a specific documentation source.
    /// The file should contain sections for `[meta]`, `[fetch]`, and `[index]`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file (typically `settings.toml`)
    ///
    /// # Returns
    ///
    /// Returns the parsed configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read (doesn't exist, permissions, I/O error)
    /// - The file contains invalid TOML syntax
    /// - The file contains invalid configuration values
    /// - Required fields are missing (e.g., `meta.name`)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use blz_core::ToolConfig;
    /// use std::path::Path;
    ///
    /// // Load source-specific configuration
    /// let config_path = Path::new("sources/react/settings.toml");
    /// let tool_config = ToolConfig::load(config_path)?;
    ///
    /// println!("Source: {}", tool_config.meta.name);
    /// if let Some(refresh) = tool_config.fetch.refresh_hours {
    ///     println!("Custom refresh interval: {} hours", refresh);
    /// }
    /// # Ok::<(), blz_core::Error>(())
    /// ```
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read tool config: {e}")))?;
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse tool config: {e}")))
    }

    /// Save per-source configuration to a file.
    ///
    /// Serializes the configuration to TOML format and writes it to the specified path.
    /// The parent directory must already exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to save the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration cannot be serialized to TOML
    /// - The parent directory doesn't exist
    /// - The file cannot be written (permissions, disk space, I/O error)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use blz_core::{ToolConfig, ToolMeta, FetchConfig, IndexConfig};
    /// use std::path::Path;
    ///
    /// let config = ToolConfig {
    ///     meta: ToolMeta {
    ///         name: "my-docs".to_string(),
    ///         display_name: Some("My Documentation".to_string()),
    ///         homepage: None,
    ///         repo: None,
    ///     },
    ///     fetch: FetchConfig {
    ///         refresh_hours: Some(6),
    ///         follow_links: None,
    ///         allowlist: None,
    ///     },
    ///     index: IndexConfig {
    ///         max_heading_block_lines: Some(300),
    ///         filter_non_english: None,
    ///     },
    /// };
    ///
    /// let config_path = Path::new("my-docs/settings.toml");
    /// config.save(config_path)?;
    /// # Ok::<(), blz_core::Error>(())
    /// ```
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize tool config: {e}")))?;
        fs::write(path, content)
            .map_err(|e| Error::Config(format!("Failed to write tool config: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    // Test fixtures
    fn create_test_config() -> Config {
        Config {
            defaults: DefaultsConfig {
                refresh_hours: 12,
                max_archives: 5,
                fetch_enabled: true,
                follow_links: FollowLinks::Allowlist,
                allowlist: vec!["example.com".to_string(), "docs.rs".to_string()],
                filter_non_english: true,
            },
            paths: PathsConfig {
                root: PathBuf::from("/tmp/test"),
            },
        }
    }

    fn create_test_tool_config() -> ToolConfig {
        ToolConfig {
            meta: ToolMeta {
                name: "test-tool".to_string(),
                display_name: Some("Test Tool".to_string()),
                homepage: Some("https://test.com".to_string()),
                repo: Some("https://github.com/test/tool".to_string()),
            },
            fetch: FetchConfig {
                refresh_hours: Some(6),
                follow_links: Some(FollowLinks::FirstParty),
                allowlist: Some(vec!["allowed.com".to_string()]),
            },
            index: IndexConfig {
                max_heading_block_lines: Some(100),
                filter_non_english: None,
            },
        }
    }

    #[test]
    fn test_default_config_values() {
        // Given: Default configuration is requested
        let config = Config::default();

        // When: Examining default values
        // Then: Should have sensible defaults
        assert_eq!(config.defaults.refresh_hours, 24);
        assert_eq!(config.defaults.max_archives, 10);
        assert!(config.defaults.fetch_enabled);
        assert!(matches!(
            config.defaults.follow_links,
            FollowLinks::FirstParty
        ));
        assert!(config.defaults.allowlist.is_empty());
        assert!(config.defaults.filter_non_english);
        assert!(!config.paths.root.as_os_str().is_empty());
    }

    #[test]
    fn test_follow_links_serialization() -> Result<()> {
        // Given: Different FollowLinks variants
        let variants = vec![
            FollowLinks::None,
            FollowLinks::FirstParty,
            FollowLinks::Allowlist,
        ];

        for variant in variants {
            // When: Serializing and deserializing
            let serialized = serde_json::to_string(&variant)?;
            let deserialized: FollowLinks = serde_json::from_str(&serialized)?;

            // Then: Should round-trip correctly
            assert_eq!(variant, deserialized, "Round-trip failed for {variant:?}");
        }
        Ok(())
    }

    #[test]
    fn test_config_save_and_load_roundtrip() -> Result<()> {
        // Given: A temporary directory and test configuration
        let temp_dir = TempDir::new().map_err(|e| Error::Config(e.to_string()))?;
        let config_path = temp_dir.path().join("test_config.toml");
        let original_config = create_test_config();

        // When: Saving and then loading the configuration
        let content = toml::to_string_pretty(&original_config)
            .map_err(|e| Error::Config(format!("Failed to serialize: {e}")))?;
        fs::write(&config_path, content)
            .map_err(|e| Error::Config(format!("Failed to write: {e}")))?;

        let loaded_config: Config = {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| Error::Config(format!("Failed to read: {e}")))?;
            toml::from_str(&content).map_err(|e| Error::Config(format!("Failed to parse: {e}")))?
        };

        // Then: Configurations should be identical
        assert_eq!(
            loaded_config.defaults.refresh_hours,
            original_config.defaults.refresh_hours
        );
        assert_eq!(
            loaded_config.defaults.max_archives,
            original_config.defaults.max_archives
        );
        assert_eq!(
            loaded_config.defaults.fetch_enabled,
            original_config.defaults.fetch_enabled
        );
        assert_eq!(
            loaded_config.defaults.allowlist,
            original_config.defaults.allowlist
        );
        assert_eq!(loaded_config.paths.root, original_config.paths.root);

        Ok(())
    }

    #[test]
    fn test_config_load_missing_file() {
        // Given: A non-existent config file path
        let non_existent = PathBuf::from("/definitely/does/not/exist/config.toml");

        // When: Attempting to load config
        let result = (|| -> Result<Config> {
            let content = fs::read_to_string(&non_existent)
                .map_err(|e| Error::Config(format!("Failed to read config: {e}")))?;
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse config: {e}")))
        })();

        // Then: Should return appropriate error
        assert!(result.is_err());
        match result {
            Err(Error::Config(msg)) => assert!(msg.contains("Failed to read config")),
            _ => unreachable!("Expected Config error"),
        }
    }

    #[test]
    fn test_config_parse_invalid_toml() {
        // Given: Invalid TOML content
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("invalid.toml");
        fs::write(&config_path, "this is not valid toml [[[").expect("Failed to write test file");

        // When: Attempting to parse
        let result = (|| -> Result<Config> {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| Error::Config(format!("Failed to read config: {e}")))?;
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse config: {e}")))
        })();

        // Then: Should return parse error
        assert!(result.is_err());
        if let Err(Error::Config(msg)) = result {
            assert!(msg.contains("Failed to parse config"));
        } else {
            panic!("Expected Config parse error");
        }
    }

    #[test]
    fn test_config_save_creates_directory() -> Result<()> {
        // Given: A temporary directory and nested config path
        let temp_dir = TempDir::new().map_err(|e| Error::Config(e.to_string()))?;
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("deeper")
            .join("config.toml");
        let config = create_test_config();

        // When: Saving config to nested path (simulating Config::save logic)
        let parent = nested_path
            .parent()
            .ok_or_else(|| Error::Config("Invalid config path".into()))?;
        fs::create_dir_all(parent)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {e}")))?;

        let content = toml::to_string_pretty(&config)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {e}")))?;
        fs::write(&nested_path, content)
            .map_err(|e| Error::Config(format!("Failed to write config: {e}")))?;

        // Then: Directory should be created and file should exist
        assert!(nested_path.exists());
        assert!(
            nested_path
                .parent()
                .expect("path should have parent")
                .exists()
        );

        Ok(())
    }

    #[test]
    fn test_tool_config_roundtrip() -> Result<()> {
        // Given: A temporary file and test tool configuration
        let temp_dir = TempDir::new().map_err(|e| Error::Config(e.to_string()))?;
        let config_path = temp_dir.path().join("tool.toml");
        let original_config = create_test_tool_config();

        // When: Saving and loading the tool configuration
        original_config.save(&config_path)?;
        let loaded_config = ToolConfig::load(&config_path)?;

        // Then: Configurations should be identical
        assert_eq!(loaded_config.meta.name, original_config.meta.name);
        assert_eq!(
            loaded_config.meta.display_name,
            original_config.meta.display_name
        );
        assert_eq!(loaded_config.meta.homepage, original_config.meta.homepage);
        assert_eq!(loaded_config.meta.repo, original_config.meta.repo);
        assert_eq!(
            loaded_config.fetch.refresh_hours,
            original_config.fetch.refresh_hours
        );
        assert_eq!(
            loaded_config.fetch.allowlist,
            original_config.fetch.allowlist
        );
        assert_eq!(
            loaded_config.index.max_heading_block_lines,
            original_config.index.max_heading_block_lines
        );

        Ok(())
    }

    #[test]
    fn test_tool_config_load_nonexistent_file() {
        // Given: A non-existent file path
        let non_existent = PathBuf::from("/does/not/exist/tool.toml");

        // When: Attempting to load
        let result = ToolConfig::load(&non_existent);

        // Then: Should return appropriate error
        assert!(result.is_err());
        if let Err(Error::Config(msg)) = result {
            assert!(msg.contains("Failed to read tool config"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_config_with_extreme_values() -> Result<()> {
        // Given: Configuration with extreme but valid values (avoiding serialization limits)
        let extreme_config = Config {
            defaults: DefaultsConfig {
                refresh_hours: 1_000_000, // Large but not MAX to avoid TOML issues
                max_archives: 1_000_000,  // Large but not MAX to avoid TOML issues
                fetch_enabled: false,
                follow_links: FollowLinks::None,
                allowlist: vec!["a".repeat(1000)], // Very long domain
                filter_non_english: false,
            },
            paths: PathsConfig {
                root: PathBuf::from("/".repeat(100)), // Very long path
            },
        };

        // When: Serializing and deserializing
        let serialized = toml::to_string_pretty(&extreme_config)
            .map_err(|e| Error::Config(format!("Serialize failed: {e}")))?;
        let deserialized: Config = toml::from_str(&serialized)
            .map_err(|e| Error::Config(format!("Deserialize failed: {e}")))?;

        // Then: Should handle extreme values correctly
        assert_eq!(deserialized.defaults.refresh_hours, 1_000_000);
        assert_eq!(deserialized.defaults.max_archives, 1_000_000);
        assert!(!deserialized.defaults.fetch_enabled);
        assert_eq!(deserialized.defaults.allowlist.len(), 1);
        assert_eq!(deserialized.defaults.allowlist[0].len(), 1000);

        Ok(())
    }

    #[test]
    fn test_config_empty_allowlist() -> Result<()> {
        // Given: Configuration with empty allowlist
        let config = Config {
            defaults: DefaultsConfig {
                refresh_hours: 24,
                max_archives: 10,
                fetch_enabled: true,
                follow_links: FollowLinks::Allowlist,
                allowlist: vec![], // Empty allowlist
                filter_non_english: true,
            },
            paths: PathsConfig {
                root: PathBuf::from("/tmp"),
            },
        };

        // When: Serializing and deserializing
        let serialized = toml::to_string_pretty(&config)?;
        let deserialized: Config = toml::from_str(&serialized)?;

        // Then: Empty allowlist should be preserved
        assert!(deserialized.defaults.allowlist.is_empty());
        assert!(matches!(
            deserialized.defaults.follow_links,
            FollowLinks::Allowlist
        ));

        Ok(())
    }

    #[test]
    fn test_defaults_config_backward_compatibility_filter_non_english() -> Result<()> {
        // Given: Configuration TOML without filter_non_english field (backward compatibility)
        let toml_without_filter = r#"
            [defaults]
            refresh_hours = 24
            max_archives = 10
            fetch_enabled = true
            follow_links = "first_party"
            allowlist = []

            [paths]
            root = "/tmp/test"
        "#;

        // When: Deserializing old config
        let config: Config = toml::from_str(toml_without_filter)
            .map_err(|e| Error::Config(format!("Failed to parse: {e}")))?;

        // Then: Should use default value (true)
        assert!(config.defaults.filter_non_english);
        assert_eq!(config.defaults.refresh_hours, 24);

        Ok(())
    }

    #[test]
    fn test_index_config_backward_compatibility_filter_non_english() -> Result<()> {
        // Given: IndexConfig without filter_non_english field (backward compatibility)
        let config = IndexConfig {
            max_heading_block_lines: Some(500),
            filter_non_english: None,
        };

        // When: Serializing and deserializing
        let serialized = serde_json::to_string(&config)
            .map_err(|e| Error::Config(format!("Failed to serialize: {e}")))?;
        let deserialized: IndexConfig = serde_json::from_str(&serialized)
            .map_err(|e| Error::Config(format!("Failed to deserialize: {e}")))?;

        // Then: None should be preserved (uses global default)
        assert_eq!(deserialized.filter_non_english, None);
        assert_eq!(deserialized.max_heading_block_lines, Some(500));

        Ok(())
    }

    #[test]
    fn test_filter_non_english_serialization() -> Result<()> {
        // Given: Config with filter_non_english explicitly set to false
        let config = Config {
            defaults: DefaultsConfig {
                refresh_hours: 24,
                max_archives: 10,
                fetch_enabled: true,
                follow_links: FollowLinks::FirstParty,
                allowlist: vec![],
                filter_non_english: false,
            },
            paths: PathsConfig {
                root: PathBuf::from("/tmp"),
            },
        };

        // When: Serializing and deserializing
        let serialized = toml::to_string_pretty(&config)?;
        let deserialized: Config = toml::from_str(&serialized)?;

        // Then: false value should be preserved
        assert!(!deserialized.defaults.filter_non_english);

        Ok(())
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_config_refresh_hours_roundtrip(refresh_hours in 1u32..=365*24) {
            let config = Config {
                defaults: DefaultsConfig {
                    refresh_hours,
                    max_archives: 10,
                    fetch_enabled: true,
                    follow_links: FollowLinks::FirstParty,
                    allowlist: vec![],
                    filter_non_english: true,
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).expect("should serialize");
            let deserialized: Config = toml::from_str(&serialized).expect("should deserialize");

            prop_assert_eq!(deserialized.defaults.refresh_hours, refresh_hours);
        }

        #[test]
        fn test_config_max_archives_roundtrip(max_archives in 1usize..=1000) {
            let config = Config {
                defaults: DefaultsConfig {
                    refresh_hours: 24,
                    max_archives,
                    fetch_enabled: true,
                    follow_links: FollowLinks::FirstParty,
                    allowlist: vec![],
                    filter_non_english: true,
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).expect("should serialize");
            let deserialized: Config = toml::from_str(&serialized).expect("should deserialize");

            prop_assert_eq!(deserialized.defaults.max_archives, max_archives);
        }

        #[test]
        fn test_config_allowlist_roundtrip(allowlist in prop::collection::vec(r"[a-z0-9\.-]+", 0..=10)) {
            let config = Config {
                defaults: DefaultsConfig {
                    refresh_hours: 24,
                    max_archives: 10,
                    fetch_enabled: true,
                    follow_links: FollowLinks::Allowlist,
                    allowlist: allowlist.clone(),
                    filter_non_english: true,
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).expect("should serialize");
            let deserialized: Config = toml::from_str(&serialized).expect("should deserialize");

            prop_assert_eq!(deserialized.defaults.allowlist, allowlist);
        }
    }

    /*
    // Security-focused tests
    #[test]
    fn test_config_path_traversal_prevention() {
            // Given: Config with potentially malicious paths
            let malicious_paths = vec![
                "../../../etc/passwd",
                "..\\..\\..\\windows\\system32",
                "/etc/shadow",
                "../../.ssh/id_rsa",
            ];

            for malicious_path in malicious_paths {
                // When: Creating config with malicious path
                let config = Config {
                    defaults: DefaultsConfig {
                        refresh_hours: 24,
                        max_archives: 10,
                        fetch_enabled: true,
                        follow_links: FollowLinks::FirstParty,
                        allowlist: vec![],
                    },
                    paths: PathsConfig {
                        root: PathBuf::from(malicious_path),
                    },
                };

                // Then: Should still serialize/deserialize (path validation is separate)
                let serialized = toml::to_string_pretty(&config).expect("should serialize");
                let deserialized: Config = toml::from_str(&serialized).expect("should deserialize");
                assert_eq!(deserialized.paths.root, PathBuf::from(malicious_path));
            }
        }

        #[test]
        fn test_config_malicious_toml_injection() {
            // Given: Potentially malicious TOML strings that could break parsing
            let malicious_strings = vec![
                "\n[malicious]\nkey = \"value\"",
                "\"quotes\"in\"weird\"places",
                "key = \"value\"\n[new_section]",
                "unicode = \"\\u0000\\u0001\\u0002\"",
            ];

            for malicious_string in malicious_strings {
                // When: Setting allowlist with potentially malicious content
                let config = Config {
                    defaults: DefaultsConfig {
                        refresh_hours: 24,
                        max_archives: 10,
                        fetch_enabled: true,
                        follow_links: FollowLinks::Allowlist,
                        allowlist: vec![malicious_string.to_string()],
                    },
                    paths: PathsConfig {
                        root: PathBuf::from("/tmp"),
                    },
                };

                // Then: Should serialize safely (TOML library handles escaping)
                let result = toml::to_string_pretty(&config);
                assert!(
                    result.is_ok(),
                    "Failed to serialize config with: {malicious_string}"
                );

                if let Ok(serialized) = result {
                    let deserialized_result: std::result::Result<Config, _> =
                        toml::from_str(&serialized);
                    assert!(
                        deserialized_result.is_ok(),
                        "Failed to deserialize config with: {malicious_string}"
                    );
                }
            }
        }

        #[test]
        fn test_config_unicode_handling() -> Result<()> {
            // Given: Configuration with Unicode content
            let unicode_config = Config {
                defaults: DefaultsConfig {
                    refresh_hours: 24,
                    max_archives: 10,
                    fetch_enabled: true,
                    follow_links: FollowLinks::Allowlist,
                    allowlist: vec![
                        "例え.com".to_string(),    // Japanese
                        "مثال.com".to_string(),    // Arabic
                        "пример.com".to_string(),  // Cyrillic
                        "🚀.test.com".to_string(), // Emoji
                    ],
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp/测试"), // Chinese characters
                },
            };

            // When: Serializing and deserializing
            let serialized = toml::to_string_pretty(&unicode_config)?;
            let deserialized: Config = toml::from_str(&serialized)?;

            // Then: Unicode should be preserved correctly
            assert_eq!(deserialized.defaults.allowlist.len(), 4);
            assert!(
                deserialized
                    .defaults
                    .allowlist
                    .contains(&"例え.com".to_string())
            );
            assert!(
                deserialized
                    .defaults
                    .allowlist
                    .contains(&"🚀.test.com".to_string())
            );
            assert_eq!(deserialized.paths.root, PathBuf::from("/tmp/测试"));

            Ok(())
        }

        #[test]
        fn test_config_edge_case_empty_values() -> Result<()> {
            // Given: Configuration with empty values
            let empty_config = Config {
                defaults: DefaultsConfig {
                    refresh_hours: 0, // Edge case: zero refresh
                    max_archives: 0,  // Edge case: no archives
                    fetch_enabled: false,
                    follow_links: FollowLinks::None,
                    allowlist: vec![String::new()], // Empty string in allowlist
                },
                paths: PathsConfig {
                    root: PathBuf::from(""), // Empty path
                },
            };

            // When: Serializing and deserializing
            let serialized = toml::to_string_pretty(&empty_config)?;
            let deserialized: Config = toml::from_str(&serialized)?;

            // Then: Empty/zero values should be handled correctly
            assert_eq!(deserialized.defaults.refresh_hours, 0);
            assert_eq!(deserialized.defaults.max_archives, 0);
            assert_eq!(deserialized.defaults.allowlist.len(), 1);
            assert_eq!(deserialized.defaults.allowlist[0], "");
            assert_eq!(deserialized.paths.root, PathBuf::from(""));

            Ok(())
        }
    }
    */
}
