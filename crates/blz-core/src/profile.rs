use std::ffi::OsStr;
use std::sync::OnceLock;

/// Execution profile that influences default storage locations and other behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppProfile {
    /// Standard release profile matching user-facing installations.
    Default,
    /// Developer profile with isolated storage intended for test builds.
    Dev,
}

static PROFILE: OnceLock<AppProfile> = OnceLock::new();

/// Override the inferred application profile (primarily used by alternative binaries).
///
/// Subsequent calls after the profile has been set are ignored to preserve initialization order.
pub fn set(profile: AppProfile) {
    let _ = PROFILE.set(profile);
}

/// Determine the active profile, defaulting to production unless overridden.
#[must_use]
pub fn current() -> AppProfile {
    *PROFILE.get_or_init(detect_profile)
}

/// Returns the directory slug used for platform-specific config/data paths.
#[must_use]
pub fn app_dir_slug() -> &'static str {
    match current() {
        AppProfile::Default => "blz",
        AppProfile::Dev => "blz-dev",
    }
}

/// Returns the dot-directory slug used for non-XDG fallbacks.
#[must_use]
pub fn dot_dir_slug() -> &'static str {
    match current() {
        AppProfile::Default => ".blz",
        AppProfile::Dev => ".blz-dev",
    }
}

fn detect_profile() -> AppProfile {
    if let Ok(value) = std::env::var("BLZ_PROFILE") {
        if value.eq_ignore_ascii_case("dev") {
            return AppProfile::Dev;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(name) = exe.file_name().and_then(OsStr::to_str) {
            if name.contains("blz-dev") {
                return AppProfile::Dev;
            }
        }
    }

    AppProfile::Default
}
