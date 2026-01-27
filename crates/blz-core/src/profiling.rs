#![allow(clippy::cast_possible_wrap)] // Wrapping is acceptable for memory delta calculations

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::{Level, debug, info, span};

use crate::numeric::{duration_to_micros_saturating, safe_average, u64_to_f64_lossy, usize_to_u64};

/// Global performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Number of search operations performed
    pub search_count: Arc<AtomicU64>,
    /// Total time spent in search operations (in microseconds)
    pub total_search_time: Arc<AtomicU64>,
    /// Number of index build operations performed
    pub index_build_count: Arc<AtomicU64>,
    /// Total time spent in index build operations (in microseconds)
    pub total_index_time: Arc<AtomicU64>,
    /// Total bytes processed during indexing
    pub bytes_processed: Arc<AtomicU64>,
    /// Total lines searched across all operations
    pub lines_searched: Arc<AtomicU64>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            search_count: Arc::new(AtomicU64::new(0)),
            total_search_time: Arc::new(AtomicU64::new(0)),
            index_build_count: Arc::new(AtomicU64::new(0)),
            total_index_time: Arc::new(AtomicU64::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            lines_searched: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl PerformanceMetrics {
    /// Record a search operation
    pub fn record_search(&self, duration: Duration, lines_count: usize) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        let inc = duration_to_micros_saturating(duration);
        let _ = self
            .total_search_time
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_add(inc))
            });
        self.lines_searched
            .fetch_add(usize_to_u64(lines_count), Ordering::Relaxed);
    }

    /// Record an index build operation
    pub fn record_index_build(&self, duration: Duration, bytes_count: usize) {
        self.index_build_count.fetch_add(1, Ordering::Relaxed);
        let inc = duration_to_micros_saturating(duration);
        let _ = self
            .total_index_time
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_add(inc))
            });
        self.bytes_processed
            .fetch_add(usize_to_u64(bytes_count), Ordering::Relaxed);
    }

    /// Get average search time in microseconds
    #[must_use]
    pub fn avg_search_time_micros(&self) -> f64 {
        let count = self.search_count.load(Ordering::Relaxed);
        let total = self.total_search_time.load(Ordering::Relaxed);
        safe_average(total, count)
    }

    /// Get average index build time in milliseconds
    #[must_use]
    pub fn avg_index_time_millis(&self) -> f64 {
        let count = self.index_build_count.load(Ordering::Relaxed);
        let total = self.total_index_time.load(Ordering::Relaxed);
        safe_average(total, count) / 1000.0
    }

    /// Get throughput in lines per second for search operations
    #[must_use]
    pub fn search_throughput_lines_per_sec(&self) -> f64 {
        let lines = self.lines_searched.load(Ordering::Relaxed);
        let time_seconds =
            u64_to_f64_lossy(self.total_search_time.load(Ordering::Relaxed)) / 1_000_000.0;
        if time_seconds == 0.0 {
            0.0
        } else {
            u64_to_f64_lossy(lines) / time_seconds
        }
    }

    /// Get processing throughput in MB/s for indexing operations
    #[must_use]
    pub fn index_throughput_mbps(&self) -> f64 {
        let bytes = self.bytes_processed.load(Ordering::Relaxed);
        let time_seconds =
            u64_to_f64_lossy(self.total_index_time.load(Ordering::Relaxed)) / 1_000_000.0;
        if time_seconds == 0.0 {
            0.0
        } else {
            (u64_to_f64_lossy(bytes) / (1024.0 * 1024.0)) / time_seconds
        }
    }

    /// Print performance summary
    pub fn print_summary(&self) {
        let searches = self.search_count.load(Ordering::Relaxed);
        let indexes = self.index_build_count.load(Ordering::Relaxed);

        println!("\n{}", "Performance Summary".bold());
        println!("{}", "===================".bold());

        if searches > 0 {
            println!("Search Operations:");
            println!("  Total searches: {searches}");
            println!(
                "  Average time: {:.2}ms",
                self.avg_search_time_micros() / 1000.0
            );
            println!(
                "  Total lines searched: {}",
                self.lines_searched.load(Ordering::Relaxed)
            );
            println!(
                "  Throughput: {:.0} lines/sec",
                self.search_throughput_lines_per_sec()
            );
        }

        if indexes > 0 {
            println!("Index Operations:");
            println!("  Total builds: {indexes}");
            println!("  Average time: {:.2}ms", self.avg_index_time_millis());
            println!(
                "  Total bytes processed: {}",
                format_bytes(self.bytes_processed.load(Ordering::Relaxed))
            );
            println!("  Throughput: {:.2} MB/s", self.index_throughput_mbps());
        }
    }
}

/// Timer for measuring operation duration with automatic metrics recording
pub struct OperationTimer {
    /// Start time of the operation
    start: Instant,
    /// Name or description of the operation being timed
    operation: String,
    /// Optional metrics collector for recording results
    metrics: Option<PerformanceMetrics>,
}

