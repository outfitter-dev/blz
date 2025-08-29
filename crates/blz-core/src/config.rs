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
//!     },
//! };
//!
//! // Save to file
//! tool_config.save(Path::new("react/settings.toml"))?;
//! # Ok::<(), blz_core::Error>(())
//! ```

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Global configuration for the blz cache system.
///
/// Contains default settings that apply to all sources unless overridden by per-source configuration.
/// Configuration is automatically loaded from the system config directory or created with sensible defaults.
///
/// ## File Location
///
/// The configuration file is stored at:
/// - Linux: `~/.config/outfitter/blz/global.toml`
/// - macOS: `~/Library/Preferences/outfitter.blz/global.toml`  
/// - Windows: `%APPDATA%\outfitter\cache\global.toml`
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
/// root = "/home/user/.outfitter/cache"
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
    /// - Linux: `~/.local/share/outfitter/blz`
    /// - macOS: `~/Library/Application Support/outfitter.blz`
    /// - Windows: `%APPDATA%\outfitter\cache`
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
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| Error::Config(format!("Failed to read config: {e}")))?;
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse config: {e}")))
        } else {
            Ok(Self::default())
        }
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
        let config_path = Self::config_path()?;
        let parent = config_path
            .parent()
            .ok_or_else(|| Error::Config("Invalid config path".into()))?;

        fs::create_dir_all(parent)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {e}")))?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {e}")))?;

        fs::write(&config_path, content)
            .map_err(|e| Error::Config(format!("Failed to write config: {e}")))?;

        Ok(())
    }

    /// Get the path where the global configuration file is stored.
    ///
    /// Uses the system-appropriate config directory based on the platform:
    /// - Linux: `~/.config/outfitter/blz/global.toml`
    /// - macOS: `~/Library/Preferences/outfitter.blz/global.toml`
    /// - Windows: `%APPDATA%\outfitter\blz\global.toml`
    ///
    /// # Errors
    ///
    /// Returns an error if the system config directory cannot be determined,
    /// which may happen on unsupported platforms or in sandboxed environments.
    fn config_path() -> Result<PathBuf> {
        let project_dirs = directories::ProjectDirs::from("dev", "outfitter", "blz")
            .ok_or_else(|| Error::Config("Failed to determine project directories".into()))?;

        let config_path = project_dirs.config_dir().join("global.toml");

        // Check for migration from old cache directory
        Self::check_and_migrate_old_config(&config_path)?;

        Ok(config_path)
    }

    /// Check if we need to migrate from the old cache config
    fn check_and_migrate_old_config(new_config_path: &Path) -> Result<()> {
        // Only migrate if new config doesn't exist
        if new_config_path.exists() {
            return Ok(());
        }

        // Try to find the old cache config
        if let Some(old_project_dirs) = directories::ProjectDirs::from("dev", "outfitter", "cache")
        {
            let old_config_path = old_project_dirs.config_dir().join("global.toml");

            if old_config_path.exists() {
                // Create parent directory if needed
                if let Some(parent) = new_config_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::Config(format!("Failed to create config directory: {e}"))
                    })?;
                }

                // Attempt to copy the old config
                match std::fs::copy(&old_config_path, new_config_path) {
                    Ok(_) => {
                        tracing::info!(
                            "Migrated config from {} to {}",
                            old_config_path.display(),
                            new_config_path.display()
                        );
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Failed to migrate config from {} to {}: {}",
                            old_config_path.display(),
                            new_config_path.display(),
                            e
                        );
                    },
                }
            }
        }

        Ok(())
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
                root: directories::ProjectDirs::from("dev", "outfitter", "blz").map_or_else(
                    || {
                        // Expand home directory properly
                        directories::BaseDirs::new().map_or_else(
                            || PathBuf::from(".outfitter/blz"),
                            |base| base.home_dir().join(".outfitter").join("blz"),
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
        if let Err(Error::Config(msg)) = result {
            assert!(msg.contains("Failed to read config"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_config_parse_invalid_toml() {
        // Given: Invalid TOML content
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");
        fs::write(&config_path, "this is not valid toml [[[").unwrap();

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
        assert!(nested_path.parent().unwrap().exists());

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
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).unwrap();
            let deserialized: Config = toml::from_str(&serialized).unwrap();

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
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).unwrap();
            let deserialized: Config = toml::from_str(&serialized).unwrap();

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
                },
                paths: PathsConfig {
                    root: PathBuf::from("/tmp"),
                },
            };

            let serialized = toml::to_string_pretty(&config).unwrap();
            let deserialized: Config = toml::from_str(&serialized).unwrap();

            prop_assert_eq!(deserialized.defaults.allowlist, allowlist);
        }
    }

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
            let serialized = toml::to_string_pretty(&config).unwrap();
            let deserialized: Config = toml::from_str(&serialized).unwrap();
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
