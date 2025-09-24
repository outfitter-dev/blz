//! Release automation utilities for the blz project.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use toml_edit::{DocumentMut, value};

#[derive(Parser, Debug)]
#[command(author, version, about = "Release tooling for the blz workspace")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compute the next semantic version
    Next(NextArgs),
    /// Synchronise npm manifests with a version
    Sync(SyncArgs),
    /// Verify npm manifests match an expected version
    Check(CheckArgs),
    /// Update Cargo.lock package entries
    UpdateLock(UpdateLockArgs),
}

#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum Mode {
    Patch,
    Minor,
    Major,
    Canary,
    Set,
}

#[derive(Parser, Debug)]
struct NextArgs {
    #[arg(long)]
    mode: Mode,
    #[arg(long)]
    current: Version,
    #[arg(long)]
    value: Option<Version>,
    #[arg(long)]
    meta: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    write_meta: bool,
}

#[derive(Parser, Debug)]
struct SyncArgs {
    #[arg(long)]
    version: Version,
    #[arg(long, value_name = "PATH")]
    repo_root: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CheckArgs {
    #[arg(long)]
    expect: Version,
    #[arg(long, value_name = "PATH")]
    repo_root: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UpdateLockArgs {
    #[arg(long)]
    version: Version,
    #[arg(long, value_name = "PATH", default_value = "Cargo.lock")]
    lock_path: PathBuf,
    #[arg(long = "package", value_name = "NAME", num_args = 1.., default_values_t = vec!["blz-cli".to_owned(), "blz-core".to_owned()])]
    packages: Vec<String>,
}

/// CLI entry point: parse arguments, execute the selected command, and return a Result.
///
/// This function parses command-line arguments into `Cli` and dispatches to the corresponding
/// command handler:
/// - `Next` — computes the next version, writes it to stdout (no trailing newline guaranteed).
/// - `Sync` — synchronizes npm manifest files with the provided version.
/// - `Check` — verifies npm manifest versions match the expected version.
/// - `UpdateLock` — updates specified entries in `Cargo.lock`.
///
/// Returns `Ok(())` on success or an `anyhow::Error` on failure.
///
/// # Examples
///
/// ```no_run
/// // Run the program entry point (no_run prevents doctest from executing it).
/// assert!(crate::main().is_ok());
/// ```
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Next(args) => {
            let next = compute_next_version(args)?;
            print!("{next}");
            std::io::stdout().flush().ok();
        },
        Command::Sync(args) => sync_npm_files(&args.version, args.repo_root.as_deref())?,
        Command::Check(args) => check_npm_files(&args.expect, args.repo_root.as_deref())?,
        Command::UpdateLock(args) => {
            update_cargo_lock(&args.version, &args.lock_path, &args.packages)?
        },
    }
    Ok(())
}

/// Compute the next semantic version according to the provided NextArgs.
///
/// The returned version is always normalized to have empty pre-release and build
/// metadata before applying the selected bump strategy.
///
/// Behavior by mode:
/// - Patch/Minor/Major: returns the corresponding semantic bump (clears pre/build).
/// - Set: returns the provided `value`; errors if `--value` is missing or not
///   greater than the current version.
/// - Canary: produces a pre-release `canary.<n>` where `n` is derived from the
///   Canary meta file (or starts at 1). If `write_meta` is true the meta file
///   is updated with the new last-canary entry.
///
/// Errors:
/// - Returns an error if `Mode::Set` is selected but `value` is missing or not
///   strictly greater than `current`.
/// - Propagates I/O and parse errors when reading/writing the canary meta file
///   or when parsing generated pre-release identifiers.
///
/// # Examples
///
/// ```
/// use semver::Version;
///
/// let args = NextArgs {
///     mode: Mode::Patch,
///     current: Version::parse("1.2.3").unwrap(),
///     value: None,
///     meta: None,
///     write_meta: false,
/// };
/// let next = compute_next_version(args).unwrap();
/// assert_eq!(next, Version::parse("1.2.4").unwrap());
/// ```
fn compute_next_version(args: NextArgs) -> Result<Version> {
    let mut current = args.current.clone();
    current.pre = semver::Prerelease::EMPTY;
    current.build = semver::BuildMetadata::EMPTY;

    let next = match args.mode {
        Mode::Patch => bump_patch(&current),
        Mode::Minor => bump_minor(&current),
        Mode::Major => bump_major(&current),
        Mode::Set => {
            let value = args
                .value
                .ok_or_else(|| anyhow::anyhow!("--value is required when mode=set"))?;
            ensure!(
                value > args.current,
                "Target version {value} must be greater than current {}",
                args.current
            );
            value
        },
        Mode::Canary => {
            let base = current.clone();
            let mut meta = read_meta(args.meta.as_deref())?;
            let base_id = format!("{}.{}.{}", base.major, base.minor, base.patch);
            let next_sequence = match meta.last_canary {
                Some(ref last) if last.base == base_id => last.sequence + 1,
                _ => 1,
            };
            let mut v = base;
            v.pre = format!("canary.{next_sequence}").parse()?;
            if args.write_meta {
                meta.last_canary = Some(CanaryMeta {
                    base: base_id,
                    sequence: next_sequence,
                    last_updated: OffsetDateTime::now_utc().unix_timestamp(),
                });
                write_meta(args.meta.as_deref(), &meta)?;
            }
            v
        },
    };

    Ok(next)
}

