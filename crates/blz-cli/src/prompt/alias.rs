//! Alias prompting for domain-based source addition.
//!
//! This module provides functionality to derive and prompt for source aliases
//! when adding documentation sources via domain discovery.
//!
//! # Example
//!
//! ```ignore
//! use blz_cli::prompt::alias::{AliasPrompt, AliasPromptResult};
//! use blz_core::Storage;
//!
//! // Derive an alias from a domain
//! let suggested = AliasPrompt::derive_from_domain("hono.dev");
//! assert_eq!(suggested, "hono");
//!
//! // Check availability
//! let storage = Storage::new()?;
//! if AliasPrompt::is_available(&storage, &suggested) {
//!     println!("Alias '{}' is available", suggested);
//! }
//! ```

use anyhow::Result;
use blz_core::Storage;
use blz_core::discovery::derive_alias;
use std::io::{BufRead, Write};

/// Result of alias prompting.
///
/// Used when prompting users for an alias during source discovery.
/// The actual CLI integration will be added in a future PR.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasPromptResult {
    /// The final alias to use.
    pub alias: String,
    /// Whether the user was prompted (vs. automatic).
    pub was_prompted: bool,
}

/// Alias prompting utilities for add command discovery.
///
/// This struct is used when adding sources via domain discovery (e.g., `blz add hono.dev`).
/// The actual CLI integration will be added in a future PR.
#[allow(dead_code)]
pub struct AliasPrompt;

impl AliasPrompt {
    /// Derive an alias from a domain name.
    ///
    /// Uses `blz_core::discovery::derive_alias` to extract the primary name
    /// from a domain by stripping common prefixes and extracting the main domain.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use blz_cli::prompt::alias::AliasPrompt;
    ///
    /// assert_eq!(AliasPrompt::derive_from_domain("hono.dev"), "hono");
    /// assert_eq!(AliasPrompt::derive_from_domain("docs.tanstack.com"), "tanstack");
    /// assert_eq!(AliasPrompt::derive_from_domain("react-router.com"), "react-router");
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn derive_from_domain(domain: &str) -> String {
        derive_alias(domain)
    }

    /// Check if an alias is available (not already in use).
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage instance to check against
    /// * `alias` - The alias to check
    ///
    /// # Returns
    ///
    /// `true` if the alias is available, `false` if it already exists.
    #[allow(dead_code)]
    #[must_use]
    pub fn is_available(storage: &Storage, alias: &str) -> bool {
        !storage.exists(alias)
    }

    /// Prompt the user for an alias with a suggested default.
    ///
    /// In interactive mode, displays the suggestion and allows the user to
    /// accept it (by pressing Enter) or provide a custom alias.
    ///
    /// # Arguments
    ///
    /// * `suggested` - The suggested alias
    /// * `existing` - List of existing aliases (for collision detection)
    ///
    /// # Returns
    ///
    /// The selected alias and whether the user was prompted.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from stdin fails.
    #[allow(dead_code)]
    pub fn prompt_alias(suggested: &str, existing: &[String]) -> Result<AliasPromptResult> {
        Self::prompt_alias_with_io(
            suggested,
            existing,
            &mut std::io::stdin().lock(),
            &mut std::io::stdout(),
        )
    }

    /// Internal implementation with injectable I/O for testing.
    pub(crate) fn prompt_alias_with_io<R, W>(
        suggested: &str,
        existing: &[String],
        reader: &mut R,
        writer: &mut W,
    ) -> Result<AliasPromptResult>
    where
        R: BufRead,
        W: Write,
    {
        use blz_core::discovery::{derive_alias_with_collision_check, is_valid_alias};

        // Check for collision
        let existing_refs: Vec<&str> = existing.iter().map(String::as_str).collect();
        let derivation = derive_alias_with_collision_check(suggested, &existing_refs);

        if derivation.has_collision {
            writeln!(writer, "Alias '{}' already exists.", derivation.alias)?;
            if !derivation.alternatives.is_empty() {
                writeln!(
                    writer,
                    "Suggestions: {}",
                    derivation.alternatives.join(", ")
                )?;
            }
        }

        write!(writer, "Enter alias [{suggested}]: ")?;
        writer.flush()?;

        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            // User accepted the suggestion
            Ok(AliasPromptResult {
                alias: suggested.to_string(),
                was_prompted: true,
            })
        } else {
            // User provided a custom alias
            let alias = input.to_string();
            if !is_valid_alias(&alias) {
                anyhow::bail!(
                    "Invalid alias '{alias}'. Aliases must be 2-50 lowercase alphanumeric characters with optional hyphens."
                );
            }
            Ok(AliasPromptResult {
                alias,
                was_prompted: true,
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_from_domain_simple() {
        assert_eq!(AliasPrompt::derive_from_domain("hono.dev"), "hono");
        assert_eq!(AliasPrompt::derive_from_domain("bun.sh"), "bun");
        assert_eq!(AliasPrompt::derive_from_domain("react.dev"), "react");
    }

    #[test]
    fn test_derive_from_domain_with_docs_prefix() {
        assert_eq!(
            AliasPrompt::derive_from_domain("docs.example.com"),
            "example"
        );
        assert_eq!(
            AliasPrompt::derive_from_domain("docs.tanstack.com"),
            "tanstack"
        );
    }

    #[test]
    fn test_derive_from_domain_preserves_hyphens() {
        assert_eq!(
            AliasPrompt::derive_from_domain("react-router.com"),
            "react-router"
        );
        assert_eq!(
            AliasPrompt::derive_from_domain("vue-router.vuejs.org"),
            "vue-router"
        );
    }

    #[test]
    fn test_derive_from_domain_normalizes_case() {
        assert_eq!(AliasPrompt::derive_from_domain("TanStack.com"), "tanstack");
        assert_eq!(AliasPrompt::derive_from_domain("HONO.DEV"), "hono");
    }

    #[test]
    fn test_prompt_alias_accepts_default() {
        let mut input = b"" as &[u8]; // Empty input = accept default
        let mut output = Vec::new();

        // This should accept the suggested alias
        // Note: We can't easily test this without stdin interaction,
        // but we test the derivation logic above
        let result =
            AliasPrompt::prompt_alias_with_io("hono", &[], &mut input, &mut output).unwrap();

        assert_eq!(result.alias, "hono");
        assert!(result.was_prompted);
    }

    #[test]
    fn test_prompt_alias_accepts_custom() {
        let mut input = b"custom-alias\n" as &[u8];
        let mut output = Vec::new();

        let result =
            AliasPrompt::prompt_alias_with_io("hono", &[], &mut input, &mut output).unwrap();

        assert_eq!(result.alias, "custom-alias");
        assert!(result.was_prompted);
    }

    #[test]
    fn test_prompt_alias_shows_collision_warning() {
        let mut input = b"\n" as &[u8];
        let mut output = Vec::new();
        let existing = vec!["hono".to_string()];

        let _result =
            AliasPrompt::prompt_alias_with_io("hono", &existing, &mut input, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("already exists"));
    }

    #[test]
    fn test_prompt_alias_rejects_invalid() {
        let mut input = b"INVALID_ALIAS\n" as &[u8];
        let mut output = Vec::new();

        let result = AliasPrompt::prompt_alias_with_io("hono", &[], &mut input, &mut output);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid alias"));
    }
}
