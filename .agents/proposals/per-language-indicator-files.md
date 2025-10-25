# Proposal: Per-Language Indicator Files

**Date:** 2025-10-23
**Status:** Proposed
**Related:** language-filter-audit-2025-10-23.md

## Problem Statement

The current `language_filter.rs` has grown to ~900 lines with all indicators in two large arrays. As we expand coverage across 10+ languages, this approach has several issues:

1. **Maintainability**: Hard to review/update indicators for a specific language
2. **Testability**: Can't easily test individual language detection in isolation
3. **Documentation**: Language-specific patterns are mixed together
4. **Collaboration**: Multiple contributors editing the same file causes conflicts
5. **Performance**: Loading all indicators even when only testing a few languages

## Proposed Structure

```
crates/blz-core/src/language_filter/
├── mod.rs                      # Main filter logic (200 lines)
├── indicators/
│   ├── mod.rs                  # Indicator loading & trait (100 lines)
│   ├── german.rs               # German indicators (150 lines)
│   ├── french.rs               # French indicators (150 lines)
│   ├── spanish.rs              # Spanish indicators (150 lines)
│   ├── portuguese.rs           # Portuguese indicators (150 lines)
│   ├── italian.rs              # Italian indicators (150 lines)
│   ├── indonesian.rs           # Indonesian indicators (100 lines)
│   ├── dutch.rs                # Dutch indicators (80 lines)
│   ├── polish.rs               # Polish indicators (80 lines)
│   └── cjk.rs                  # CJK script detection (100 lines)
└── tests/
    ├── mod.rs                  # Test infrastructure
    ├── german_test.rs          # German-specific tests
    ├── french_test.rs          # French-specific tests
    ├── spanish_test.rs         # Spanish-specific tests
    ├── portuguese_test.rs      # Portuguese-specific tests
    ├── italian_test.rs         # Italian-specific tests
    ├── indonesian_test.rs      # Indonesian-specific tests
    └── anthropic_validation.rs # Real-world validation tests
```

**Total:** ~1,500 lines split across 20+ files vs. 900 lines in 1 file

## Per-Language File Structure

Each language indicator file follows this template:

```rust
//! German language detection indicators
//!
//! ## Morphological Patterns
//! - Capitalized nouns (Plugins, Einstellungen, Entwickler)
//! - -en infinitive endings (installieren, verwalten, erstellen)
//! - Umlauts (ü, ö, ä)
//! - Compound words (Kontextfenster, Netzwerkrichtlinie)
//!
//! ## Common False Positives to Avoid
//! - "container" (English word)
//! - "design" (contains "de" but is English)
//!
//! ## Update History
//! - 2025-10-23: Added 23 new indicators from Anthropic audit
//! - 2025-10-15: Initial set of 10 indicators

use super::LanguageIndicators;

/// Strong indicators that rarely appear in English text.
/// A single match is enough to flag text as German.
const STRONG: &[&str] = &[
    // Installation/Setup verbs
    "installieren",
    "einrichten",
    "hinzufügen",
    "konfigurieren",

    // Management verbs
    "verwalten",
    "entwickeln",
    "erstellen",
    "teilen",
    "organisieren",

    // Update/Configuration verbs
    "aktualisieren",
    "aktivieren",
    "deaktivieren",
    "einstellen",

    // Question words
    "warum",
    "wann",

    // Technical compound nouns
    "kontextfenster",
    "umgebung",
    "netzwerkrichtlinie",

    // Documentation terms
    "anleitung",
    "leitfaden",
    "schnellstart",
    "überblick",
    "schritt",

    // Other strong indicators
    "wie",
    "klar",
    "direkt",
    "detailliert",
    "spezifisch",
    "kontextbezogen",
    "verstehen",
    "funktioniert",
    "implementieren",
    "nutzung",
    "richtlinie",
    "netzwerk",
    "zweite",
    "ausgabe",
    "kontrolle",
    "befehle",
    "benutzerdefinierte",
    "dokumentation",
    "praktische",
    "marktplätze",
    "troubleshooten",
    "validierung",
    "testen",
    "umbenennung",
    "anfrage",
    "anfragen",
    "verwenden",
    "verwendet",
    "wiederverwenden",
    "erzwingen",
    "abrufen",
];

/// Weak indicators that are common function words.
/// Requires 2+ matches combined with strong indicators.
const WEAK: &[&str] = &[
    "und",
    "der",
    "die",
    "das",
    "für",
    "mit",
    "von",
    "zur",
    "im",
    "man",
    "sei",
];

pub struct GermanIndicators;

impl LanguageIndicators for GermanIndicators {
    fn name(&self) -> &'static str {
        "German"
    }

    fn strong_indicators(&self) -> &'static [&'static str] {
        STRONG
    }

    fn weak_indicators(&self) -> &'static [&'static str] {
        WEAK
    }

    fn has_characteristic_script(&self, _text: &str) -> bool {
        // German uses Latin script, no special detection needed
        false
    }

    fn has_characteristic_diacritics(&self, text: &str) -> bool {
        // Check for German umlauts
        text.chars().any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'Ä' | 'Ö' | 'Ü' | 'ß'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_german_headings() {
        let filter = LanguageFilter::with_indicators(vec![
            Box::new(GermanIndicators),
        ]);

        // Should detect as German
        assert!(!filter.is_english_text("Das Kontextfenster verstehen"));
        assert!(!filter.is_english_text("Plugins installieren und verwalten"));
        assert!(!filter.is_english_text("Schnellstart-Anleitung für Entwickler"));

        // Should pass as English
        assert!(filter.is_english_text("Container Management"));
        assert!(filter.is_english_text("Design Patterns"));
    }
}
```