/// Bumps the patch component of a semantic `Version`, clearing any pre-release identifier.
///
/// The returned `Version` has `patch` incremented by one, while `major` and `minor` are preserved.
/// Any pre-release component is removed (set to empty); build metadata is left unchanged.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// let v = Version::parse("1.2.3-alpha.1").unwrap();
/// let next = bump_patch(&v);
/// assert_eq!(next.major, 1);
/// assert_eq!(next.minor, 2);
/// assert_eq!(next.patch, 4);
/// assert!(next.pre.is_empty());
/// ```
fn bump_patch(current: &Version) -> Version {
    let mut next = current.clone();
    next.patch += 1;
    next.pre = semver::Prerelease::EMPTY;
    next
}

/// Bump the minor version, reset patch to 0, and clear prerelease/build metadata.
///
/// The returned `Version` has `minor` incremented by one, `patch` set to `0`,
/// and `pre` cleared. `build` metadata (if any) is not preserved.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// let current = Version::new(1, 2, 3);
/// let next = bump_minor(&current);
/// assert_eq!(next, Version::new(1, 3, 0));
///
/// // prerelease is cleared
/// let mut with_pre = Version::new(2, 4, 5);
/// with_pre.pre = semver::Prerelease::new("alpha.1").unwrap();
/// let next2 = bump_minor(&with_pre);
/// assert!(next2.pre.is_empty());
/// assert_eq!(next2, Version::new(2, 5, 0));
/// ```
fn bump_minor(current: &Version) -> Version {
    let mut next = current.clone();
    next.minor += 1;
    next.patch = 0;
    next.pre = semver::Prerelease::EMPTY;
    next
}

