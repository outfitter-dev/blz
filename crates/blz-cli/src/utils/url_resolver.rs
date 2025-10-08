//! URL resolution utilities for smart llms.txt variant detection
//!
//! Automatically prefers llms-full.txt over llms.txt when available, with
//! fallback to exact URL if neither variant exists. Uses HEAD requests to
//! check availability before fetching content.

use anyhow::Result;
use blz_core::{ContentType, Fetcher, SourceVariant};
use tracing::{debug, warn};

/// Result of URL resolution with variant and content info
#[derive(Debug, Clone)]
pub struct ResolvedUrl {
    /// The final URL that succeeded
    pub final_url: String,
    /// Which variant was resolved
    pub variant: SourceVariant,
    /// Content type based on line count
    pub content_type: ContentType,
    /// Number of lines in the content
    pub line_count: usize,
    /// Whether to warn the user about this source
    pub should_warn: bool,
}

/// Resolve the best URL for llms.txt documentation
///
/// Tries URLs in order:
/// 1. llms-full.txt variant (if base URL ends in .txt)
/// 2. llms.txt variant (if base URL ends in -full.txt)
/// 3. Exact URL as provided
///
/// Uses HEAD requests to check availability before fetching content.
///
/// # Examples
///
/// ```no_run
/// use blz_cli::utils::url_resolver::resolve_best_url;
/// use blz_core::Fetcher;
///
/// # async fn example() -> anyhow::Result<()> {
/// let fetcher = Fetcher::new()?;
/// let resolved = resolve_best_url(&fetcher, "https://react.dev/llms.txt").await?;
///
/// // Will try llms-full.txt first, fallback to llms.txt
/// assert_eq!(resolved.variant, blz_core::SourceVariant::LlmsFull);
/// # Ok(())
/// # }
/// ```
pub async fn resolve_best_url(fetcher: &Fetcher, base_url: &str) -> Result<ResolvedUrl> {
    // Try variants in order of preference
    let variants = [
        try_full_variant(base_url),
        Some(base_url.to_string()),
        try_base_variant(base_url),
    ];

    for (idx, maybe_url) in variants.iter().enumerate() {
        if let Some(url) = maybe_url {
            // Check if this URL is available
            let should_fetch = match fetcher.head_metadata(url).await {
                Ok(head_info) => {
                    let status = head_info.status;
                    if (200..=399).contains(&status) {
                        true
                    } else if status == 405 || status == 501 {
                        warn!(
                            %status,
                            %url,
                            "HEAD not supported; falling back to GET for candidate URL"
                        );
                        true
                    } else {
                        debug!(
                            %status,
                            %url,
                            "HEAD preflight rejected candidate URL"
                        );
                        false
                    }
                },
                Err(err) => {
                    debug!(error = %err, %url, "HEAD preflight failed for candidate URL");
                    false
                },
            };

            if !should_fetch {
                continue;
            }

            // URL exists or HEAD is unsupported; fetch the content to analyze it
            let (content, _sha256) = match fetcher.fetch(url).await {
                Ok(result) => result,
                Err(err) => {
                    debug!(error = %err, %url, "GET fallback failed for candidate URL");
                    continue;
                },
            };

            let line_count = content.lines().count();
            let (content_type, should_warn) = classify_content(line_count);

            // Determine variant based on which URL succeeded
            let variant = match idx {
                0 => SourceVariant::LlmsFull, // llms-full.txt variant
                1 => {
                    // Exact URL - determine if it's a known variant
                    if base_url.ends_with("llms-full.txt") {
                        SourceVariant::LlmsFull
                    } else if base_url.ends_with("llms.txt") {
                        SourceVariant::Llms
                    } else {
                        SourceVariant::Custom
                    }
                },
                2 => SourceVariant::Llms, // llms.txt fallback
                _ => SourceVariant::Custom,
            };

            return Ok(ResolvedUrl {
                final_url: url.clone(),
                variant,
                content_type,
                line_count,
                should_warn,
            });
        }
    }

    // None of the variants worked
    anyhow::bail!(
        "Failed to resolve any llms.txt variant for '{}'. \
         Tried: llms-full.txt, exact URL, and llms.txt fallback.",
        base_url
    )
}

/// Generate llms-full.txt variant URL
fn try_full_variant(url: &str) -> Option<String> {
    // If URL ends with "llms.txt" (but not "llms-full.txt"), try "llms-full.txt"
    if url.ends_with("llms.txt") && !url.ends_with("llms-full.txt") {
        Some(url.replace("llms.txt", "llms-full.txt"))
    } else {
        None
    }
}

/// Generate llms.txt variant URL
fn try_base_variant(url: &str) -> Option<String> {
    // If URL ends with "llms-full.txt", try "llms.txt"
    if url.ends_with("llms-full.txt") {
        Some(url.replace("llms-full.txt", "llms.txt"))
    } else {
        None
    }
}

/// Determine content type from line count
const fn classify_content(line_count: usize) -> (ContentType, bool) {
    if line_count > 1000 {
        (ContentType::Full, false)
    } else if line_count < 100 {
        (ContentType::Index, true) // Warn user
    } else {
        (ContentType::Mixed, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_full_variant() {
        // Should convert llms.txt to llms-full.txt
        assert_eq!(
            try_full_variant("https://example.com/llms.txt"),
            Some("https://example.com/llms-full.txt".to_string())
        );

        // Should not modify llms-full.txt
        assert_eq!(try_full_variant("https://example.com/llms-full.txt"), None);

        // Should not modify custom URLs
        assert_eq!(try_full_variant("https://example.com/docs.txt"), None);
    }

    #[test]
    fn test_try_base_variant() {
        // Should convert llms-full.txt to llms.txt
        assert_eq!(
            try_base_variant("https://example.com/llms-full.txt"),
            Some("https://example.com/llms.txt".to_string())
        );

        // Should not modify llms.txt
        assert_eq!(try_base_variant("https://example.com/llms.txt"), None);

        // Should not modify custom URLs
        assert_eq!(try_base_variant("https://example.com/docs.txt"), None);
    }

    #[test]
    fn test_classify_content() {
        // Full documentation
        let (content_type, should_warn) = classify_content(1500);
        assert_eq!(content_type, ContentType::Full);
        assert!(!should_warn);

        // Index file (should warn)
        let (content_type, should_warn) = classify_content(50);
        assert_eq!(content_type, ContentType::Index);
        assert!(should_warn);

        // Mixed content
        let (content_type, should_warn) = classify_content(500);
        assert_eq!(content_type, ContentType::Mixed);
        assert!(!should_warn);

        // Edge cases
        let (content_type, _) = classify_content(100);
        assert_eq!(content_type, ContentType::Mixed);

        let (content_type, _) = classify_content(1000);
        assert_eq!(content_type, ContentType::Mixed);

        let (content_type, _) = classify_content(1001);
        assert_eq!(content_type, ContentType::Full);

        let (content_type, should_warn) = classify_content(99);
        assert_eq!(content_type, ContentType::Index);
        assert!(should_warn);
    }

    // Integration tests with mock server would go here
    // Requires tokio and wiremock, similar to fetcher tests
}
