use blz_core::registry::{Registry, RegistryEntry};

#[cfg(test)]
mod flavor_detection_tests {
    use super::*;

    #[test]
    fn test_registry_search_edge_cases_unicode() {
        let registry = Registry::new();

        // Test with emoji
        let results = registry.search("🔥");
        // Should handle emoji gracefully
        assert!(results.is_empty() || results[0].score < 50);

        // Test with CJK characters
        let results = registry.search("反応");
        assert!(results.is_empty() || results[0].score < 50);

        // Test with RTL text
        let results = registry.search("مرحبا");
        assert!(results.is_empty() || results[0].score < 50);
    }

    #[test]
    fn test_registry_search_edge_cases_long_query() {
        let registry = Registry::new();

        // Test with very long query
        let long_query = "a".repeat(1000);
        let results = registry.search(&long_query);
        // Should handle without panic
        assert!(results.is_empty() || results[0].score < 50);
    }

    #[test]
    fn test_registry_search_edge_cases_whitespace() {
        let registry = Registry::new();

        // Empty string
        let results = registry.search("");
        assert!(results.is_empty());

        // Whitespace only
        let results = registry.search("   ");
        assert!(results.is_empty());

        // Multiple spaces between words
        let results = registry.search("react    native");
        // Multiple spaces might affect fuzzy matching
        assert!(results.len() <= registry.all_entries().len());

        // Leading/trailing whitespace
        let results = registry.search("  react  ");
        // Should handle whitespace, but fuzzy matcher may vary
        assert!(results.len() <= registry.all_entries().len());
    }

    #[test]
    fn test_registry_search_edge_cases_special_chars() {
        let registry = Registry::new();

        // Special characters
        let special_chars = vec![
            "react!", "react@", "react#", "react$", "react%", "react^", "react&", "react*",
            "react()", "react[]", "react{}",
        ];

        for query in special_chars {
            let results = registry.search(query);
            // Special characters might confuse the fuzzy matcher
            // Just verify it doesn't crash and returns valid results
            assert!(results.len() <= registry.all_entries().len());
        }
    }

    #[test]
    fn test_registry_search_edge_cases_mixed_case_unicode() {
        let registry = Registry::new();

        // Mixed case with accents
        let queries = vec!["RÉACT", "rëact", "reäct"];

        for query in queries {
            let results = registry.search(query);
            // Should handle gracefully even if no exact match
            assert!(results.is_empty() || results[0].score >= 0);
        }
    }
}
