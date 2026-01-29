//! CLI detection and version validation for Firecrawl.
//!
//! This module provides functionality to detect the Firecrawl CLI installation,
//! validate its version meets minimum requirements, and check authentication status.

use crate::{Error, Result};
use regex::Regex;
use semver::Version;
use std::sync::OnceLock;
use tokio::process::Command;
use tracing::instrument;

use super::MIN_VERSION;

/// Status of Firecrawl CLI detection.
///
/// Represents the various states the Firecrawl CLI can be in,
/// from fully ready to not installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirecrawlStatus {
    /// Firecrawl is installed, meets version requirements, and is authenticated.
    Ready {
        /// Detected version of Firecrawl CLI.
        version: Version,
        /// Path to the Firecrawl executable.
        path: String,
    },
    /// Firecrawl is installed but version is too old.
    VersionTooOld {
        /// Version that was found.
        found: Version,
        /// Minimum required version.
        required: Version,
        /// Path to the Firecrawl executable.
        path: String,
    },
    /// Firecrawl is installed but not authenticated.
    NotAuthenticated {
        /// Detected version of Firecrawl CLI.
        version: Version,
        /// Path to the Firecrawl executable.
        path: String,
    },
    /// Firecrawl CLI is not installed or not found in PATH.
    NotInstalled,
}

/// Handle to a detected Firecrawl CLI installation.
///
/// This struct represents a validated Firecrawl CLI that meets
/// minimum version requirements. Use [`FirecrawlCli::detect`] to
/// create an instance.
#[derive(Debug, Clone)]
pub struct FirecrawlCli {
    path: String,
    version: Version,
}

impl FirecrawlCli {
    /// Detect and validate Firecrawl CLI installation.
    ///
    /// Searches for `firecrawl` in PATH, validates version meets
    /// minimum requirements, but does NOT check authentication.
    /// Use [`is_authenticated`] to check auth status separately.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Firecrawl is not installed or not in PATH
    /// - Version cannot be determined
    /// - Version is below minimum required
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn example() -> blz_core::Result<()> {
    /// use blz_core::firecrawl::FirecrawlCli;
    ///
    /// let cli = FirecrawlCli::detect().await?;
    /// println!("Found Firecrawl {} at {}", cli.version(), cli.path());
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(level = "debug")]
    pub async fn detect() -> Result<Self> {
        // First, check if firecrawl is in PATH
        let path = find_firecrawl_path().await?;

        // Get version
        let version = get_firecrawl_version(&path).await?;

        // Validate version meets minimum requirements
        let min_version = Version::parse(MIN_VERSION)
            .map_err(|e| Error::Config(format!("Invalid MIN_VERSION constant: {e}")))?;

        if version < min_version {
            return Err(Error::Config(format!(
                "Firecrawl version {version} is below minimum required {min_version}"
            )));
        }

        Ok(Self { path, version })
    }

    /// Returns the path to the Firecrawl executable.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the detected version of Firecrawl.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Check if Firecrawl is authenticated with valid API credentials.
    ///
    /// Runs `firecrawl whoami` to verify authentication status.
    ///
    /// # Errors
    ///
    /// Returns an error if the auth check command fails to execute.
    #[instrument(level = "debug", skip(self))]
    pub async fn is_authenticated(&self) -> Result<bool> {
        check_firecrawl_auth(&self.path).await
    }
}

