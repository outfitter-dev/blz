//! Alias derivation from domain names.
//!
//! This module provides functionality to automatically derive source aliases
//! from domain names, with collision detection and alternative suggestions.
//!
//! ## Quick Start
//!
//! ```rust
//! use blz_core::discovery::alias::{derive_alias, is_valid_alias, derive_alias_with_collision_check};
//!
//! // Simple alias derivation
//! assert_eq!(derive_alias("hono.dev"), "hono");
//! assert_eq!(derive_alias("docs.react.dev"), "react");
//!
//! // Validation
//! assert!(is_valid_alias("hono"));
//! assert!(!is_valid_alias("123"));  // Can't start with number
//!
//! // With collision checking
//! let existing = ["react", "vue"];
//! let result = derive_alias_with_collision_check("hono.dev", &existing);
//! assert_eq!(result.alias, "hono");
//! assert!(!result.has_collision);
//! ```
//!
//! ## Alias Rules
//!
//! Valid aliases must:
//! - Be 2-50 characters long
//! - Start with a lowercase letter
//! - Contain only lowercase alphanumeric characters and hyphens
//! - Not have consecutive hyphens
//! - Not end with a hyphen

/// Result of alias derivation with collision information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasDerivation {
    /// The derived alias.
    pub alias: String,
    /// Whether this collides with an existing alias.
    pub has_collision: bool,
    /// Alternative suggestions if there's a collision.
    pub alternatives: Vec<String>,
}

/// Common prefixes to strip from domain names.
const STRIP_PREFIXES: &[&str] = &[
    "docs.", "www.", "api.", "dev.", "beta.", "staging.", "blog.", "help.", "support.", "learn.",
];

/// Derive an alias from a domain name.
///
/// Extracts the primary name from a domain by:
/// 1. Stripping common prefixes (docs., www., api., etc.)
/// 2. Extracting the primary domain name before the TLD
/// 3. Normalizing to lowercase
/// 4. Preserving hyphens in domain names
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::alias::derive_alias;
///
/// assert_eq!(derive_alias("hono.dev"), "hono");
/// assert_eq!(derive_alias("docs.hono.dev"), "hono");
/// assert_eq!(derive_alias("react-router.com"), "react-router");
/// assert_eq!(derive_alias("TanStack.com"), "tanstack");
/// ```
#[must_use]
pub fn derive_alias(domain: &str) -> String {
    let domain = domain.to_lowercase();

    // Strip common prefixes
    let mut stripped = domain.as_str();
    for prefix in STRIP_PREFIXES {
        if let Some(rest) = stripped.strip_prefix(prefix) {
            stripped = rest;
            break; // Only strip one prefix
        }
    }

    // Split by dots and extract the primary name
    // For "hono.dev" -> "hono"
    // For "vue-router.vuejs.org" -> "vue-router"
    let parts: Vec<&str> = stripped.split('.').collect();

    if parts.is_empty() {
        return String::new();
    }

    // If there's only one part (shouldn't happen with real domains), return it
    if parts.len() == 1 {
        return normalize_alias_string(parts[0]);
    }

    // Get the first part (primary name) before the TLD
    let primary = parts[0];

    normalize_alias_string(primary)
}

/// Normalize a string to be a valid alias.
///
/// - Converts to lowercase
/// - Replaces non-alphanumeric chars (except hyphen) with hyphen
/// - Collapses consecutive hyphens
/// - Trims leading/trailing hyphens
fn normalize_alias_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_hyphen = true; // Start true to skip leading hyphens

    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            result.push('-');
            last_was_hyphen = true;
        }
    }

    // Remove trailing hyphen
    while result.ends_with('-') {
        result.pop();
    }

    result
}