/// Bumps the major version, resetting minor and patch and clearing pre-release/build metadata.
///
/// Returns a new `Version` with `major` incremented by 1, `minor` and `patch` set to 0,
/// and `pre` cleared.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// let v = Version::new(1, 2, 3);
/// let next = bump_major(&v);
/// assert_eq!(next.major, 2);
/// assert_eq!(next.minor, 0);
/// assert_eq!(next.patch, 0);
/// assert!(next.pre.is_empty());
/// ```
fn bump_major(current: &Version) -> Version {
    let mut next = current.clone();
    next.major += 1;
    next.minor = 0;
    next.patch = 0;
    next.pre = semver::Prerelease::EMPTY;
    next
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MetaFile {
    #[serde(rename = "lastCanary")]
    last_canary: Option<CanaryMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CanaryMeta {
    base: String,
    sequence: u64,
    #[serde(default, rename = "lastUpdated")]
    last_updated: i64,
}

/// Reads a Canary meta file (JSON) from `path` and returns its deserialized `MetaFile`.
///
/// If `path` is `None` or the file does not exist, this returns `MetaFile::default()`.
/// I/O or JSON parse errors are returned as `anyhow::Error` with context identifying the path.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// // When no path is provided, the default meta is returned.
/// let meta = crate::read_meta(None).unwrap();
/// assert!(meta.last_canary.is_none());
///
/// // Example (non-exhaustive): to read from a real file:
/// // let meta = crate::read_meta(Some(Path::new("meta.json"))).unwrap();
/// ```
fn read_meta(path: Option<&Path>) -> Result<MetaFile> {
    let Some(path) = path else {
        return Ok(MetaFile::default());
    };
    if !path.exists() {
        return Ok(MetaFile::default());
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read meta file {}", path.display()))?;
    let meta = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse meta file {}", path.display()))?;
    Ok(meta)
}

/// Write `meta` as pretty-printed JSON to `path`.
///
/// If `path` is `None`, the function is a no-op and returns `Ok(())`. When a
/// path is provided, `meta` is serialized with pretty formatting and written
/// to the file with a trailing newline. Errors are returned for serialization
/// or I/O failures; the error context includes the target path.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// let meta = MetaFile::default();
/// // no-op when no path is provided
/// assert!(write_meta(None, &meta).is_ok());
/// // writes to `meta.json` when a path is provided
/// let _ = write_meta(Some(Path::new("meta.json")), &meta);
/// ```
fn write_meta(path: Option<&Path>, meta: &MetaFile) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let json = serde_json::to_string_pretty(meta)?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("Failed to write meta file {}", path.display()))
}

/// Synchronizes npm manifest versions in a repository to the provided semantic `version`.
///
/// Updates `package.json` in `repo_root` (defaults to current directory) to set its `version` field,
/// then updates `package-lock.json` if the lockfile needs a corresponding change and writes it back.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// use std::path::Path;
///
/// let v = Version::parse("1.2.3").unwrap();
/// // Update manifests in the current working directory:
/// sync_npm_files(&v, None).unwrap();
///
/// // Update manifests in a specific repository root:
/// sync_npm_files(&v, Some(Path::new("/path/to/repo"))).unwrap();
/// ```
fn sync_npm_files(version: &Version, repo_root: Option<&Path>) -> Result<()> {
    let root = repo_root.map_or_else(|| Path::new(".").to_path_buf(), ToOwned::to_owned);
    update_json_version(root.join("package.json"), version)?;
    if let Some(lock) = update_package_lock(root.join("package-lock.json"), version)? {
        fs::write(lock.path, lock.contents)?;
    }
    Ok(())
}

/// Update the "version" field in a JSON file (if the file exists).
///
/// If the file at `path` does not exist this is a no-op and returns `Ok(())`.
/// If the file exists it is parsed as JSON, the top-level `version` string is
/// set to the provided `version`, and the file is written back using pretty
/// JSON formatting (with a trailing newline).
///
/// Returns an error if reading, parsing, serializing, or writing the file fails,
/// with context indicating the failing path.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use std::path::PathBuf;
/// use semver::Version;
///
/// let tmp = std::env::temp_dir();
/// let path = tmp.join(format!("update_json_version_example_{}.json", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
/// fs::write(&path, r#"{"name":"example","version":"0.0.0"}"#).unwrap();
///
/// let v = Version::parse("1.2.3").unwrap();
/// update_json_version(path.clone(), &v).unwrap();
///
/// let out = fs::read_to_string(&path).unwrap();
/// assert!(out.contains(r#""version": "1.2.3""#));
/// fs::remove_file(&path).ok();
/// ```
fn update_json_version(path: PathBuf, version: &Version) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut json: JsonValue = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("Failed to parse JSON file {}", path.display()))?;
    if let Some(obj) = json.as_object_mut() {
        obj.insert("version".into(), JsonValue::String(version.to_string()));
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&json)?))
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    Ok(())
}

struct LockUpdate {
    path: PathBuf,
    contents: String,
}

/// Update a npm `package-lock.json` file's version fields and return the new contents.
///
/// If `path` does not exist, this returns `Ok(None)`. If the file is present and parses as a
/// JSON object, the function sets the top-level `"version"` field to `version` and, if the
/// `"packages"` object contains a root entry (`""`), sets its `"version"` field as well. When an
/// update is performed the function returns `Ok(Some(LockUpdate { path, contents }))` where
/// `contents` is the pretty-printed JSON with a trailing newline.
///
/// IO and JSON parsing errors are propagated as `Err`.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use semver::Version;
///
/// // Given a package-lock.json at ./package-lock.json, produce an updated contents buffer.
/// let path = PathBuf::from("./package-lock.json");
/// let ver = Version::parse("1.2.3").unwrap();
/// let result = update_package_lock(path.clone(), &ver).unwrap();
/// if let Some(update) = result {
///     assert_eq!(update.path, path);
///     assert!(update.contents.contains("\"version\": \"1.2.3\""));
/// }
/// ```
fn update_package_lock(path: PathBuf, version: &Version) -> Result<Option<LockUpdate>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut json: JsonValue = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("Failed to parse package-lock.json at {}", path.display()))?;
    if let Some(obj) = json.as_object_mut() {
        obj.insert("version".into(), JsonValue::String(version.to_string()));
        if let Some(packages) = obj.get_mut("packages").and_then(|v| v.as_object_mut()) {
            if let Some(root) = packages.get_mut("").and_then(|v| v.as_object_mut()) {
                root.insert("version".into(), JsonValue::String(version.to_string()));
            }
        }
        let contents = format!("{}\n", serde_json::to_string_pretty(&json)?);
        return Ok(Some(LockUpdate { path, contents }));
    }
    Ok(None)
}

