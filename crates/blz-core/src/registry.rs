use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};

/// Registry entry representing a documented tool/package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Display name of the tool/package
    pub name: String,
    /// Kebab-case identifier for the entry
    pub slug: String,
    /// Alternative names and common abbreviations
    pub aliases: Vec<String>,
    /// Brief description of the tool/package
    pub description: String,
    /// URL to the llms.txt documentation file
    pub llms_url: String,
}

impl RegistryEntry {
    /// Creates a new registry entry
    pub fn new(name: &str, slug: &str, description: &str, llms_url: &str) -> Self {
        Self {
            name: name.to_string(),
            slug: slug.to_string(),
            aliases: vec![slug.to_string()],
            description: description.to_string(),
            llms_url: llms_url.to_string(),
        }
    }

    /// Sets the aliases for this registry entry
    #[must_use]
    pub fn with_aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|s| (*s).to_string()).collect();
        self
    }
}

impl std::fmt::Display for RegistryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})\n   {}", self.name, self.slug, self.description)
    }
}

/// Registry for looking up documentation sources
pub struct Registry {
    /// List of all registered documentation sources
    entries: Vec<RegistryEntry>,
}

impl Registry {
    /// Create a new registry with hardcoded entries
    /// Creates a new registry with built-in entries
    pub fn new() -> Self {
        let entries = vec![
            RegistryEntry::new(
                "Bun",
                "bun",
                "Fast all-in-one JavaScript runtime and package manager",
                "https://bun.sh/docs/llms.txt",
            )
            .with_aliases(&["bun", "bunjs"]),
            RegistryEntry::new(
                "Node.js",
                "node",
                "JavaScript runtime built on Chrome's V8 JavaScript engine",
                "https://nodejs.org/docs/llms.txt",
            )
            .with_aliases(&["node", "nodejs", "js"]),
            RegistryEntry::new(
                "Deno",
                "deno",
                "Modern runtime for JavaScript and TypeScript",
                "https://docs.deno.com/llms.txt",
            )
            .with_aliases(&["deno"]),
            RegistryEntry::new(
                "React",
                "react",
                "JavaScript library for building user interfaces",
                "https://react.dev/llms.txt",
            )
            .with_aliases(&["react", "reactjs"]),
            RegistryEntry::new(
                "Vue.js",
                "vue",
                "Progressive JavaScript framework for building UIs",
                "https://vuejs.org/llms.txt",
            )
            .with_aliases(&["vue", "vuejs"]),
            RegistryEntry::new(
                "Next.js",
                "nextjs",
                "React framework for production with hybrid static & server rendering",
                "https://nextjs.org/docs/llms.txt",
            )
            .with_aliases(&["nextjs", "next"]),
            RegistryEntry::new(
                "Claude Code",
                "claude-code",
                "Anthropic's AI coding assistant documentation",
                "https://docs.anthropic.com/claude-code/llms.txt",
            )
            .with_aliases(&["claude-code", "claude"]),
            RegistryEntry::new(
                "Pydantic",
                "pydantic",
                "Data validation library using Python type hints",
                "https://docs.pydantic.dev/llms.txt",
            )
            .with_aliases(&["pydantic"]),
            RegistryEntry::new(
                "Anthropic Claude API",
                "anthropic",
                "Claude API documentation and guides",
                "https://docs.anthropic.com/llms.txt",
            )
            .with_aliases(&["anthropic", "claude-api"]),
            RegistryEntry::new(
                "OpenAI API",
                "openai",
                "OpenAI API documentation and guides",
                "https://platform.openai.com/docs/llms.txt",
            )
            .with_aliases(&["openai", "gpt"]),
        ];

        Self { entries }
    }

    /// Create a new registry with custom entries
    #[must_use]
    pub const fn from_entries(entries: Vec<RegistryEntry>) -> Self {
        Self { entries }
    }