impl OperationTimer {
    /// Creates a new operation timer with basic logging
    pub fn new(operation: &str) -> Self {
        info!("Starting operation: {}", operation);
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
            metrics: None,
        }
    }

    /// Creates a new operation timer with metrics collection
    pub fn with_metrics(operation: &str, metrics: PerformanceMetrics) -> Self {
        info!("Starting operation with metrics: {}", operation);
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
            metrics: Some(metrics),
        }
    }

    /// Finish timing and optionally record metrics
    pub fn finish(self) -> Duration {
        let duration = self.start.elapsed();
        info!(
            "Completed {}: {:.2}ms",
            self.operation,
            duration.as_millis()
        );
        duration
    }

    /// Finish timing a search operation with line count
    pub fn finish_search(self, lines_count: usize) -> Duration {
        let duration = self.start.elapsed();
        info!(
            "Completed {} search: {:.2}ms ({} lines)",
            self.operation,
            duration.as_millis(),
            lines_count
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_search(duration, lines_count);
        }
        duration
    }

    /// Finish timing an index operation with byte count
    pub fn finish_index(self, bytes_count: usize) -> Duration {
        let duration = self.start.elapsed();
        info!(
            "Completed {} indexing: {:.2}ms ({} bytes)",
            self.operation,
            duration.as_millis(),
            bytes_count
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_index_build(duration, bytes_count);
        }
        duration
    }
}

/// Component-level timing breakdown for detailed analysis
#[derive(Debug, Default)]
pub struct ComponentTimings {
    /// Map of component names to their cumulative durations
    timings: HashMap<String, Duration>,
}

impl ComponentTimings {
    /// Creates a new component timings tracker
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Times an operation and records it under the given component name
    pub fn time<T, F>(&mut self, component: &str, operation: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _span = span!(Level::DEBUG, "component_timing", component = component);
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();

        self.timings.insert(
            component.to_string(),
            self.timings.get(component).copied().unwrap_or_default() + duration,
        );

        debug!("Component {}: {:.2}ms", component, duration.as_millis());
        result
    }

    /// Gets the cumulative timing for a specific component
    #[must_use]
    pub fn get_timing(&self, component: &str) -> Option<Duration> {
        self.timings.get(component).copied()
    }

    /// Calculates the total time across all components
    #[must_use]
    pub fn total_time(&self) -> Duration {
        self.timings.values().sum()
    }

    /// Prints a formatted breakdown of component timings
    pub fn print_breakdown(&self) {
        if self.timings.is_empty() {
            return;
        }

        let total = self.total_time();
        println!("\n{}", "Component Breakdown".bold());
        println!("{}", "==================".bold());

        let mut sorted_timings: Vec<_> = self.timings.iter().collect();
        sorted_timings.sort_by(|a, b| b.1.cmp(a.1));

        for (component, duration) in sorted_timings {
            #[allow(clippy::cast_precision_loss)] // Display-only percentage calculation
            let percentage = if total.as_micros() > 0 {
                (duration.as_micros() as f64 / total.as_micros() as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "  {:<20}: {:>8.2}ms ({:>5.1}%)",
                component,
                duration.as_millis(),
                percentage
            );
        }

        println!("  {:<20}: {:>8.2}ms", "TOTAL", total.as_millis());
    }
}

/// System resource monitor for memory and CPU usage
pub struct ResourceMonitor {
    /// System information collector from sysinfo crate
    system: System,
    /// Process ID of the current process
    pid: u32,
    /// Memory usage at monitor creation time (in bytes)
    initial_memory: u64,
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMonitor {
    /// Creates a new resource monitor and captures initial state
    #[must_use]
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = std::process::id();

        let initial_memory = system
            .process(sysinfo::Pid::from(pid as usize))
            .map_or(0, sysinfo::Process::memory);

        Self {
            system,
            pid,
            initial_memory,
        }
    }

    /// Refreshes system information
    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    /// Gets the current memory usage in megabytes
    #[allow(clippy::cast_precision_loss)] // Memory values far below f64 precision limit
    pub fn current_memory_mb(&mut self) -> f64 {
        self.refresh();
        self.system
            .process(sysinfo::Pid::from(self.pid as usize))
            .map_or(0.0, |process| process.memory() as f64 / (1024.0 * 1024.0))
    }

