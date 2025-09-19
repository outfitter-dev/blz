use std::thread;
use std::time::Duration;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::warn;

const POLL_INTERVAL: Duration = Duration::from_millis(750);

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
        let current_pid = std::process::id();
        if thread::Builder::new()
            .name("blz-parent-guard".into())
            .spawn(move || monitor_parent(current_pid))
            .is_err()
        {
            warn!("failed to spawn parent exit guard; continuing without orphan protection");
        }
    }
}

fn monitor_parent(current_pid_raw: u32) {
    let mut system = System::new();
    let refresh_kind = ProcessRefreshKind::new();
    let current_pid = Pid::from(current_pid_raw as usize);
    let current_update = [current_pid];

    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&current_update),
        false,
        refresh_kind,
    );

    let Some(parent_pid) = system.process(current_pid).and_then(|proc| proc.parent()) else {
        // No parent detected (already orphaned or running under init). Nothing to monitor.
        return;
    };

    loop {
        let parent_update = [parent_pid];
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&parent_update),
            true,
            refresh_kind,
        );

        if system.process(parent_pid).is_none() {
            warn!(parent = %parent_pid, "parent process exited; terminating orphaned blz process");
            std::process::exit(0);
        }

        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&current_update),
            false,
            refresh_kind,
        );

        match system.process(current_pid).and_then(|proc| proc.parent()) {
            Some(pid) if pid == parent_pid => {
                // Parent unchanged; continue monitoring.
            },
            Some(pid) => {
                warn!(new_parent = %pid, "parent PID changed; terminating orphaned blz process");
                std::process::exit(0);
            },
            None => {
                // Our process no longer has a parent reference; exit to avoid misbehaving.
                std::process::exit(0);
            },
        }

        thread::sleep(POLL_INTERVAL);
    }
}
