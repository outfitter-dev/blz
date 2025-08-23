use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

/// Registry entry representing a documented tool/package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub slug: String, // kebab-case identifier
    pub aliases: Vec<String>,
    pub description: String,
    pub llms_url: String,
}

impl RegistryEntry {
    pub fn new(name: &str, slug: &str, description: &str, llms_url: &str) -> Self {
        Self {
            name: name.to_string(),
            slug: slug.to_string(),
            aliases: vec![slug.to_string()],
            description: description.to_string(),
            llms_url: llms_url.to_string(),
        }
    }

    pub fn with_aliases(mut self, aliases: Vec<&str>) -> Self {
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
    entries: Vec<RegistryEntry>,
}

impl Registry {
    /// Create a new registry with hardcoded entries
    pub fn new() -> Self {
        let entries = vec![
            RegistryEntry::new(
                "Bun",
                "bun",
                "Fast all-in-one JavaScript runtime and package manager",
                "https://bun.sh/docs/llms.txt",
            )
            .with_aliases(vec!["bun", "bunjs"]),
            RegistryEntry::new(
                "Node.js",
                "node",
                "JavaScript runtime built on Chrome's V8 JavaScript engine",
                "https://nodejs.org/docs/llms.txt",
            )
            .with_aliases(vec!["node", "nodejs", "js"]),
            RegistryEntry::new(
                "Deno",
                "deno",
                "Modern runtime for JavaScript and TypeScript",
                "https://docs.deno.com/llms.txt",
            )
            .with_aliases(vec!["deno"]),
            RegistryEntry::new(
                "React",
                "react",
                "JavaScript library for building user interfaces",
                "https://react.dev/llms.txt",
            )
            .with_aliases(vec!["react", "reactjs"]),
            RegistryEntry::new(
                "Vue.js",
                "vue",
                "Progressive JavaScript framework for building UIs",
                "https://vuejs.org/llms.txt",
            )
            .with_aliases(vec!["vue", "vuejs"]),
            RegistryEntry::new(
                "Next.js",
                "nextjs",
                "React framework for production with hybrid static & server rendering",
                "https://nextjs.org/docs/llms.txt",
            )
            .with_aliases(vec!["nextjs", "next"]),
            RegistryEntry::new(
                "Claude Code",
                "claude-code",
                "Anthropic's AI coding assistant documentation",
                "https://docs.anthropic.com/claude-code/llms.txt",
            )
            .with_aliases(vec!["claude-code", "claude"]),
            RegistryEntry::new(
                "Pydantic",
                "pydantic",
                "Data validation library using Python type hints",
                "https://docs.pydantic.dev/llms.txt",
            )
            .with_aliases(vec!["pydantic"]),
            RegistryEntry::new(
                "Anthropic Claude API",
                "anthropic",
                "Claude API documentation and guides",
                "https://docs.anthropic.com/llms.txt",
            )
            .with_aliases(vec!["anthropic", "claude-api"]),
            RegistryEntry::new(
                "OpenAI API",
                "openai",
                "OpenAI API documentation and guides",
                "https://platform.openai.com/docs/llms.txt",
            )
            .with_aliases(vec!["openai", "gpt"]),
        ];

        Self { entries }
    }

    /// Search for registry entries using fuzzy matching
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
    pub entry: RegistryEntry,
    pub score: i64,
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
        .with_aliases(vec!["node", "nodejs", "js"]);

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
            // Check that URL ends with .txt
            assert!(entry.llms_url.ends_with(".txt"));
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
}