/// Detect Firecrawl CLI and return comprehensive status.
///
/// This is the main entry point for checking Firecrawl availability.
/// It performs all checks and returns a status enum indicating the
/// exact state of the installation.
///
/// # Examples
///
/// ```rust,no_run
/// use blz_core::firecrawl::{detect_firecrawl, FirecrawlStatus};
///
/// # async fn example() {
/// match detect_firecrawl().await {
///     FirecrawlStatus::Ready { version, path } => {
///         println!("Ready: {} at {}", version, path);
///     }
///     FirecrawlStatus::NotInstalled => {
///         println!("Please install Firecrawl CLI");
///     }
///     _ => {}
/// }
/// # }
/// ```
#[instrument(level = "debug")]
pub async fn detect_firecrawl() -> FirecrawlStatus {
    // Check if firecrawl is in PATH
    let Ok(path) = find_firecrawl_path().await else {
        return FirecrawlStatus::NotInstalled;
    };

    // Get version
    let Ok(version) = get_firecrawl_version(&path).await else {
        return FirecrawlStatus::NotInstalled;
    };

    // Check version meets minimum
    let Ok(min_version) = Version::parse(MIN_VERSION) else {
        return FirecrawlStatus::NotInstalled;
    };

    if version < min_version {
        return FirecrawlStatus::VersionTooOld {
            found: version,
            required: min_version,
            path,
        };
    }

    // Check authentication
    match check_firecrawl_auth(&path).await {
        Ok(true) => FirecrawlStatus::Ready { version, path },
        Ok(false) | Err(_) => FirecrawlStatus::NotAuthenticated { version, path },
    }
}

/// Find the path to the firecrawl executable.
async fn find_firecrawl_path() -> Result<String> {
    // Try `which firecrawl` on Unix, `where firecrawl` on Windows
    #[cfg(windows)]
    let which_cmd = "where";
    #[cfg(not(windows))]
    let which_cmd = "which";

    let output = Command::new(which_cmd)
        .arg("firecrawl")
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::NotFound("firecrawl not found in PATH".to_string()));
    }

    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .ok_or_else(|| Error::NotFound("firecrawl not found in PATH".to_string()))?
        .trim()
        .to_string();

    if path.is_empty() {
        return Err(Error::NotFound("firecrawl not found in PATH".to_string()));
    }

    Ok(path)
}

/// Get the version of firecrawl by running `firecrawl --version`.
async fn get_firecrawl_version(path: &str) -> Result<Version> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::Other("Failed to get firecrawl version".to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout)
}

/// Parse version string from firecrawl --version output.
///
/// Handles various formats:
/// - "firecrawl 1.2.3"
/// - "Firecrawl CLI v1.1.0"
/// - "1.2.3"
fn parse_version(output: &str) -> Result<Version> {
    // Regex to match version patterns: X.Y.Z (with optional v prefix)
    // The regex pattern is a compile-time constant, so unwrap is safe here.
    static VERSION_RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    let re = VERSION_RE
        .get_or_init(|| Regex::new(r"v?(\d+\.\d+\.\d+)").expect("version regex is valid"));

    let captures = re.captures(output).ok_or_else(|| {
        Error::Parse(format!(
            "Could not parse version from firecrawl output: {output}"
        ))
    })?;

    let version_str = captures.get(1).map_or("", |m| m.as_str());

    Version::parse(version_str)
        .map_err(|e| Error::Parse(format!("Invalid version format '{version_str}': {e}")))
}

/// Check if firecrawl is authenticated by running `firecrawl whoami`.
async fn check_firecrawl_auth(path: &str) -> Result<bool> {
    let output = Command::new(path)
        .arg("whoami")
        .output()
        .await
        .map_err(Error::Io)?;

    // Exit code 0 means authenticated
    Ok(output.status.success())
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_macros
)]
mod tests {
    use super::*;

    // ============================================================
    // Version Parsing Tests
    // ============================================================

    #[test]
    fn test_version_parsing_simple() {
        // Simple version format: "1.2.3"
        let result = parse_version("1.2.3");
        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_parsing_with_name() {
        // Format: "firecrawl 1.2.3"
        let result = parse_version("firecrawl 1.2.3");
        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn test_version_parsing_with_prefix() {
        // Format: "Firecrawl CLI v1.1.0"
        let result = parse_version("Firecrawl CLI v1.1.0");
        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version, Version::new(1, 1, 0));
    }

    #[test]
    fn test_version_parsing_with_newline() {
        // Output might have trailing newline
        let result = parse_version("firecrawl 2.0.1\n");
        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version, Version::new(2, 0, 1));
    }

    #[test]
    fn test_version_parsing_invalid() {
        // Invalid version strings should return errors
        let result = parse_version("no version here");
        assert!(result.is_err());

        let result = parse_version("");
        assert!(result.is_err());

        let result = parse_version("1.2");
        assert!(result.is_err());
    }