/// Verify that npm manifest files in the repository match the expected version.
///
/// This checks `package.json` and `package-lock.json` (in that order) under `repo_root`
/// (or the current working directory when `repo_root` is `None`) and returns an error
/// if either file exists but contains a differing `version` value or if parsing fails.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// # use anyhow::Result;
/// # fn example() -> Result<()> {
/// let expected = Version::parse("1.2.3")?;
/// // Check files in the current directory:
/// check_npm_files(&expected, None)?;
/// # Ok(()) }
/// ```
fn check_npm_files(expected: &Version, repo_root: Option<&Path>) -> Result<()> {
    let root = repo_root.map_or_else(|| Path::new(".").to_path_buf(), ToOwned::to_owned);
    check_json_version(root.join("package.json"), expected)?;
    check_package_lock(root.join("package-lock.json"), expected)?;
    Ok(())
}

/// Check that a JSON file's top-level "version" field matches an expected semantic version.
///
/// If the file does not exist, this function succeeds (no-op). Otherwise it reads and parses
/// the file as JSON, verifies a string-valued top-level `"version"` field is present, and
/// ensures it equals `expected`. Returns an error if the file cannot be read/parsed, the
/// `version` field is missing/non-string, or the value does not match `expected`.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use std::path::PathBuf;
/// use semver::Version;
/// # fn try_main() -> anyhow::Result<()> {
/// let tmp = tempfile::NamedTempFile::new()?.into_temp_path();
/// let path = PathBuf::from(&tmp);
/// fs::write(&path, r#"{ "version": "1.2.3" }"#)?;
/// let expected = Version::parse("1.2.3")?;
/// check_json_version(path, &expected)?;
/// # Ok(()) }
/// ```
fn check_json_version(path: PathBuf, expected: &Version) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let json: JsonValue = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    let Some(actual) = json.get("version").and_then(JsonValue::as_str) else {
        bail!("{} missing version field", path.display());
    };
    ensure!(
        actual == expected.to_string(),
        "{} version {} does not match {}",
        path.display(),
        actual,
        expected
    );
    Ok(())
}

/// Validates that a package-lock.json at `path` reports the given `expected` version.
///
/// - If `path` does not exist, this is a no-op and returns Ok(()).
/// - Fails if the top-level `version` field is missing or does not equal `expected`.
/// - If a root package entry exists at `packages[""]` and it contains a `version` field,
///   that version must also equal `expected` or the function will return an error.
///
/// The function returns an anyhow::Result, so errors include context for I/O and JSON parsing.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use semver::Version;
///
/// let expected = Version::parse("1.2.3").unwrap();
/// let path = PathBuf::from("package-lock.json");
/// // Returns Ok(()) when file is absent or when versions match; otherwise returns an error.
/// let _ = check_package_lock(path, &expected);
/// ```
fn check_package_lock(path: PathBuf, expected: &Version) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let json: JsonValue = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    let version = json
        .get("version")
        .and_then(JsonValue::as_str)
        .context("package-lock.json missing version field")?;
    ensure!(
        version == expected.to_string(),
        "package-lock.json version {} does not match {}",
        version,
        expected
    );
    if let Some(root_version) = json
        .get("packages")
        .and_then(JsonValue::as_object)
        .and_then(|packages| packages.get(""))
        .and_then(JsonValue::as_object)
        .and_then(|root| root.get("version"))
        .and_then(JsonValue::as_str)
    {
        ensure!(
            root_version == expected.to_string(),
            "Root entry in package-lock.json is {}, expected {}",
            root_version,
            expected
        );
    }
    Ok(())
}

