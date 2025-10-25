# Language Filter Research Notes

This directory contains research findings from systematic exploration of multilingual content in the Anthropic documentation source.

## Investigation Date
2025-10-23

## Methodology

4 parallel subagents searched the anthropic source using `blz-dev` CLI with different query patterns:
1. Installation patterns ("npm install")
2. GitHub Actions patterns
3. Plugin/marketplace patterns
4. Update/configuration patterns

## Directory Structure

### `subagent1-npm-install/`
Research on installation and setup terminology across 10 languages.

**Key findings:**
- 50 total results across 10 languages
- English (46%), Chinese Simplified (14%), Spanish (8%), others (32%)
- Strong indicators: Install, Instalar, Instalação, Installare, Menginstal

**Files:**
- `LANGUAGE_REPORT.md` - Structured analysis
- `DETAILED_INDICATOR_WORDS.md` - Terminology deep dive
- `SEARCH_SUMMARY.txt` - Executive summary
- `language_report.json` - Machine-readable data
- `README.txt` - Methodology overview

### `subagent2-github-actions/`
Action verbs and configuration terminology across 12 languages.

**Key findings:**
- Universal action verbs: Setup, Install, Configure, Add, Execute, Copy, Open
- 7 linguistic families with distinct formation patterns
- Complete translation matrices for 7 core verbs × 12 languages

**Files:**
- `FINDINGS_SUMMARY.md` - Executive summary with methodology
- `action_verbs_matrix.md` - Translation matrices (7×12)
- `github_actions_analysis.md` - Detailed linguistic patterns
- `VERBATIM_EXCERPTS.md` - Parallel documentation sections
- `README.md` - Overview guide

### `subagent3-plugins-marketplace/`
Plugin management verbs and marketplace navigation terms.

**Key findings:**
- 6 languages with high confidence detection
- Morphological patterns: French -ez endings, German -en infinitives, Indonesian meng- prefix
- Plugin verbs: install, add, manage, develop, create, share, test

**Files:**
- `README_ANTHROPIC_FINDINGS.md` - Complete reference guide
- `anthropic_findings_comprehensive.md` - Full technical documentation
- `anthropic_verb_patterns.txt` - Linguistic analysis tables
- `anthropic_multilingual_structure.txt` - Document hierarchies

### `subagent4-update-config/`
Update, configuration, and settings terminology across 8 languages.

**Key findings:**
- Configuration verbs: Enable, Disable, Configure, Set, Update, Install
- Question word patterns: German wie/warum, Spanish cómo/qué, French comment/pourquoi
- Universal plugin commands work in all languages (slash commands, env vars)

**Files:**
- `ANTHROPIC_LANGUAGE_FINDINGS.md` - Comprehensive analysis with verb tables

## Summary Statistics

**Languages Detected:**
- German (99% confidence, 2% of content)
- French (99% confidence, 2%)
- Spanish (98% confidence, 8%)
- Portuguese (98% confidence, 6%)
- Italian (97% confidence)
- Indonesian (99% confidence, 4%)
- Japanese, Korean, Chinese (CJK scripts, 26% combined)

**English Baseline:** 46% of content

**Critical Missing Indicators:** 200+ indicators across 6 languages identified

## Related Files

- `../../reports/language-filter-audit-2025-10-23.md` - Consolidated audit report
- `../../scripts/validate-language-filter.sh` - Validation script
- `../../proposals/per-language-indicator-files.md` - Modularization proposal

## Next Steps

See Linear issues:
- **Immediate expansion** (Option A): Add ~200 high-priority indicators
- **Modular refactor** (Option B): Per-language file structure