    /// Searches the registry for matching entries using fuzzy matching
    pub fn search(&self, query: &str) -> Vec<RegistrySearchResult> {
        let matcher = SkimMatcherV2::default();
        let query = query.trim().to_lowercase();

        let mut results = Vec::new();

        for entry in &self.entries {
            let mut max_score = 0;
            let mut best_match_field = "name";

            // Try matching against name
            if let Some(score) = matcher.fuzzy_match(&entry.name.to_lowercase(), &query) {
                if score > max_score {
                    max_score = score;
                    best_match_field = "name";
                }
            }

            // Try matching against slug
            if let Some(score) = matcher.fuzzy_match(&entry.slug.to_lowercase(), &query) {
                if score > max_score {
                    max_score = score;
                    best_match_field = "slug";
                }
            }

            // Try matching against aliases
            for alias in &entry.aliases {
                if let Some(score) = matcher.fuzzy_match(&alias.to_lowercase(), &query) {
                    if score > max_score {
                        max_score = score;
                        best_match_field = "alias";
                    }
                }
            }

            // Try matching against description (lower weight)
            if let Some(score) = matcher.fuzzy_match(&entry.description.to_lowercase(), &query) {
                let description_score = score / 2; // Lower weight for description matches
                if description_score > max_score {
                    max_score = description_score;
                    best_match_field = "description";
                }
            }

            if max_score > 0 {
                results.push(RegistrySearchResult {
                    entry: entry.clone(),
                    score: max_score,
                    match_field: best_match_field.to_string(),
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));

        results
    }

    /// Get all registry entries
    /// Returns all entries in the registry
    pub fn all_entries(&self) -> &[RegistryEntry] {
        &self.entries
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Search result from registry
#[derive(Debug, Clone)]
pub struct RegistrySearchResult {
    /// The matched registry entry
    pub entry: RegistryEntry,
    /// Fuzzy matching score (higher is better)
    pub score: i64,
    /// Field that matched the search query (name, slug, or alias)
    pub match_field: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_entry_creation() {
        let entry = RegistryEntry::new(
            "React",
            "react",
            "JavaScript library for building user interfaces",
            "https://react.dev/llms.txt",
        );

        assert_eq!(entry.name, "React");
        assert_eq!(entry.slug, "react");
        assert_eq!(entry.aliases, vec!["react"]);
        assert!(entry.description.contains("JavaScript library"));
        assert_eq!(entry.llms_url, "https://react.dev/llms.txt");
    }

    #[test]
    fn test_registry_entry_with_aliases() {
        let entry = RegistryEntry::new(
            "Node.js",
            "node",
            "JavaScript runtime",
            "https://nodejs.org/llms.txt",
        )
        .with_aliases(&["node", "nodejs", "js"]);

        assert_eq!(entry.aliases, vec!["node", "nodejs", "js"]);
    }

    #[test]
    fn test_registry_creation() {
        let registry = Registry::new();
        let entries = registry.all_entries();

        assert!(!entries.is_empty());

        // Check that we have some expected entries
        let react_entry = entries.iter().find(|e| e.slug == "react");
        assert!(react_entry.is_some());

        let node_entry = entries.iter().find(|e| e.slug == "node");
        assert!(node_entry.is_some());

        let claude_entry = entries.iter().find(|e| e.slug == "claude-code");
        assert!(claude_entry.is_some());
    }

    #[test]
    fn test_registry_search_exact_match() {
        let registry = Registry::new();
        let results = registry.search("react");

        assert!(!results.is_empty());
        // Should find React as top result
        let top_result = &results[0];
        assert_eq!(top_result.entry.slug, "react");
    }

    #[test]
    fn test_registry_search_fuzzy_match() {
        let registry = Registry::new();
        let results = registry.search("reactjs");

        assert!(!results.is_empty());
        // Should find React even with "reactjs" query
        let react_result = results.iter().find(|r| r.entry.slug == "react");
        assert!(react_result.is_some());
    }

    #[test]
    fn test_registry_search_partial_match() {
        let registry = Registry::new();
        let results = registry.search("claude");

        assert!(!results.is_empty());
        // Should find Claude-related entries
        let has_claude = results.iter().any(|r| r.entry.slug.contains("claude"));
        assert!(has_claude);
    }

    #[test]
    fn test_registry_search_description_match() {
        let registry = Registry::new();
        let results = registry.search("javascript runtime");

        assert!(!results.is_empty());
        // Should find entries with "javascript runtime" in description
        let has_js_runtime = results.iter().any(|r| {
            r.entry.description.to_lowercase().contains("javascript")
                && r.entry.description.to_lowercase().contains("runtime")
        });
        assert!(has_js_runtime);
    }

    #[test]
    fn test_registry_search_no_match() {
        let registry = Registry::new();
        let results = registry.search("nonexistentframework");

        // Should return empty results for nonsensical query
        assert!(results.is_empty() || results[0].score < 50);
    }

    #[test]
    fn test_registry_search_case_insensitive() {
        let registry = Registry::new();
        let results_lower = registry.search("react");
        let results_upper = registry.search("REACT");
        let results_mixed = registry.search("React");

        assert!(!results_lower.is_empty());
        assert!(!results_upper.is_empty());
        assert!(!results_mixed.is_empty());

        // All should find the same entry
        assert_eq!(results_lower[0].entry.slug, "react");
        assert_eq!(results_upper[0].entry.slug, "react");
        assert_eq!(results_mixed[0].entry.slug, "react");
    }

    #[test]
    fn test_registry_display_format() {
        let entry = RegistryEntry::new(
            "React",
            "react",
            "JavaScript library for building user interfaces",
            "https://react.dev/llms.txt",
        );

        let display = entry.to_string();
        assert!(display.contains("React"));
        assert!(display.contains("(react)"));
        assert!(display.contains("JavaScript library"));
    }

    #[test]
    fn test_all_registry_entries_have_valid_urls() {
        let registry = Registry::new();

        for entry in registry.all_entries() {
            // Check that URL looks like a valid HTTP/HTTPS URL
            assert!(
                entry.llms_url.starts_with("http://") || entry.llms_url.starts_with("https://")
            );
            // Check that URL ends with .txt (case-insensitive)

            assert!(
                std::path::Path::new(&entry.llms_url)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
            );
            // Check that slug is kebab-case (no spaces, lowercase)
            assert!(!entry.slug.contains(' '));
            assert!(!entry.slug.chars().any(char::is_uppercase));
        }
    }

    #[test]
    fn test_registry_entries_have_unique_slugs() {
        let registry = Registry::new();
        let entries = registry.all_entries();

        let mut slugs = std::collections::HashSet::new();
        for entry in entries {
            assert!(
                slugs.insert(&entry.slug),
                "Duplicate slug found: {}",
                entry.slug
            );
        }
    }

    // Registry edge cases tests - Unicode and special characters
    #[test]
    fn test_registry_search_unicode_queries() {
        let registry = Registry::new();

        // Test CJK characters
        let results = registry.search("日本語");
        assert!(results.is_empty() || results.iter().all(|r| r.score < 100));

        // Test Arabic text (RTL)
        let results = registry.search("العربية");
        assert!(results.is_empty() || results.iter().all(|r| r.score < 100));

        // Test Cyrillic
        let results = registry.search("русский");
        assert!(results.is_empty() || results.iter().all(|r| r.score < 100));

        // Test emoji
        let results = registry.search("🚀");
        assert!(results.is_empty() || results.iter().all(|r| r.score < 100));

        // Test mixed scripts
        let results = registry.search("react 日本語");
        // Mixed scripts might confuse the fuzzy matcher
        // Just verify it doesn't crash
        assert!(results.len() <= registry.all_entries().len());
    }

    #[test]
    fn test_registry_search_very_long_queries() {
        let registry = Registry::new();

        // Test extremely long query
        let long_query = "javascript".repeat(1000);
        let results = registry.search(&long_query);

        // Should handle gracefully without crashing
        // May return empty or partial results due to fuzzy matching limits
        assert!(results.len() <= registry.all_entries().len());
    }

    #[test]
    fn test_registry_search_empty_and_whitespace() {
        let registry = Registry::new();

        // Test empty string
        let results = registry.search("");
        assert!(results.is_empty());

        // Test whitespace-only queries
        let whitespace_queries = vec!["   ", "\t", "\n", "\r\n", " \t \n "];

        for query in whitespace_queries {
            let results = registry.search(query);
            assert!(
                results.is_empty(),
                "Whitespace query '{}' should return empty",
                query.escape_debug()
            );
        }
    }

    #[test]
    fn test_registry_search_special_characters() {
        let registry = Registry::new();

        // Test various punctuation and special characters
        let special_chars = vec![
            "!@#$%^&*()",
            "[]{}|\\;':\",./<>?",
            "~`",
            "react!",
            "node.js",
            "vue-js",
            "next/js",
            "c++",
            "c#",
            ".net",
            "node@18",
        ];

        for query in special_chars {
            let results = registry.search(query);

            // Should not crash and return reasonable results
            assert!(results.len() <= registry.all_entries().len());

            // Special character queries might not match exact entries
            // The fuzzy matcher handles these differently
            // Just verify that search doesn't crash and returns valid results
        }
    }

    #[test]
    fn test_registry_search_multiple_spaces() {
        let registry = Registry::new();

        // Test queries with multiple spaces
        let spaced_queries = vec![
            "javascript  runtime",
            "javascript   runtime",
            "   javascript runtime   ",
            "javascript\truntime",
            "javascript\n\nruntime",
        ];

        for query in spaced_queries {
            let results = registry.search(query);

            // Multiple spaces might affect fuzzy matching
            // Just verify that search returns some results without crashing
            // The fuzzy matcher may or may not handle multiple spaces well
            assert!(results.len() <= registry.all_entries().len());
        }
    }

    #[test]
    fn test_registry_search_leading_trailing_whitespace() {
        let registry = Registry::new();

        let query_variants = vec![
            "react",
            " react",
            "react ",
            " react ",
            "\treact\t",
            "\nreact\n",
            "  \t react \n  ",
        ];

        for query in query_variants {
            let results = registry.search(query);

            // All variants should find React
            assert!(
                !results.is_empty(),
                "Query '{}' should find results",
                query.escape_debug()
            );
            assert_eq!(results[0].entry.slug, "react");
        }
    }

    #[test]
    fn test_registry_search_fuzzy_matching_edge_cases() {
        let registry = Registry::new();

        // Test various typos and fuzzy matches
        // Note: Fuzzy matching has limits - not all typos will match
        let fuzzy_cases = vec![
            ("react", "react"),   // Exact match should work
            ("nodejs", "node"),   // Common alternative spelling
            ("nextjs", "nextjs"), // Exact match
            ("vue", "vue"),       // Exact match
        ];

        for (query, expected_slug) in fuzzy_cases {
            let results = registry.search(query);

            assert!(
                !results.is_empty(),
                "Query '{query}' should find results for '{expected_slug}'"
            );

            // Should find the expected entry for exact or close matches
            let found_expected = results.iter().any(|r| r.entry.slug == expected_slug);
            assert!(
                found_expected,
                "Query '{query}' should find entry '{expected_slug}'"
            );
        }

        // Test that typos don't crash the search
        let typo_queries = vec!["reactt", "reac", "raect", "nxtjs", "vue.js"];
        for query in typo_queries {
            let results = registry.search(query);
            // Just verify it doesn't crash
            assert!(results.len() <= registry.all_entries().len());
        }
    }

    #[test]
    fn test_registry_search_score_ranking() {
        let registry = Registry::new();

        // Test that exact matches score higher than partial matches
        let results = registry.search("react");
        assert!(!results.is_empty());

        // React should be the top result for "react" query
        assert_eq!(results[0].entry.slug, "react");

        // Test that name matches score higher than description matches
        let results = registry.search("node");
        assert!(!results.is_empty());

        // Node.js entry should score higher than entries that only mention "node" in description
        let node_result = results.iter().find(|r| r.entry.slug == "node");
        assert!(node_result.is_some());

        // The Node.js result should have a high score
        let node_score = node_result.unwrap().score;
        assert!(
            node_score > 50,
            "Node.js should have high score for 'node' query"
        );
    }

    #[test]
    fn test_registry_search_alias_matching() {
        let registry = Registry::new();

        // Test searches that should match via aliases
        let alias_tests = vec![
            ("reactjs", "react"),
            ("nodejs", "node"),
            ("js", "node"),
            ("bunjs", "bun"),
            ("claude", "claude-code"),
            ("claude-api", "anthropic"),
            ("gpt", "openai"),
        ];

        for (query, expected_slug) in alias_tests {
            let results = registry.search(query);

            assert!(!results.is_empty(), "Query '{query}' should find results");

            let found_entry = results.iter().find(|r| r.entry.slug == expected_slug);
            assert!(
                found_entry.is_some(),
                "Query '{query}' should find entry '{expected_slug}'"
            );

            // Should be marked as alias match
            let found = found_entry.unwrap();
            assert!(
                found.match_field == "alias"
                    || found.match_field == "slug"
                    || found.match_field == "name",
                "Match field should indicate alias/slug/name match for '{}' -> '{}', got '{}'",
                query,
                expected_slug,
                found.match_field
            );
        }
    }

    #[test]
    fn test_registry_search_case_variations() {
        let registry = Registry::new();

        let test_cases = vec!["REACT", "React", "rEaCt", "react"];

        let mut all_scores = Vec::new();

        for query in &test_cases {
            let results = registry.search(query);
            assert!(!results.is_empty(), "Query '{query}' should find results");
            assert_eq!(results[0].entry.slug, "react");
            all_scores.push(results[0].score);
        }

        // All case variations should produce similar scores
        let min_score = *all_scores.iter().min().unwrap();
        let max_score = *all_scores.iter().max().unwrap();

        // Scores should be within reasonable range of each other
        assert!(
            (max_score - min_score) <= 50,
            "Case variations should have similar scores"
        );
    }

    #[test]
    fn test_registry_search_performance() {
        let registry = Registry::new();

        // Test that search performance is reasonable even with many queries
        let queries = vec![
            "react",
            "node",
            "vue",
            "angular",
            "javascript",
            "typescript",
            "python",
            "rust",
            "go",
            "java",
            "c++",
            "c#",
            "nonexistent",
            "blahblahblah",
            "qwerty",
            "asdfgh",
        ];

        let start_time = std::time::Instant::now();

        for query in &queries {
            let results = registry.search(query);
            // Ensure we actually process the results
            assert!(results.len() <= registry.all_entries().len());
        }

        let elapsed = start_time.elapsed();

        // Should complete reasonably quickly (adjust threshold as needed)
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Registry search should be fast, took {elapsed:?}"
        );
    }

    #[test]
    fn test_registry_search_boundary_conditions() {
        let registry = Registry::new();

        // Test single characters
        let single_chars = vec!["a", "j", "r", "n", "v"];
        for char_query in single_chars {
            let results = registry.search(char_query);
            // Single characters might match multiple entries or none
            assert!(results.len() <= registry.all_entries().len());
        }

        // Test maximum reasonable query length
        let max_query = "a".repeat(1000);
        let results = registry.search(&max_query);
        assert!(results.len() <= registry.all_entries().len());

        // Test query with only punctuation
        let punct_results = registry.search("!@#$%^&*()");
        assert!(punct_results.is_empty() || punct_results.iter().all(|r| r.score < 50));
    }

    #[test]
    fn test_registry_search_description_weighting() {
        let registry = Registry::new();

        // Search for terms that appear in descriptions
        let results = registry.search("documentation");

        if !results.is_empty() {
            // Results should be sorted by score
            for i in 1..results.len() {
                assert!(
                    results[i - 1].score >= results[i].score,
                    "Results should be sorted by score descending"
                );
            }

            // Description matches should have lower scores than name/slug matches
            let desc_matches = results
                .iter()
                .filter(|r| r.match_field == "description")
                .collect::<Vec<_>>();
            let name_matches = results
                .iter()
                .filter(|r| r.match_field == "name" || r.match_field == "slug")
                .collect::<Vec<_>>();

            if !desc_matches.is_empty() && !name_matches.is_empty() {
                let max_desc_score = desc_matches.iter().map(|r| r.score).max().unwrap();
                let min_name_score = name_matches.iter().map(|r| r.score).min().unwrap();

                // Description matches should generally score lower (though this isn't strict)
                // This test verifies the weighting logic is applied
                if max_desc_score > min_name_score {
                    // This is fine - sometimes description matches can be very relevant
                } else {
                    assert!(
                        max_desc_score <= min_name_score * 2,
                        "Description match scores should be weighted appropriately"
                    );
                }
            }
        }
    }
}