/// Validate that an alias follows the rules.
///
/// Valid aliases must:
/// - Be 2-50 characters long
/// - Start with a lowercase letter
/// - Contain only lowercase alphanumeric characters and hyphens
/// - Not have consecutive hyphens
/// - Not end with a hyphen
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::alias::is_valid_alias;
///
/// assert!(is_valid_alias("hono"));
/// assert!(is_valid_alias("react-router"));
/// assert!(is_valid_alias("vue3"));
/// assert!(is_valid_alias("ai"));  // 2 chars minimum
///
/// assert!(!is_valid_alias(""));     // Too short
/// assert!(!is_valid_alias("a"));    // Too short
/// assert!(!is_valid_alias("123"));  // Starts with number
/// assert!(!is_valid_alias("HoNo")); // Uppercase
/// ```
#[must_use]
pub fn is_valid_alias(alias: &str) -> bool {
    // Length check: 2-50 characters
    if alias.len() < 2 || alias.len() > 50 {
        return false;
    }

    let mut chars = alias.chars();

    // First character must be a lowercase letter
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {},
        _ => return false,
    }

    let mut prev_was_hyphen = false;

    for c in chars {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_was_hyphen = false;
        } else if c == '-' {
            // No consecutive hyphens
            if prev_was_hyphen {
                return false;
            }
            prev_was_hyphen = true;
        } else {
            // Invalid character (uppercase, special chars, etc.)
            return false;
        }
    }

    // Cannot end with hyphen
    !prev_was_hyphen
}

/// Check if alias collides with existing aliases.
///
/// Performs case-insensitive comparison.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::alias::has_collision;
///
/// let existing = ["react", "vue", "angular"];
/// assert!(has_collision("react", &existing));
/// assert!(has_collision("React", &existing));  // Case insensitive
/// assert!(!has_collision("svelte", &existing));
/// ```
#[must_use]
pub fn has_collision(alias: &str, existing: &[&str]) -> bool {
    let alias_lower = alias.to_lowercase();
    existing.iter().any(|e| e.to_lowercase() == alias_lower)
}

/// Derive alias with collision checking and alternative suggestions.
///
/// If the derived alias collides with an existing one, generates alternative
/// suggestions by appending suffixes like `-dev`, `-docs`, `-2`, `-3`.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::alias::derive_alias_with_collision_check;
///
/// // No collision
/// let result = derive_alias_with_collision_check("hono.dev", &["react", "vue"]);
/// assert_eq!(result.alias, "hono");
/// assert!(!result.has_collision);
/// assert!(result.alternatives.is_empty());
///
/// // With collision
/// let result = derive_alias_with_collision_check("react.dev", &["react", "vue"]);
/// assert_eq!(result.alias, "react");
/// assert!(result.has_collision);
/// assert!(!result.alternatives.is_empty());
/// ```
#[must_use]
pub fn derive_alias_with_collision_check(domain: &str, existing: &[&str]) -> AliasDerivation {
    let alias = derive_alias(domain);
    let collision = has_collision(&alias, existing);

    let alternatives = if collision {
        generate_alternatives(&alias, existing)
    } else {
        Vec::new()
    };

    AliasDerivation {
        alias,
        has_collision: collision,
        alternatives,
    }
}

