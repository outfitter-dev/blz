use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{IS_SUPPORTED_SYSTEM, Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::{info, warn};

const DEFAULT_POLL_INTERVAL_MS: u64 = 500;
const MIN_POLL_INTERVAL_MS: u64 = 100;
const MAX_POLL_INTERVAL_MS: u64 = 10_000;

/// Spawn a background watchdog that terminates the CLI when its parent process
/// goes away. This prevents orphaned `blz` processes when test harnesses or
/// wrapper scripts die unexpectedly (e.g., Ctrl+C during `cargo test`).
pub fn spawn_parent_exit_guard() {
    // No parent concept on WASM targets.
    #[cfg(target_family = "wasm")]
    {
        let _ = POLL_INTERVAL;
        return;
    }

    #[cfg(not(target_family = "wasm"))]
    {
        if !IS_SUPPORTED_SYSTEM {
            info!("parent-exit-guard not supported on this platform; skipping");
            return;
        }

        if std::env::var_os("BLZ_DISABLE_GUARD").is_some() {
            return;
        }

        let current_pid = std::process::id();

        let poll_interval_ms = std::env::var("BLZ_PARENT_GUARD_INTERVAL_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .filter(|value| (MIN_POLL_INTERVAL_MS..=MAX_POLL_INTERVAL_MS).contains(value))
            .unwrap_or(DEFAULT_POLL_INTERVAL_MS);

        let timeout_ms = std::env::var("BLZ_PARENT_GUARD_TIMEOUT_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .filter(|&ms| ms > 0)
            .map(Duration::from_millis);

        let timeout = timeout_ms.or_else(|| {
            std::env::var("BLZ_PARENT_GUARD_TIMEOUT_SECS")
                .ok()
                .and_then(|raw| raw.parse::<u64>().ok())
                .filter(|&secs| secs > 0)
                .map(Duration::from_secs)
        });

        if thread::Builder::new()
            .name("blz-parent-guard".into())
            .spawn(move || monitor_parent(current_pid, poll_interval_ms, timeout))
            .is_err()
        {
            warn!("failed to spawn parent exit guard; continuing without orphan protection");
        }
    }
}

fn monitor_parent(current_pid_raw: u32, poll_interval_ms: u64, guard_timeout: Option<Duration>) {
    const EXIT_PARENT_LOST: i32 = 129;
    const EXIT_GUARD_TIMEOUT: i32 = 124;

    let mut system = System::new();
    let refresh_kind = ProcessRefreshKind::new();
    let current_pid = Pid::from(current_pid_raw as usize);
    let current_update = [current_pid];

    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&current_update),
        false,
        refresh_kind,
    );

    let Some(parent_pid) = system
        .process(current_pid)
        .and_then(sysinfo::Process::parent)
    else {
        // No parent detected (already orphaned or running under init). Nothing to monitor.
        return;
    };

    // Optional timeout, primarily used by test harnesses when BLZ_PARENT_GUARD_TIMEOUT_{MS,SECS} is set
    let guard_deadline = guard_timeout.map(|timeout| Instant::now() + timeout);

    loop {
        let parent_update = [parent_pid];
        let updated = system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&parent_update),
            false,
            refresh_kind,
        );

        if updated == 0 && system.process(parent_pid).is_none() {
            // If sysinfo failed to refresh processes on this platform, wait and retry instead of
            // eagerly exiting. This still detects real exits once the process truly disappears.
            tracing::debug!(parent = %parent_pid, "unable to refresh parent process state; retrying");
            thread::sleep(Duration::from_millis(poll_interval_ms));
            continue;
        }

        if system.process(parent_pid).is_none() {
            warn!(parent = %parent_pid, "parent process exited; terminating orphaned blz process");
            std::process::exit(EXIT_PARENT_LOST);
        }

        if let Some(deadline) = guard_deadline {
            if Instant::now() >= deadline {
                warn!("parent guard timeout reached; terminating");
                std::process::exit(EXIT_GUARD_TIMEOUT);
            }
        }

        thread::sleep(Duration::from_millis(poll_interval_ms));
    }
}