    // ============================================================
    // Version Validation Tests
    // ============================================================

    #[test]
    fn test_rejects_old_version() {
        // Version 1.0.0 should fail validation (< 1.1.0)
        let old_version = Version::new(1, 0, 0);
        let min_version = Version::parse(MIN_VERSION).unwrap();

        assert!(
            old_version < min_version,
            "Version 1.0.0 should be less than minimum {MIN_VERSION}"
        );
    }

    #[test]
    fn test_accepts_minimum_version() {
        // Version 1.1.0 should pass validation (== MIN_VERSION)
        let version = Version::new(1, 1, 0);
        let min_version = Version::parse(MIN_VERSION).unwrap();

        assert!(
            version >= min_version,
            "Version 1.1.0 should meet minimum {MIN_VERSION}"
        );
    }

    #[test]
    fn test_accepts_newer_version() {
        // Version 2.0.0 should pass validation (> MIN_VERSION)
        let newer_version = Version::new(2, 0, 0);
        let min_version = Version::parse(MIN_VERSION).unwrap();

        assert!(
            newer_version >= min_version,
            "Version 2.0.0 should exceed minimum {MIN_VERSION}"
        );
    }

    #[test]
    fn test_accepts_patch_version() {
        // Version 1.1.5 should pass validation
        let patch_version = Version::new(1, 1, 5);
        let min_version = Version::parse(MIN_VERSION).unwrap();

        assert!(
            patch_version >= min_version,
            "Version 1.1.5 should meet minimum {MIN_VERSION}"
        );
    }

    #[test]
    fn test_rejects_lower_minor() {
        // Version 1.0.9 should fail (minor is less even though patch is higher)
        let version = Version::new(1, 0, 9);
        let min_version = Version::parse(MIN_VERSION).unwrap();

        assert!(
            version < min_version,
            "Version 1.0.9 should be less than minimum {MIN_VERSION}"
        );
    }

    // ============================================================
    // FirecrawlStatus Tests
    // ============================================================

    #[test]
    fn test_status_ready_equality() {
        let status1 = FirecrawlStatus::Ready {
            version: Version::new(1, 2, 3),
            path: "/usr/bin/firecrawl".to_string(),
        };
        let status2 = FirecrawlStatus::Ready {
            version: Version::new(1, 2, 3),
            path: "/usr/bin/firecrawl".to_string(),
        };
        assert_eq!(status1, status2);
    }

    #[test]
    fn test_status_not_installed_equality() {
        assert_eq!(FirecrawlStatus::NotInstalled, FirecrawlStatus::NotInstalled);
    }

    #[test]
    fn test_status_version_too_old_contains_versions() {
        let status = FirecrawlStatus::VersionTooOld {
            found: Version::new(1, 0, 0),
            required: Version::new(1, 1, 0),
            path: "/usr/bin/firecrawl".to_string(),
        };

        if let FirecrawlStatus::VersionTooOld {
            found,
            required,
            path,
        } = status
        {
            assert_eq!(found, Version::new(1, 0, 0));
            assert_eq!(required, Version::new(1, 1, 0));
            assert_eq!(path, "/usr/bin/firecrawl");
        } else {
            panic!("Expected VersionTooOld status");
        }
    }

    // ============================================================
    // FirecrawlCli Tests
    // ============================================================

    #[test]
    fn test_firecrawl_cli_accessors() {
        let cli = FirecrawlCli {
            path: "/usr/local/bin/firecrawl".to_string(),
            version: Version::new(1, 2, 0),
        };

        assert_eq!(cli.path(), "/usr/local/bin/firecrawl");
        assert_eq!(cli.version(), &Version::new(1, 2, 0));
    }

    // ============================================================
    // MIN_VERSION Constant Tests
    // ============================================================

    #[test]
    fn test_min_version_is_valid_semver() {
        let result = Version::parse(MIN_VERSION);
        assert!(
            result.is_ok(),
            "MIN_VERSION should be valid semver: {MIN_VERSION}"
        );
    }

    #[test]
    fn test_min_version_value() {
        let version = Version::parse(MIN_VERSION).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }
}
