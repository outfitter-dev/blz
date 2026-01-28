//! Safe numeric conversion helpers with documented precision trade-offs.
//!
//! This module provides explicit, documented conversions for cases where
//! clippy would otherwise require `#[allow]` annotations. Each function
//! clearly documents its precision/truncation behavior.

use std::time::Duration;

/// Convert a [`Duration`] to microseconds as `u64`, saturating at `u64::MAX`.
///
/// # Precision
///
/// For durations longer than ~584,942 years, this saturates at `u64::MAX`.
/// This is acceptable for performance metrics where such durations are impossible.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use blz_core::numeric::duration_to_micros_saturating;
///
/// let d = Duration::from_millis(100);
/// assert_eq!(duration_to_micros_saturating(d), 100_000);
/// ```
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)] // Documented: saturates via min()
pub fn duration_to_micros_saturating(d: Duration) -> u64 {
    // as_micros returns u128, saturate to u64::MAX for extremely long durations
    d.as_micros().min(u128::from(u64::MAX)) as u64
}

/// Convert a [`Duration`] to milliseconds as `f64` with potential precision loss.
///
/// # Precision
///
/// `f64` has 53 bits of mantissa, so durations beyond ~285 years may lose
/// sub-millisecond precision. This is acceptable for display purposes.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use blz_core::numeric::duration_to_millis_lossy;
///
/// let d = Duration::from_micros(1500);
/// assert!((duration_to_millis_lossy(d) - 1.5).abs() < f64::EPSILON);
/// ```
#[inline]
#[must_use]
pub fn duration_to_millis_lossy(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Convert a `usize` to `u64`, saturating on 32-bit platforms (no-op on 64-bit).
///
/// # Platform Behavior
///
/// - On 64-bit platforms: lossless conversion
/// - On 32-bit platforms: lossless (usize fits in u64)
///
/// This is always safe since `usize` <= `u64` on all supported platforms.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::usize_to_u64;
///
/// assert_eq!(usize_to_u64(42), 42u64);
/// assert_eq!(usize_to_u64(usize::MAX), usize::MAX as u64);
/// ```
#[inline]
#[must_use]
pub const fn usize_to_u64(n: usize) -> u64 {
    n as u64
}

/// Convert a `u64` to `f64` with potential precision loss for large values.
///
/// # Precision
///
/// `f64` has 53 bits of mantissa. Values above 2^53 (~9 quadrillion) may
/// lose precision. This is acceptable for display metrics and averages.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::u64_to_f64_lossy;
///
/// assert_eq!(u64_to_f64_lossy(1000), 1000.0);
/// // Large values lose precision but remain usable for display
/// let large = 1u64 << 54;
/// assert!(u64_to_f64_lossy(large) > 0.0);
/// ```
#[inline]
#[must_use]
#[allow(clippy::cast_precision_loss)] // Documented: acceptable for display metrics
pub const fn u64_to_f64_lossy(n: u64) -> f64 {
    n as f64
}

/// Convert a `usize` to `f64` with potential precision loss for large values.
///
/// # Precision
///
/// `f64` has 53 bits of mantissa. On 64-bit platforms, values above 2^53
/// (~9 quadrillion) may lose precision. This is acceptable for display
/// metrics, percentages, and averages.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::usize_to_f64_lossy;
///
/// assert_eq!(usize_to_f64_lossy(1000), 1000.0);
/// // Large values lose precision but remain usable for display
/// let large = 1usize << 54;
/// assert!(usize_to_f64_lossy(large) > 0.0);
/// ```
#[inline]
#[must_use]
#[allow(clippy::cast_precision_loss)] // Documented: acceptable for display metrics
pub const fn usize_to_f64_lossy(n: usize) -> f64 {
    n as f64
}

/// Compute an average from total and count as `f64`, returning 0.0 if count is zero.
///
/// # Precision
///
/// Both `total` and `count` are converted to `f64` before division,
/// which may lose precision for very large values. This is acceptable
/// for performance metric averages.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::safe_average;
///
/// assert_eq!(safe_average(100, 4), 25.0);
/// assert_eq!(safe_average(0, 0), 0.0);
/// assert_eq!(safe_average(100, 0), 0.0);
/// ```
#[inline]
#[must_use]
pub fn safe_average(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        u64_to_f64_lossy(total) / u64_to_f64_lossy(count)
    }
}

/// Compute a percentage from part and total as `f64`, returning 0.0 if total is zero.
///
/// # Precision
///
/// Both `part` and `total` are converted to `f64` before division,
/// which may lose precision for very large values. This is acceptable
/// for percentage calculations in display contexts.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::safe_percentage;
///
/// assert_eq!(safe_percentage(50, 100), 50.0);
/// assert_eq!(safe_percentage(0, 100), 0.0);
/// assert_eq!(safe_percentage(100, 0), 0.0);
/// assert_eq!(safe_percentage(1, 4), 25.0);
/// ```
#[inline]
#[must_use]
pub fn safe_percentage(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (usize_to_f64_lossy(part) / usize_to_f64_lossy(total)) * 100.0
    }
}

