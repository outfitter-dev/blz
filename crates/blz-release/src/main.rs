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

/// Command-line interface definition for the release automation helper.
#[derive(Parser, Debug)]
#[command(author, version, about = "Release tooling for the blz workspace")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Top-level actions supported by the release automation helper.
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

/// Strategies for computing the next semantic version.
#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum Mode {
    Patch,
    Minor,
    Major,
    Canary,
    Set,
}

/// Arguments used for computing the next semantic version.
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

/// Arguments used to synchronise npm manifests with a version.
#[derive(Parser, Debug)]
struct SyncArgs {
    #[arg(long)]
    version: Version,
    #[arg(long, value_name = "PATH")]
    repo_root: Option<PathBuf>,
}

/// Arguments used for verifying npm manifests against the expected version.
#[derive(Parser, Debug)]
struct CheckArgs {
    #[arg(long)]
    expect: Version,
    #[arg(long, value_name = "PATH")]
    repo_root: Option<PathBuf>,
}

/// Arguments controlling how Cargo.lock entries should be updated.
#[derive(Parser, Debug)]
struct UpdateLockArgs {
    #[arg(long)]
    version: Version,
    #[arg(long, value_name = "PATH", default_value = "Cargo.lock")]
    lock_path: PathBuf,
    #[arg(
        long = "package",
        value_name = "NAME",
        num_args = 1..,
        default_values_t = vec![
            "blz-cli".to_owned(),
            "blz-core".to_owned(),
            "blz-release".to_owned(),
        ]
    )]
    packages: Vec<String>,
}

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
                value >= current,
                "Target version {value} must be at least current {current}"
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

fn bump_patch(current: &Version) -> Version {
    let mut next = current.clone();
    next.patch += 1;
    next.pre = semver::Prerelease::EMPTY;
    next
}

fn bump_minor(current: &Version) -> Version {
    let mut next = current.clone();
    next.minor += 1;
    next.patch = 0;
    next.pre = semver::Prerelease::EMPTY;
    next
}

fn bump_major(current: &Version) -> Version {
    let mut next = current.clone();
    next.major += 1;
    next.minor = 0;
    next.patch = 0;
    next.pre = semver::Prerelease::EMPTY;
    next
}

/// On-disk metadata persisted between canary builds.
#[derive(Debug, Default, Serialize, Deserialize)]
struct MetaFile {
    #[serde(rename = "lastCanary")]
    last_canary: Option<CanaryMeta>,
}

/// Metadata written to `.semver-meta.json` for canary sequencing.
#[derive(Debug, Serialize, Deserialize)]
struct CanaryMeta {
    base: String,
    sequence: u64,
    #[serde(default, rename = "lastUpdated")]
    last_updated: i64,
}

const PACKAGE_JSON: &str = "package.json";
const PACKAGE_LOCK_JSON: &str = "package-lock.json";

/// Read persisted canary metadata if a path is provided, falling back to defaults.
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

/// Write canary metadata back to disk when a backing file exists.
fn write_meta(path: Option<&Path>, meta: &MetaFile) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let json = serde_json::to_string_pretty(meta)?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("Failed to write meta file {}", path.display()))
}

/// Load and parse a JSON document, returning `None` if it does not exist.
fn read_json_file(path: &Path) -> Result<Option<JsonValue>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let value = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(value))
}

/// Serialise a JSON document with a trailing newline to stabilise diffs.
fn write_json_file(path: &Path, value: &JsonValue) -> Result<()> {
    let contents = format!("{}\n", serde_json::to_string_pretty(value)?);
    fs::write(path, contents).with_context(|| format!("Failed to write {}", path.display()))
}

fn sync_npm_files(version: &Version, repo_root: Option<&Path>) -> Result<()> {
    let root = repo_root.map_or_else(|| Path::new(".").to_path_buf(), ToOwned::to_owned);
    update_json_version(root.join(PACKAGE_JSON), version)?;
    if let Some(lock) = update_package_lock(root.join(PACKAGE_LOCK_JSON), version)? {
        fs::write(&lock.path, &lock.contents)
            .with_context(|| format!("Failed to write {}", lock.path.display()))?;
    }
    Ok(())
}

fn update_json_version(path: PathBuf, version: &Version) -> Result<()> {
    let Some(JsonValue::Object(mut json)) = read_json_file(&path)? else {
        return Ok(());
    };
    json.insert("version".into(), JsonValue::String(version.to_string()));
    write_json_file(&path, &JsonValue::Object(json))?;
    Ok(())
}

/// Buffer representing a pending package-lock update.
struct LockUpdate {
    path: PathBuf,
    contents: String,
}