    /// Gets the memory usage change since initialization in megabytes
    #[allow(clippy::cast_precision_loss)] // Memory values far below f64 precision limit
    pub fn memory_delta_mb(&mut self) -> f64 {
        self.refresh();
        if let Some(process) = self.system.process(sysinfo::Pid::from(self.pid as usize)) {
            let current = process.memory();
            (current as i64 - self.initial_memory as i64) as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }

    /// Gets the current CPU usage percentage for this process
    pub fn cpu_usage(&mut self) -> f32 {
        self.refresh();
        self.system
            .process(sysinfo::Pid::from(self.pid as usize))
            .map_or(0.0, sysinfo::Process::cpu_usage)
    }

    /// Prints formatted resource usage information
    pub fn print_resource_usage(&mut self) {
        println!("\n{}", "Resource Usage".bold());
        println!("{}", "==============".bold());
        println!(
            "Memory: {:.1} MB (Δ{:+.1} MB)",
            self.current_memory_mb(),
            self.memory_delta_mb()
        );
        println!("CPU: {:.1}%", self.cpu_usage());
    }
}

/// Start CPU profiling (requires --features=flamegraph).
///
/// # Errors
///
/// Returns an error if the profiler cannot be initialized.
#[cfg(feature = "flamegraph")]
pub fn start_profiling() -> Result<pprof::ProfilerGuard<'static>, Box<dyn std::error::Error>> {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000) // 1kHz sampling
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()?;
    Ok(guard)
}

/// Stop profiling and generate flamegraph.
///
/// # Errors
///
/// Returns an error if the report cannot be generated or written.
#[cfg(feature = "flamegraph")]
pub fn stop_profiling_and_report(
    guard: &pprof::ProfilerGuard,
) -> Result<(), Box<dyn std::error::Error>> {
    match guard.report().build() {
        Ok(report) => {
            // Note: Protobuf output temporarily disabled due to API changes
            // TODO: Re-enable once pprof protobuf API is clarified

            // Generate flamegraph if possible
            let file = std::fs::File::create("flamegraph.svg")?;
            report.flamegraph(file)?;
            println!("Flamegraph saved to flamegraph.svg");
        },
        Err(e) => {
            eprintln!("Failed to generate profile report: {e}");
        },
    }
    Ok(())
}

/// Fallback profiling stubs when flamegraph feature is disabled.
///
/// # Errors
///
/// Returns `Ok(())` because profiling is disabled.
#[cfg(not(feature = "flamegraph"))]
#[allow(clippy::unnecessary_wraps)] // Need to match the API of the feature-enabled version
pub fn start_profiling() -> Result<(), Box<dyn std::error::Error>> {
    debug!("CPU profiling not available (flamegraph feature not enabled)");
    Ok(())
}

/// Stops CPU profiling and generates a flamegraph report (no-op when flamegraph feature is disabled).
///
/// # Errors
///
/// Returns `Ok(())` because profiling is disabled.
#[cfg(not(feature = "flamegraph"))]
#[allow(clippy::unnecessary_wraps)] // Need to match the API of the feature-enabled version
pub fn stop_profiling_and_report(_guard: ()) -> Result<(), Box<dyn std::error::Error>> {
    debug!("CPU profiling not available (flamegraph feature not enabled)");
    Ok(())
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    use crate::numeric::u64_to_f64_lossy;
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = u64_to_f64_lossy(bytes);
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Extension trait to add bold formatting (simple implementation for this example)
trait BoldFormat {
    fn bold(&self) -> &Self;
}

impl BoldFormat for str {
    fn bold(&self) -> &Self {
        // In a real implementation, you might use colored crate or similar
        // For now, just return the string as-is
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::default();

        // Record some search operations
        metrics.record_search(Duration::from_millis(5), 1000);
        metrics.record_search(Duration::from_millis(7), 1500);

        assert_eq!(metrics.search_count.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.lines_searched.load(Ordering::Relaxed), 2500);

        // Average should be 6ms = 6000 microseconds
        assert!((metrics.avg_search_time_micros() - 6000.0).abs() < 1.0);
    }

    #[test]
    fn test_operation_timer() {
        let timer = OperationTimer::new("test_operation");
        thread::sleep(Duration::from_millis(1));
        let duration = timer.finish();

        assert!(duration >= Duration::from_millis(1));
    }

    #[test]
    fn test_component_timings() {
        let mut timings = ComponentTimings::new();

        timings.time("parsing", || {
            thread::sleep(Duration::from_millis(2));
            "parsed"
        });

        timings.time("indexing", || {
            thread::sleep(Duration::from_millis(3));
            "indexed"
        });

        let parsing_time = timings.get_timing("parsing").unwrap();
        let indexing_time = timings.get_timing("indexing").unwrap();

        assert!(parsing_time >= Duration::from_millis(2));
        assert!(indexing_time >= Duration::from_millis(3));
        assert!(timings.total_time() >= Duration::from_millis(5));
    }

    #[test]
    fn test_resource_monitor() {
        let mut monitor = ResourceMonitor::new();
        let memory = monitor.current_memory_mb();
        let _cpu = monitor.cpu_usage();

        assert!(memory > 0.0, "Should report some memory usage");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(2_097_152), "2.0 MB");
    }
}
