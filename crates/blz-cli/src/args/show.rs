//! Show component argument types for CLI commands.
//!
//! This module provides the `ShowComponent` enum for specifying which
//! additional columns to display in search results.
//!
//! # Design
//!
//! The `ShowComponent` enum maps to the `--show` CLI argument and can be
//! used multiple times or with comma-separated values:
//!
//! ```bash
//! blz query "useEffect" --show rank --show url
//! blz query "useEffect" --show rank,url,lines
//! ```
//!
//! # Available Components
//!
//! - `rank` - Include global rank prefix (1., 2., ...)
//! - `url` - Display source URL header for matched aliases
//! - `lines` - Prefix snippet lines with line numbers
//! - `anchor` - Show hashed section anchor above snippet
//! - `raw-score` - Show raw BM25 scores instead of percentages

use serde::{Deserialize, Serialize};

/// Additional columns that can be displayed in text search results.
///
/// Use with the `--show` flag to customize output:
///
/// ```bash
/// blz query "react hooks" --show rank,url
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShowComponent {
    /// Include the global rank prefix (1., 2., ...).
    Rank,
    /// Display the source URL header for aliases present on the page.
    Url,
    /// Prefix snippet lines with their line numbers.
    Lines,
    /// Show the hashed section anchor above the snippet.
    Anchor,
    /// Show raw BM25 scores instead of percentages.
    #[value(name = "raw-score")]
    RawScore,
}

impl std::fmt::Display for ShowComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rank => write!(f, "rank"),
            Self::Url => write!(f, "url"),
            Self::Lines => write!(f, "lines"),
            Self::Anchor => write!(f, "anchor"),
            Self::RawScore => write!(f, "raw-score"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(ShowComponent::Rank.to_string(), "rank");
        assert_eq!(ShowComponent::Url.to_string(), "url");
        assert_eq!(ShowComponent::Lines.to_string(), "lines");
        assert_eq!(ShowComponent::Anchor.to_string(), "anchor");
        assert_eq!(ShowComponent::RawScore.to_string(), "raw-score");
    }

    #[test]
    fn test_value_enum_parsing() {
        use clap::ValueEnum;

        // Test that all variants can be parsed
        let variants = ShowComponent::value_variants();
        assert_eq!(variants.len(), 5);

        // Test to_possible_value for kebab-case handling
        assert_eq!(
            ShowComponent::RawScore
                .to_possible_value()
                .unwrap()
                .get_name(),
            "raw-score"
        );
    }

    #[test]
    fn test_clone_copy() {
        let component = ShowComponent::Rank;
        let cloned = component;
        let copied: ShowComponent = component;
        assert_eq!(cloned, copied);
    }

    #[test]
    fn test_serde_uses_kebab_case() {
        // Verify serde serialization matches CLI/Display format
        assert_eq!(
            serde_json::to_string(&ShowComponent::RawScore).unwrap(),
            "\"raw-score\""
        );
        assert_eq!(
            serde_json::to_string(&ShowComponent::Rank).unwrap(),
            "\"rank\""
        );

        // Verify round-trip
        let deserialized: ShowComponent = serde_json::from_str("\"raw-score\"").unwrap();
        assert_eq!(deserialized, ShowComponent::RawScore);
    }
}