fn update_package_lock(path: PathBuf, version: &Version) -> Result<Option<LockUpdate>> {
    let Some(mut json) = read_json_file(&path)? else {
        return Ok(None);
    };
    let JsonValue::Object(ref mut obj) = json else {
        bail!("package-lock.json was not an object");
    };
    obj.insert("version".into(), JsonValue::String(version.to_string()));
    if let Some(packages) = obj.get_mut("packages").and_then(JsonValue::as_object_mut) {
        if let Some(root) = packages.get_mut("").and_then(JsonValue::as_object_mut) {
            root.insert("version".into(), JsonValue::String(version.to_string()));
        }
    }
    let contents = format!("{}\n", serde_json::to_string_pretty(&json)?);
    Ok(Some(LockUpdate { path, contents }))
}

fn check_npm_files(expected: &Version, repo_root: Option<&Path>) -> Result<()> {
    let root = repo_root.map_or_else(|| Path::new(".").to_path_buf(), ToOwned::to_owned);
    check_json_version(root.join("package.json"), expected)?;
    check_package_lock(root.join("package-lock.json"), expected)?;
    Ok(())
}

fn check_json_version(path: PathBuf, expected: &Version) -> Result<()> {
    let expected_s = expected.to_string();
    if !path.exists() {
        return Ok(());
    }
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let json: JsonValue = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    let Some(actual) = json.get("version").and_then(JsonValue::as_str) else {
        bail!("{} missing version field", path.display());
    };
    ensure!(
        actual == expected_s,
        "{} version {} does not match {}",
        path.display(),
        actual,
        expected_s
    );
    Ok(())
}

fn check_package_lock(path: PathBuf, expected: &Version) -> Result<()> {
    let expected_s = expected.to_string();
    if !path.exists() {
        return Ok(());
    }
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let json: JsonValue = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    let version = json
        .get("version")
        .and_then(JsonValue::as_str)
        .context("package-lock.json missing version field")?;
    ensure!(
        version == expected_s,
        "package-lock.json version {} does not match {}",
        version,
        expected_s
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
            root_version == expected_s,
            "Root entry in package-lock.json is {}, expected {}",
            root_version,
            expected_s
        );
    }
    Ok(())
}

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
    type TestResult<T> = anyhow::Result<T>;

    #[test]
    fn next_patch_clears_prerelease() -> TestResult<()> {
        let args = NextArgs {
            mode: Mode::Patch,
            current: Version::parse("1.2.3-beta.1")?,
            value: None,
            meta: None,
            write_meta: false,
        };
        let next = compute_next_version(args)?;
        assert_eq!(next, Version::parse("1.2.4")?);
        Ok(())
    }

    #[test]
    fn next_canary_increments_meta() -> TestResult<()> {
        let temp = tempfile::tempdir()?;
        let meta_path = temp.path().join("meta.json");
        let args = NextArgs {
            mode: Mode::Canary,
            current: Version::parse("0.3.1")?,
            value: None,
            meta: Some(meta_path.clone()),
            write_meta: true,
        };
        let v1 = compute_next_version(args)?;
        assert_eq!(v1.to_string(), "0.3.1-canary.1");

        let args = NextArgs {
            mode: Mode::Canary,
            current: Version::parse("0.3.1")?,
            value: None,
            meta: Some(meta_path.clone()),
            write_meta: true,
        };
        let v2 = compute_next_version(args)?;
        assert_eq!(v2.to_string(), "0.3.1-canary.2");
        Ok(())
    }

    #[test]
    fn update_lock_updates_packages() -> TestResult<()> {
        let temp = tempfile::tempdir()?;
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
        )?;

        update_cargo_lock(
            &Version::parse("0.3.2")?,
            &lock_path,
            &["blz-cli".into(), "blz-core".into()],
        )?;
        let updated = fs::read_to_string(lock_path)?;
        assert!(updated.contains("name = \"blz-cli\""));
        assert!(updated.contains("name = \"blz-core\""));
        assert!(updated.contains("version = \"0.3.2\""));
        Ok(())
    }

    #[test]
    fn update_lock_missing_package_errors() -> TestResult<()> {
        let temp = tempfile::tempdir()?;
        let lock_path = temp.path().join("Cargo.lock");
        fs::write(
            &lock_path,
            r#"version = 3

[[package]]
name = "other"
version = "0.3.1"
"#,
        )?;

        let result = update_cargo_lock(&Version::parse("0.3.2")?, &lock_path, &["blz-cli".into()]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("blz-cli"));
        }
        Ok(())
    }
}
