================================================================================
LANGUAGE VARIANT ANALYSIS REPORT
NPM Install Patterns - Anthropic Documentation Source
================================================================================

OVERVIEW
--------

This analysis examines all language variants of npm installation instructions
found in the Anthropic documentation (https://docs.anthropic.com/llms-full.txt).

A single search query ("npm install") was executed using the blz-cli tool with
JSON output to systematically identify and categorize language-specific
installation documentation patterns.

METHODOLOGY
-----------

Tool:       blz-cli (Tantivy-powered search on Rust)
Query:      "npm install"
Source:     anthropic (Anthropic documentation)
Results:    50 total matches
Date:       October 23, 2025

Language detection was performed using:
- Unicode character range analysis (CJK, Hangul, Cyrillic, Latin)
- Keyword pattern matching (language-specific indicators)
- Heading structure analysis

REPORT DOCUMENTS INCLUDED
-------------------------

1. LANGUAGE_REPORT.md
   Complete structured report with language distribution, indicator words,
   and example headings for each language variant.
   
   Contents:
   - Language distribution table
   - Top 15 indicator words per language
   - Example headings showing indicators in context
   - Key observations across languages

2. DETAILED_INDICATOR_WORDS.md
   Deep dive into specific indicator words and phrases with cross-language
   comparisons, phonetic guides, and frequency analysis.
   
   Contents:
   - Installation keywords across languages
   - Quick Start/Guide terminology
   - Step/Stage terminology
   - Language-specific core vocabulary
   - Tool/Platform terms
   - Development/Workflow terminology

3. SEARCH_SUMMARY.txt
   Executive summary with all key findings, statistics, and recommendations
   for localization teams.
   
   Contents:
   - Headline results
   - Language distribution table
   - Top indicator words (all languages)
   - Universal concepts comparison
   - Technical terminology preservation patterns
   - Example headings
   - Key findings
   - Recommendations

4. Language Distribution JSON (language_report.json)
   Machine-readable structured data suitable for tooling integration.

QUICK REFERENCE
---------------

LANGUAGES DETECTED: 10

Ranked by result count:
  1. English            23 results (46%)
  2. Chinese (Simp)      7 results (14%)
  3. Spanish             4 results (8%)
  4. Korean              3 results (6%)
  5. Portuguese          3 results (6%)
  6. Chinese (Trad)      3 results (6%)
  7. Japanese            3 results (6%)
  8. Indonesian          2 results (4%)
  9. German              1 result  (2%)
 10. French              1 result  (2%)

KEY STATISTICS

- Asian Languages:      20 results (40%)
- European Languages:   12 results (24%)
- English:              23 results (46%)

- Total indicator words extracted: 150+
- Universal concepts identified: 10
- Technical terms preserved across languages: 8

STRONGEST INDICATORS (By Frequency)

Across ALL 10 languages:

1. "Install" variants appear 36+ times
   - English: install (12), installation (5)
   - Spanish: instalar (2), instalación (2)
   - Chinese: 安装 (3) / 安裝 (3)
   - Korean: 설치 (3)
   - Indonesian: instalasi (2), instal (1)
   - Japanese: インストール (1)
   - Portuguese: instalação (1)
   - French: installer (1), installation (1)

2. "Quick Start" variants appear 10+ times
   - English: quickstart, quick start
   - Chinese: 快速入门 (simplified), 快速入門 (traditional)
   - Korean: 빠른 시작
   - Spanish: guía de inicio rápido
   - Indonesian: panduan memulai

3. "Step N" variants appear 10+ times
   - English: step
   - Spanish: paso
   - Chinese: 步骤 (simplified), 步驟 (traditional)
   - Korean: 단계
   - Indonesian: langkah
   - French: étape

UNIVERSAL TECHNICAL TERMS
(Preserved across all languages)

- NPM (Node Package Manager)
- npm install (command syntax)
- -g flag
- @anthropic-ai/claude-code (package reference)
- Claude (product name)
- SDK (Software Development Kit)
- Agent
- Bash

LOCALIZATION PATTERNS

Most documentation uses a hybrid approach:
- Technical terms kept in English (NPM, Claude Code, SDK)
- Structural/contextual words translated to local language
- Heading structure remains consistent across languages

Example (Chinese Simplified):
"快速入门 > 步骤 1：安装 Claude Code > NPM 安装"
(Quick Start > Step 1: Install Claude Code > NPM Installation)

RECOMMENDATIONS

For Localization Teams:
1. Standardize terminology using this report as reference
2. Prioritize English, Chinese (Simplified), Spanish, and Korean (74% of content)
3. Ensure all 10 language variants stay synchronized
4. Consider expanding German and French coverage
5. Maintain consistent heading structure across languages
6. Preserve technical terms and proper nouns

For Search/Discovery Teams:
1. "npm install" is a reliable anchor phrase across all languages
2. Installation terminology is highly consistent
3. Consider building language-aware search filters
4. Technical terms (NPM, Claude) are searchable in all variants

USAGE NOTES

- Report data is current as of October 23, 2025
- Search was executed on the full anthropic source (llms-full.txt)
- Results represent documentation structure, not code examples
- Some headings mix English and native language (intentional pattern)

FUTURE ANALYSIS OPPORTUNITIES

1. Expand search to other Anthropic documentation sections
2. Compare with other technical documentation sources
3. Analyze translation quality/consistency patterns
4. Build machine-readable language variant index
5. Create automated terminology validation system

================================================================================
For detailed analysis, see LANGUAGE_REPORT.md and DETAILED_INDICATOR_WORDS.md
For quick reference, see SEARCH_SUMMARY.txt
For machine integration, use language_report.json
================================================================================