/// Generate alternative alias suggestions.
///
/// Tries suffixes in order: -dev, -docs, -2, -3, -4, ...
/// Returns up to 5 unique alternatives that don't collide.
fn generate_alternatives(base: &str, existing: &[&str]) -> Vec<String> {
    let suffixes = [
        "-dev", "-docs", "-2", "-3", "-4", "-5", "-6", "-7", "-8", "-9",
    ];
    let mut alternatives = Vec::new();

    for suffix in suffixes {
        let candidate = format!("{base}{suffix}");
        if is_valid_alias(&candidate) && !has_collision(&candidate, existing) {
            alternatives.push(candidate);
            if alternatives.len() >= 5 {
                break;
            }
        }
    }

    alternatives
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;

    // ============================================
    // derive_alias tests
    // ============================================

    #[test]
    fn test_simple_domain() {
        assert_eq!(derive_alias("hono.dev"), "hono");
        assert_eq!(derive_alias("tanstack.com"), "tanstack");
        assert_eq!(derive_alias("bun.sh"), "bun");
    }

    #[test]
    fn test_strips_docs_prefix() {
        assert_eq!(derive_alias("docs.hono.dev"), "hono");
        assert_eq!(derive_alias("docs.example.com"), "example");
    }

    #[test]
    fn test_strips_www_prefix() {
        assert_eq!(derive_alias("www.example.com"), "example");
    }

    #[test]
    fn test_strips_common_prefixes() {
        assert_eq!(derive_alias("api.example.com"), "example");
        assert_eq!(derive_alias("dev.example.com"), "example");
        assert_eq!(derive_alias("beta.example.com"), "example");
        assert_eq!(derive_alias("staging.example.com"), "example");
        assert_eq!(derive_alias("blog.example.com"), "example");
        assert_eq!(derive_alias("help.example.com"), "example");
        assert_eq!(derive_alias("support.example.com"), "example");
        assert_eq!(derive_alias("learn.example.com"), "example");
    }

    #[test]
    fn test_handles_subdomains() {
        // "blog.hono.dev" - keep as "hono" since it's the main product
        assert_eq!(derive_alias("blog.hono.dev"), "hono");
    }

    #[test]
    fn test_preserves_hyphens() {
        assert_eq!(derive_alias("react-router.com"), "react-router");
        assert_eq!(derive_alias("vue-router.vuejs.org"), "vue-router");
    }

    #[test]
    fn test_normalizes_to_lowercase() {
        assert_eq!(derive_alias("EXAMPLE.COM"), "example");
        assert_eq!(derive_alias("TanStack.com"), "tanstack");
        assert_eq!(derive_alias("React-Router.IO"), "react-router");
    }

    #[test]
    fn test_handles_various_tlds() {
        assert_eq!(derive_alias("example.io"), "example");
        assert_eq!(derive_alias("example.org"), "example");
        assert_eq!(derive_alias("example.app"), "example");
        assert_eq!(derive_alias("example.ai"), "example");
        assert_eq!(derive_alias("example.co.uk"), "example");
    }

    #[test]
    fn test_handles_edge_cases() {
        // Single part (unusual but handle gracefully)
        assert_eq!(derive_alias("localhost"), "localhost");
        // Empty string
        assert_eq!(derive_alias(""), "");
    }

    // ============================================
    // is_valid_alias tests
    // ============================================

    #[test]
    fn test_valid_aliases() {
        assert!(is_valid_alias("hono"));
        assert!(is_valid_alias("react-router"));
        assert!(is_valid_alias("vue3"));
        assert!(is_valid_alias("ai")); // 2 chars is minimum
        assert!(is_valid_alias("tanstack"));
        assert!(is_valid_alias("next-js"));
        assert!(is_valid_alias("react-query-v5"));
    }

    #[test]
    fn test_invalid_aliases_too_short() {
        assert!(!is_valid_alias("")); // empty
        assert!(!is_valid_alias("a")); // 1 char
    }

    #[test]
    fn test_invalid_aliases_starts_with_number() {
        assert!(!is_valid_alias("123"));
        assert!(!is_valid_alias("3js"));
        assert!(!is_valid_alias("1password"));
    }

    #[test]
    fn test_invalid_aliases_starts_with_hyphen() {
        assert!(!is_valid_alias("-hono"));
        assert!(!is_valid_alias("-react-router"));
    }

    #[test]
    fn test_invalid_aliases_ends_with_hyphen() {
        assert!(!is_valid_alias("hono-"));
        assert!(!is_valid_alias("react-"));
    }

    #[test]
    fn test_invalid_aliases_consecutive_hyphens() {
        assert!(!is_valid_alias("ho--no"));
        assert!(!is_valid_alias("react---router"));
    }

    #[test]
    fn test_invalid_aliases_special_chars() {
        assert!(!is_valid_alias("ho.no")); // dot
        assert!(!is_valid_alias("ho no")); // space
        assert!(!is_valid_alias("ho_no")); // underscore
        assert!(!is_valid_alias("ho@no")); // at sign
        assert!(!is_valid_alias("ho/no")); // slash
    }

    #[test]
    fn test_invalid_aliases_uppercase() {
        assert!(!is_valid_alias("HoNo"));
        assert!(!is_valid_alias("HONO"));
        assert!(!is_valid_alias("hoNo"));
    }

    #[test]
    fn test_invalid_aliases_too_long() {
        let long_alias = "a".repeat(51);
        assert!(!is_valid_alias(&long_alias));
    }

    #[test]
    fn test_valid_aliases_boundary_length() {
        // 2 chars (minimum)
        assert!(is_valid_alias("ab"));
        // 50 chars (maximum)
        let max_alias = "a".repeat(50);
        assert!(is_valid_alias(&max_alias));
    }

    // ============================================
    // has_collision tests
    // ============================================

    #[test]
    fn test_collision_detection() {
        let existing = ["react", "vue", "angular"];
        assert!(has_collision("react", &existing));
        assert!(has_collision("vue", &existing));
        assert!(has_collision("angular", &existing));
        assert!(!has_collision("svelte", &existing));
        assert!(!has_collision("solid", &existing));
    }

    #[test]
    fn test_collision_case_insensitive() {
        let existing = ["react", "vue", "angular"];
        assert!(has_collision("React", &existing));
        assert!(has_collision("REACT", &existing));
        assert!(has_collision("VUE", &existing));
    }

    #[test]
    fn test_collision_empty_existing() {
        let existing: [&str; 0] = [];
        assert!(!has_collision("anything", &existing));
    }

    // ============================================
    // derive_alias_with_collision_check tests
    // ============================================

    #[test]
    fn test_no_collision() {
        let result = derive_alias_with_collision_check("hono.dev", &["react", "vue"]);
        assert_eq!(result.alias, "hono");
        assert!(!result.has_collision);
        assert!(result.alternatives.is_empty());
    }

    #[test]
    fn test_with_collision() {
        let result = derive_alias_with_collision_check("react.dev", &["react", "vue"]);
        assert_eq!(result.alias, "react");
        assert!(result.has_collision);
        assert!(!result.alternatives.is_empty());
        // Should suggest alternatives like "react-dev", "react-docs", "react-2"
        assert!(result.alternatives.iter().any(|a| a.starts_with("react-")));
    }

    #[test]
    fn test_alternatives_generation() {
        let result =
            derive_alias_with_collision_check("docs.react.dev", &["react", "react-dev", "react-2"]);
        assert_eq!(result.alias, "react");
        assert!(result.has_collision);

        // All alternatives should be valid and not collide
        for alt in &result.alternatives {
            assert!(is_valid_alias(alt), "Alternative '{alt}' should be valid");
            assert!(
                !["react", "react-dev", "react-2"].contains(&alt.as_str()),
                "Alternative '{alt}' should not collide"
            );
        }
    }

    #[test]
    fn test_alternatives_max_count() {
        // Even with many collisions, should return at most 5 alternatives
        let existing = [
            "test",
            "test-dev",
            "test-docs",
            "test-2",
            "test-3",
            "test-4",
            "test-5",
        ];
        let result = derive_alias_with_collision_check("test.com", &existing);
        assert!(result.alternatives.len() <= 5);
    }

    #[test]
    fn test_alternatives_are_unique() {
        let result = derive_alias_with_collision_check("react.dev", &["react"]);
        let mut seen = std::collections::HashSet::new();
        for alt in &result.alternatives {
            assert!(seen.insert(alt), "Duplicate alternative: {alt}");
        }
    }

    // ============================================
    // Integration tests
    // ============================================

    #[test]
    fn test_real_world_domains() {
        // Common documentation sites
        assert_eq!(derive_alias("react.dev"), "react");
        assert_eq!(derive_alias("vuejs.org"), "vuejs");
        assert_eq!(derive_alias("angular.io"), "angular");
        assert_eq!(derive_alias("svelte.dev"), "svelte");
        assert_eq!(derive_alias("docs.solidjs.com"), "solidjs");
        assert_eq!(derive_alias("nextjs.org"), "nextjs");
        assert_eq!(derive_alias("remix.run"), "remix");
        assert_eq!(derive_alias("astro.build"), "astro");
        assert_eq!(derive_alias("kit.svelte.dev"), "kit");
    }

    #[test]
    fn test_normalize_alias_string() {
        assert_eq!(normalize_alias_string("Hello"), "hello");
        assert_eq!(normalize_alias_string("hello-world"), "hello-world");
        assert_eq!(normalize_alias_string("hello--world"), "hello-world");
        assert_eq!(normalize_alias_string("-hello-"), "hello");
        assert_eq!(normalize_alias_string("hello_world"), "hello-world");
        assert_eq!(normalize_alias_string("hello.world"), "hello-world");
    }
}