/// Update the versions of specified packages in a Cargo.lock file.
///
/// Reads the Cargo.lock at `path`, locates each `[[package]]` entry whose `name`
/// matches an item in `packages`, and sets its `version` field to `version`.
/// Writes the modified lockfile back to `path`.
///
/// Returns an error if the lockfile cannot be read or parsed, if writing fails,
/// or if any package from `packages` is not present as a `[[package]]` entry in
/// the lockfile.
///
/// # Examples
///
/// ```
/// use semver::Version;
/// use std::path::Path;
///
/// // Update blz-core and blz-cli entries in Cargo.lock to 1.2.3
/// let v = Version::parse("1.2.3").unwrap();
/// let path = Path::new("Cargo.lock");
/// let packages = vec!["blz-core".to_string(), "blz-cli".to_string()];
/// // `update_cargo_lock` returns Result<(), anyhow::Error>
/// let _ = crate::update_cargo_lock(&v, path, &packages);
/// ```
fn update_cargo_lock(version: &Version, path: &Path, packages: &[String]) -> Result<()> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read Cargo.lock at {}", path.display()))?;
    let mut doc: DocumentMut = contents.parse().context("Failed to parse Cargo.lock")?;
    let mut to_update: BTreeMap<&str, bool> =
        packages.iter().map(|p| (p.as_str(), false)).collect();

    let pkg_item = doc
        .as_table_mut()
        .get_mut("package")
        .context("Cargo.lock missing [[package]] array")?;

    let array = pkg_item
        .as_array_of_tables_mut()
        .context("Cargo.lock package entry is not an array of tables")?;

    for table in array.iter_mut() {
        let Some(name_item) = table.get("name") else {
            continue;
        };
        let Some(name) = name_item.as_str() else {
            continue;
        };
        if let Some(flag) = to_update.get_mut(name) {
            table["version"] = value(version.to_string());
            *flag = true;
        }
    }

    let missing: Vec<_> = to_update
        .iter()
        .filter_map(|(name, done)| if !done { Some(*name) } else { None })
        .collect();
    if !missing.is_empty() {
        bail!(
            "Failed to update Cargo.lock: missing package entries for {}",
            missing.join(", ")
        );
    }

    fs::write(path, doc.to_string())
        .with_context(|| format!("Failed to write updated Cargo.lock to {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_patch_clears_prerelease() {
        let args = NextArgs {
            mode: Mode::Patch,
            current: Version::parse("1.2.3-beta.1").unwrap(),
            value: None,
            meta: None,
            write_meta: false,
        };
        let next = compute_next_version(args).unwrap();
        assert_eq!(next, Version::parse("1.2.4").unwrap());
    }

    #[test]
    fn next_canary_increments_meta() {
        let temp = tempfile::tempdir().unwrap();
        let meta_path = temp.path().join("meta.json");
        let args = NextArgs {
            mode: Mode::Canary,
            current: Version::parse("0.3.1").unwrap(),
            value: None,
            meta: Some(meta_path.clone()),
            write_meta: true,
        };
        let v1 = compute_next_version(args).unwrap();
        assert_eq!(v1.to_string(), "0.3.1-canary.1");

        let args = NextArgs {
            mode: Mode::Canary,
            current: Version::parse("0.3.1").unwrap(),
            value: None,
            meta: Some(meta_path.clone()),
            write_meta: true,
        };
        let v2 = compute_next_version(args).unwrap();
        assert_eq!(v2.to_string(), "0.3.1-canary.2");
    }

    #[test]
    fn update_lock_updates_packages() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join("Cargo.lock");
        fs::write(
            &lock_path,
            r#"version = 3

[[package]]
name = "blz-cli"
version = "0.3.1"

[[package]]
name = "blz-core"
version = "0.3.1"
"#,
        )
        .unwrap();

        update_cargo_lock(
            &Version::parse("0.3.2").unwrap(),
            &lock_path,
            &["blz-cli".into(), "blz-core".into()],
        )
        .unwrap();
        let updated = fs::read_to_string(lock_path).unwrap();
        assert!(updated.contains("name = \"blz-cli\""));
        assert!(updated.contains("name = \"blz-core\""));
        assert!(updated.contains("version = \"0.3.2\""));
    }

    #[test]
    fn update_lock_missing_package_errors() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join("Cargo.lock");
        fs::write(
            &lock_path,
            r#"version = 3

[[package]]
name = "other"
version = "0.3.1"
"#,
        )
        .unwrap();

        let err = update_cargo_lock(
            &Version::parse("0.3.2").unwrap(),
            &lock_path,
            &["blz-cli".into()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("blz-cli"));
    }
}