/// Convert a floating-point percentage (0.0-100.0) to u8, clamping to valid range.
///
/// # Precision
///
/// The value is rounded and clamped to 0-100, then truncated to u8.
/// This is acceptable for display percentages where sub-percent precision is not needed.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::percent_to_u8;
///
/// assert_eq!(percent_to_u8(50.0), 50);
/// assert_eq!(percent_to_u8(99.9), 100);  // Rounded
/// assert_eq!(percent_to_u8(-5.0), 0);    // Clamped
/// assert_eq!(percent_to_u8(150.0), 100); // Clamped
/// ```
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Documented: clamped to 0-100
pub fn percent_to_u8(percent: f64) -> u8 {
    percent.round().clamp(0.0, 100.0) as u8
}

/// Calculate how many items to take from a collection for a given percentile.
///
/// Returns the count of items needed to include the top `percentile` percent
/// of the collection, with a minimum of 1 (unless len is 0).
///
/// # Precision
///
/// Uses `f64` for the calculation, which may lose precision for very large
/// collections. This is acceptable for typical use cases like result filtering.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::percentile_count;
///
/// assert_eq!(percentile_count(100, 10), 10);  // Top 10% of 100 = 10 items
/// assert_eq!(percentile_count(100, 50), 50);  // Top 50% of 100 = 50 items
/// assert_eq!(percentile_count(10, 25), 3);    // Top 25% of 10 = 3 items (rounded up)
/// assert_eq!(percentile_count(5, 10), 1);     // Top 10% of 5 = 1 item (minimum)
/// assert_eq!(percentile_count(0, 50), 0);     // Empty collection returns 0
/// ```
#[inline]
#[must_use]
pub fn percentile_count(len: usize, percentile: u8) -> usize {
    if len == 0 {
        return 0;
    }
    let percentile_f = f64::from(percentile) / 100.0;
    let len_f = usize_to_f64_lossy(len);
    let count = (len_f * percentile_f).ceil().min(len_f);
    // count is guaranteed to be non-negative and <= len_f, so conversion is safe
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (count as usize).max(1)
}

/// Convert bytes to a human-readable size string with appropriate units.
///
/// # Precision
///
/// Uses `f64` for division, which may lose precision for sizes above
/// ~9 PB. This is acceptable for display purposes.
///
/// # Examples
///
/// ```
/// use blz_core::numeric::format_bytes;
///
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1023), "1023 B");
/// assert_eq!(format_bytes(1024), "1.0 KB");
/// assert_eq!(format_bytes(1536), "1.5 KB");
/// assert_eq!(format_bytes(1_048_576), "1.0 MB");
/// ```
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let bytes_f = u64_to_f64_lossy(bytes);

    if bytes_f >= TB {
        format!("{:.1} TB", bytes_f / TB)
    } else if bytes_f >= GB {
        format!("{:.1} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_to_micros_saturating() {
        assert_eq!(duration_to_micros_saturating(Duration::ZERO), 0);
        assert_eq!(
            duration_to_micros_saturating(Duration::from_secs(1)),
            1_000_000
        );
        assert_eq!(
            duration_to_micros_saturating(Duration::from_millis(500)),
            500_000
        );
    }

    #[test]
    fn test_duration_to_millis_lossy() {
        let d = Duration::from_micros(1500);
        assert!((duration_to_millis_lossy(d) - 1.5).abs() < f64::EPSILON);

        let d = Duration::from_secs(2);
        assert!((duration_to_millis_lossy(d) - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_usize_to_u64() {
        assert_eq!(usize_to_u64(0), 0u64);
        assert_eq!(usize_to_u64(42), 42u64);
        assert_eq!(usize_to_u64(usize::MAX), usize::MAX as u64);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact comparison acceptable for these small values
    fn test_u64_to_f64_lossy() {
        assert_eq!(u64_to_f64_lossy(0), 0.0);
        assert_eq!(u64_to_f64_lossy(1000), 1000.0);
        // Large values should convert without panic
        let _ = u64_to_f64_lossy(u64::MAX);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact comparison acceptable for these small values
    fn test_safe_average() {
        assert_eq!(safe_average(100, 4), 25.0);
        assert_eq!(safe_average(0, 0), 0.0);
        assert_eq!(safe_average(100, 0), 0.0);
        assert_eq!(safe_average(7, 2), 3.5);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
        assert_eq!(format_bytes(1_099_511_627_776), "1.0 TB");
    }
}
