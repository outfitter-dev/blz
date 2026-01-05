//! URL resolution utilities for smart llms.txt variant detection.
//!
//! Automatically prefers llms-full.txt over llms.txt when available, with
//! fallback to exact URL if neither variant exists. Uses HEAD requests to
//! check availability before fetching content.

use tracing::{debug, warn};

use crate::{ContentType, Error, Fetcher, Result, SourceVariant};

/// Result of URL resolution with variant and content info.
#[derive(Debug, Clone)]
pub struct ResolvedUrl {
    /// The final URL that succeeded.
    pub final_url: String,
    /// Which variant was resolved.
    pub variant: SourceVariant,
    /// Content type based on line count.
    pub content_type: ContentType,
    /// Number of lines in the content.
    pub line_count: usize,
    /// Whether to warn the user about this source.
    pub should_warn: bool,
}

/// Resolve the best URL for llms.txt documentation.
///
/// Tries URLs in order:
/// 1. llms-full.txt variant (if base URL ends in .txt)
/// 2. llms.txt variant (if base URL ends in -full.txt)
/// 3. Exact URL as provided
///
/// Uses HEAD requests to check availability before fetching content.
pub async fn resolve_best_url(fetcher: &Fetcher, base_url: &str) -> Result<ResolvedUrl> {
    let variants = [
        try_full_variant(base_url),
        Some(base_url.to_string()),
        try_base_variant(base_url),
    ];

    for (idx, maybe_url) in variants.iter().enumerate() {
        if let Some(url) = maybe_url {
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

            let (content, _sha256) = match fetcher.fetch(url).await {
                Ok(result) => result,
                Err(err) => {
                    debug!(error = %err, %url, "GET fallback failed for candidate URL");
                    continue;
                },
            };

            let line_count = content.lines().count();
            let (content_type, should_warn) = classify_content(line_count);

            let variant = match idx {
                0 => SourceVariant::LlmsFull,
                1 => {
                    if base_url.ends_with("llms-full.txt") {
                        SourceVariant::LlmsFull
                    } else if base_url.ends_with("llms.txt") {
                        SourceVariant::Llms
                    } else {
                        SourceVariant::Custom
                    }
                },
                2 => SourceVariant::Llms,
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

    Err(Error::NotFound(format!(
        "Failed to resolve any llms.txt variant for '{base_url}'. \
         Tried: llms-full.txt, exact URL, and llms.txt fallback."
    )))
}

fn try_full_variant(url: &str) -> Option<String> {
    if url.ends_with("llms.txt") && !url.ends_with("llms-full.txt") {
        Some(url.replace("llms.txt", "llms-full.txt"))
    } else {
        None
    }
}

fn try_base_variant(url: &str) -> Option<String> {
    if url.ends_with("llms-full.txt") {
        Some(url.replace("llms-full.txt", "llms.txt"))
    } else {
        None
    }
}

const fn classify_content(line_count: usize) -> (ContentType, bool) {
    if line_count > 1000 {
        (ContentType::Full, false)
    } else if line_count < 100 {
        (ContentType::Index, true)
    } else {
        (ContentType::Mixed, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_full_variant() {
        assert_eq!(
            try_full_variant("https://example.com/llms.txt"),
            Some("https://example.com/llms-full.txt".to_string())
        );

        assert_eq!(try_full_variant("https://example.com/llms-full.txt"), None);

        assert_eq!(try_full_variant("https://example.com/docs.txt"), None);
    }

    #[test]
    fn test_try_base_variant() {
        assert_eq!(
            try_base_variant("https://example.com/llms-full.txt"),
            Some("https://example.com/llms.txt".to_string())
        );

        assert_eq!(try_base_variant("https://example.com/llms.txt"), None);

        assert_eq!(try_base_variant("https://example.com/docs.txt"), None);
    }
}