## Trait Definition

```rust
// In indicators/mod.rs

/// Trait for language-specific indicator sets
pub trait LanguageIndicators: Send + Sync {
    /// Language name for reporting
    fn name(&self) -> &'static str;

    /// Strong indicators (1 match = non-English)
    fn strong_indicators(&self) -> &'static [&'static str];

    /// Weak indicators (need 2+ matches)
    fn weak_indicators(&self) -> &'static [&'static str];

    /// Check for characteristic scripts (e.g., CJK, Cyrillic)
    fn has_characteristic_script(&self, text: &str) -> bool;

    /// Check for characteristic diacritics (e.g., German umlauts, French accents)
    fn has_characteristic_diacritics(&self, text: &str) -> bool;
}

/// Registry of all language indicator sets
pub struct IndicatorRegistry {
    languages: Vec<Box<dyn LanguageIndicators>>,
}

impl Default for IndicatorRegistry {
    fn default() -> Self {
        Self {
            languages: vec![
                Box::new(GermanIndicators),
                Box::new(FrenchIndicators),
                Box::new(SpanishIndicators),
                Box::new(PortugueseIndicators),
                Box::new(ItalianIndicators),
                Box::new(IndonesianIndicators),
                Box::new(DutchIndicators),
                Box::new(PolishIndicators),
                Box::new(CjkIndicators),
            ],
        }
    }
}
```

## Updated Main Filter

```rust
// In mod.rs

pub struct LanguageFilter {
    enabled: bool,
    registry: IndicatorRegistry,
    stats: FilterStats,
}

impl LanguageFilter {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            registry: IndicatorRegistry::default(),
            stats: FilterStats::default(),
        }
    }

    pub fn is_english_text(&self, text: &str) -> bool {
        if !self.enabled {
            return true;
        }

        // Quick reject for non-Latin scripts
        for lang in &self.registry.languages {
            if lang.has_characteristic_script(text) {
                return false;
            }
        }

        // Check diacritics
        for lang in &self.registry.languages {
            if lang.has_characteristic_diacritics(text) {
                return false;
            }
        }

        // Count indicators across all languages
        let lower_text = text.to_lowercase();
        for lang in &self.registry.languages {
            let counts = self.count_indicators(&lower_text, lang.as_ref());

            // 1 strong indicator = non-English
            if counts.strong >= 1 {
                return false;
            }

            // 2+ total indicators = non-English
            if (counts.strong + counts.weak) >= 2 {
                return false;
            }
        }

        true
    }

    fn count_indicators(&self, text: &str, lang: &dyn LanguageIndicators) -> IndicatorCounts {
        let mut counts = IndicatorCounts::default();

        for word in text.split(|c: char| !c.is_alphabetic()) {
            if word.is_empty() {
                continue;
            }

            if lang.strong_indicators().contains(&word) {
                counts.strong += 1;
            } else if lang.weak_indicators().contains(&word) {
                counts.weak += 1;
            }
        }

        counts
    }
}
```

## Migration Strategy

### Phase 1: Create Infrastructure (Week 1)
1. Create `indicators/mod.rs` with trait definition
2. Create `indicators/german.rs` by extracting existing German indicators
3. Update tests to ensure no regression
4. Update main `mod.rs` to use the new structure

### Phase 2: Extract Existing Languages (Week 2)
1. Extract French, Spanish, Portuguese, Italian indicators
2. Extract Indonesian, Dutch, Polish indicators
3. Create CJK script detection module
4. Update all tests

### Phase 3: Expand Coverage (Week 3-4)
1. Add missing indicators from audit findings
2. Create per-language test files
3. Add real-world validation tests using anthropic source
4. Document indicator selection criteria

### Phase 4: Continuous Improvement (Ongoing)
1. Run validation script weekly
2. Review failures and add missing indicators
3. Update documentation with patterns discovered

## Benefits

### For Development
- **Parallel work**: Multiple contributors can work on different languages without conflicts
- **Focused reviews**: PRs only touch relevant language files
- **Clear ownership**: Language experts can own specific files

### For Testing
- **Isolated tests**: Test German detection without worrying about French
- **Fast iteration**: Change German indicators, run only German tests
- **Real-world validation**: Test against actual anthropic headings per language

### For Performance
- **Lazy loading**: Only load indicators for languages actually needed
- **Optimized search**: Stop after first language match
- **Cacheable**: Compile-time optimization of indicator arrays

### For Documentation
- **Self-documenting**: Each file documents its language's patterns
- **Update history**: Track when/why indicators were added
- **False positives**: Document known edge cases per language

## Success Metrics

### Quantitative
- Pass rate on anthropic validation: **>95%** (currently ~60%)
- Test coverage per language: **>90%**
- Build time increase: **<10%**

### Qualitative
- Contributors can add new language support in <1 hour
- Language-specific PRs don't conflict with other language work
- New indicators are self-documented with examples

## Next Steps

1. **Create RFC** for team review
2. **Implement Phase 1** as proof of concept
3. **Measure performance impact** before proceeding
4. **Get feedback** from maintainers
5. **Execute migration** if approved

## Open Questions

1. Should we use a const array or lazy_static for indicator sets?
2. Do we want runtime configuration to enable/disable specific languages?
3. Should diacritical marks be checked before word indicators for performance?
4. How do we handle language variants (PT-BR vs PT-PT, ES-MX vs ES-ES)?
