//! Input validation utilities

use anyhow::Result;

use super::constants::RESERVED_KEYWORDS;

/// Normalize an alias to kebab-case lowercase
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(normalize_alias("Vercel AI SDK"), "vercel-ai-sdk");
/// assert_eq!(normalize_alias("React_Native"), "react-native");
/// assert_eq!(normalize_alias("NextJS 14"), "nextjs-14");
/// assert_eq!(normalize_alias("My__Cool___Tool"), "my-cool-tool");
/// ```
pub fn normalize_alias(alias: &str) -> String {
    alias
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() || c == '_' || c == '-' {
                '-'
            } else {
                // Skip other characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        // Collapse multiple dashes into one
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Validate an alias to ensure it's not a reserved keyword
pub fn validate_alias(alias: &str) -> Result<()> {
    if RESERVED_KEYWORDS.contains(&alias.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!(
            "Alias '{}' is reserved. Reserved keywords: {}",
            alias,
            RESERVED_KEYWORDS.join(", ")
        ));
    }
    Ok(())
}

/// Validate a relaxed "metadata alias" that is not used as a directory name.
///
/// Allows formats like `@scope/package` and other characters that are safe
/// for in-memory resolution but not for filesystem paths.
///
/// Rules:
/// - Non-empty, no whitespace
/// - Length <= 100
/// - Disallow control characters
/// - Disallow reserved keywords used by the CLI
pub fn validate_relaxed_alias(alias: &str) -> Result<()> {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("Alias cannot be empty"));
    }
    if trimmed.len() > 100 {
        return Err(anyhow::anyhow!("Alias exceeds maximum length (100)"));
    }
    if trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(anyhow::anyhow!(
            "Alias cannot contain whitespace or control characters"
        ));
    }

    // Reserve core CLI keywords to avoid confusion
    if RESERVED_KEYWORDS.contains(&trimmed.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!(
            "Alias '{}' is reserved. Reserved keywords: {}",
            alias,
            RESERVED_KEYWORDS.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_alias_basic() {
        assert_eq!(normalize_alias("hello"), "hello");
        assert_eq!(normalize_alias("Hello"), "hello");
        assert_eq!(normalize_alias("HELLO"), "hello");
    }

    #[test]
    fn test_normalize_alias_with_spaces() {
        assert_eq!(normalize_alias("Vercel AI SDK"), "vercel-ai-sdk");
        assert_eq!(normalize_alias("React Native"), "react-native");
        assert_eq!(normalize_alias("Next JS"), "next-js");
    }

    #[test]
    fn test_normalize_alias_with_underscores() {
        assert_eq!(normalize_alias("react_native"), "react-native");
        assert_eq!(normalize_alias("my_cool_tool"), "my-cool-tool");
        assert_eq!(normalize_alias("SNAKE_CASE"), "snake-case");
    }

    #[test]
    fn test_normalize_alias_with_numbers() {
        assert_eq!(normalize_alias("NextJS 14"), "nextjs-14");
        assert_eq!(normalize_alias("Vue3"), "vue3");
        assert_eq!(normalize_alias("Angular 2.0"), "angular-20");
    }

    #[test]
    fn test_normalize_alias_multiple_separators() {
        assert_eq!(normalize_alias("My__Cool___Tool"), "my-cool-tool");
        assert_eq!(normalize_alias("a---b---c"), "a-b-c");
        assert_eq!(normalize_alias("  spaced   out  "), "spaced-out");
    }

    #[test]
    fn test_normalize_alias_special_characters() {
        assert_eq!(normalize_alias("React@18"), "react18");
        assert_eq!(normalize_alias("Node.js"), "nodejs");
        assert_eq!(normalize_alias("C++"), "c");
        assert_eq!(normalize_alias("My Tool (v2)"), "my-tool-v2");
    }

    #[test]
    fn test_normalize_alias_mixed_cases() {
        assert_eq!(normalize_alias("CamelCase"), "camelcase");
        assert_eq!(normalize_alias("PascalCase"), "pascalcase");
        assert_eq!(
            normalize_alias("mixedCASE_with-STUFF"),
            "mixedcase-with-stuff"
        );
    }

    #[test]
    fn test_normalize_alias_edge_cases() {
        assert_eq!(normalize_alias(""), "");
        assert_eq!(normalize_alias("-"), "");
        assert_eq!(normalize_alias("___"), "");
        assert_eq!(normalize_alias("   "), "");
        assert_eq!(normalize_alias("@#$%"), "");
    }

    #[test]
    fn test_normalize_alias_leading_trailing() {
        assert_eq!(normalize_alias("-hello-"), "hello");
        assert_eq!(normalize_alias("_world_"), "world");
        assert_eq!(normalize_alias("  test  "), "test");
        assert_eq!(normalize_alias("--multi--dash--"), "multi-dash");
    }

    #[test]
    fn test_validate_alias_reserved() {
        // These should fail (reserved keywords)
        assert!(validate_alias("all").is_err());
        assert!(validate_alias("ALL").is_err());
        assert!(validate_alias("search").is_err());

        // These should pass
        assert!(validate_alias("my-app").is_ok());
        assert!(validate_alias("react").is_ok());
        assert!(validate_alias("bun").is_ok());
    }
}
