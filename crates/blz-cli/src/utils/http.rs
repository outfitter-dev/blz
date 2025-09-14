use anyhow::Result;
use blz_core::{Fetcher, fetcher::HeadInfo};
use tokio::time::{Duration, sleep};

/// Perform HEAD preflight with limited retries on transient failures.
///
/// Retries on:
/// - Network errors from reqwest
/// - HTTP 429 or 5xx statuses
///   Does not retry on 4xx (except 429).
pub async fn head_with_retries(
    fetcher: &Fetcher,
    url: &str,
    attempts: u32,
    base_delay_ms: u64,
) -> Result<HeadInfo> {
    let mut last_err: Option<anyhow::Error> = None;
    for i in 0..attempts {
        match fetcher.head_metadata(url).await {
            Ok(info) => {
                let status = info.status;
                if status == 429 || (500..=599).contains(&status) {
                    // transient HTTP error, retry
                } else {
                    return Ok(info);
                }
            },
            Err(e) => {
                last_err = Some(e.into());
            },
        }

        // Backoff before next attempt (except after the last one)
        if i + 1 < attempts {
            let delay = base_delay_ms.saturating_mul(1u64 << i);
            sleep(Duration::from_millis(delay.min(2_000))).await;
        }
    }

    last_err.map_or_else(
        || {
            // If we ended up here due to repeated transient HTTP statuses, surface a clear error
            Err(anyhow::anyhow!(
                "Preflight failed for {url} after {attempts} attempts (transient HTTP status)"
            ))
        },
        Err,
    )
}
