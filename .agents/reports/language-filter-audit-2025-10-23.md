# Language Filter Audit - Anthropic Source
**Date:** 2025-10-23
**Source:** anthropic llms-full.txt (881,070 lines, 24,640 headings)
**Method:** 4 parallel subagent searches using blz-dev CLI

## Executive Summary

Systematic search of the Anthropic documentation revealed **10 languages** with comprehensive translations, requiring significant expansion of our language filter indicators.

### Languages Detected (Confidence Scores)
1. **German** (99%) - 2% of content, strong morphological patterns
2. **French** (99%) - 2% of content, distinctive -ez endings
3. **Spanish** (98%) - 8% of content, clear verb patterns
4. **Portuguese** (98%) - 6% of content, Brazilian variant dominant
5. **Italian** (97%) - high confidence, distinct infinitives
6. **Indonesian** (99%) - 4% of content, meng- prefix pattern
7. **Japanese** (95%) - 6% of content, distinct scripts
8. **Korean** (95%) - 6% of content, distinct scripts
9. **Chinese Simplified** (95%) - 14% of content
10. **Chinese Traditional** (95%) - 6% of content

**English baseline:** 46% of content

## Critical Missing Indicators by Category

### 1. Installation/Setup Verbs (Universal Pattern)
All languages translate "install" and "setup" consistently:

| English | German | Spanish | French | Portuguese | Italian | Indonesian |
|---------|--------|---------|--------|------------|---------|------------|
| Install | installieren | instalar | installer | instalar | installare | menginstal |
| Setup | einrichten | configurar | configurer | configurar | configurare | mengatur |
| Add | hinzufügen | agregar/añadir | ajouter | adicionar | aggiungere | menambahkan |
| Configure | konfigurieren | configurar | configurer | configurar | configurare | mengkonfigurasi |

**Missing from current filter:**
- German: einrichten, hinzufügen, konfigurieren
- French: installer, ajouter, configurer
- Indonesian: mengatur, mengkonfigurasi

### 2. Management Verbs
Common across plugin/marketplace documentation:

| English | German | Spanish | French | Portuguese | Italian | Indonesian |
|---------|--------|---------|--------|------------|---------|------------|
| Manage | verwalten | gestionar | gérer | gerenciar | gestire | mengelola |
| Develop | entwickeln | desarrollar | développer | desenvolver | sviluppare | mengembangkan |
| Create | erstellen | crear | créer | criar | creare | membuat |
| Share | teilen | compartir | partager | compartilhar | condividere | berbagi |
| Organize | organisieren | organizar | organiser | organizar | organizzare | mengorganisir |

**Missing from current filter:**
- German: verwalten, organisieren, teilen
- Spanish: gestionar, desarrollar, crear, compartir, organizar
- French: gérer, développer, créer, partager, organiser
- Portuguese: gerenciar, criar, compartilhar, organizar
- Italian: gestire, sviluppare, creare, condividere, organizzare
- Indonesian: mengelola, berbagi, mengorganisir

### 3. Update/Configuration Verbs

| English | German | Spanish | French | Portuguese | Italian |
|---------|--------|---------|--------|------------|---------|
| Update | aktualisieren | actualizar | mettre à jour | atualizar | aggiornare |
| Enable | aktivieren | habilitar | activer | ativar | abilitare |
| Disable | deaktivieren | deshabilitar | désactiver | desativar | disabilitare |
| Set | einstellen | establecer | définir | definir | impostare |
| Upgrade | upgraden | actualizar | mettre à niveau | atualizar | aggiornare |

**Missing from current filter:**
- German: aktualisieren, aktivieren, deaktivieren, einstellen
- Spanish: habilitar, gestionar
- French: activer, désactiver, définir, mettre
- Portuguese: ativar, desativar, definir
- Italian: abilitare, impostare

### 4. Question Words (Critical for Headings)
These appear frequently in documentation section titles:

| Language | How | What | Why | When | Where |
|----------|-----|------|-----|------|-------|
| German | wie | was | warum | wann | wo |
| Spanish | cómo | qué | por qué | cuándo | dónde |
| French | comment | quoi/que | pourquoi | quand | où |
| Portuguese | como | que/o que | por que | quando | onde |
| Italian | come | cosa/che | perché | quando | dove |

**Currently have:** German "wie", "warum", "wann"
**Missing:** All question words for Spanish, French, Portuguese, Italian

### 5. Common Documentation Terms

| English | German | Spanish | French | Portuguese | Italian | Indonesian |
|---------|--------|---------|--------|------------|---------|------------|
| Guide | Anleitung/Leitfaden | Guía | Guide | Guia | Guida | Panduan |
| Quick Start | Schnellstart | Inicio Rápido | Démarrage Rapide | Início Rápido | Avvio Rapido | Memulai Cepat |
| Overview | Überblick | Visión General | Aperçu | Visão Geral | Panoramica | Ringkasan |
| Step | Schritt | Paso | Étape | Passo | Passaggio | Langkah |

**Missing from current filter:**
- German: leitfaden, schnellstart, überblick, schritt
- Spanish: guía, inicio, rápido, visión, general, paso
- French: démarrage, rapide, aperçu, étape
- Portuguese: início, rápido, visão, geral, passo, guia
- Italian: avvio, rapido, panoramica, passaggio
- Indonesian: panduan, memulai, cepat, ringkasan, langkah

## Morphological Patterns for Detection

### French (-ez Imperative Pattern)
All verbs in installation/setup instructions use -ez endings:
- Installez, Ajoutez, Gérez, Développez, Créez, Organisez, Partagez, Testez
- Configurez, Activez, Désactivez

### German (Capitalized Nouns + -en Infinitives)
- CAPITALIZED: Plugins, Marketplaces, Anleitung, Entwickler, Einstellungen
- Infinitives: installieren, verwalten, entwickeln, erstellen, teilen

### Portuguese (-e Imperative + Nasal Vowels)
- Imperatives: Instale, Adicione, Gerencie, Desenvolva, Configure, Ative
- Nasals: configuração, instalação, atualizações

### Italian (-are/-ere/-ire Infinitives)
- -are: Installare, Configurare, Aggiungere, Creare, Organizzare
- -ere: Gestire, Condividere
- -ire: Abilitare, Disabilitare

### Indonesian (meng- Prefix Pattern)
- meng-: Menginstal, Menambahkan, Mengelola, Mengembangkan, Mengatur
- mem-: Membuat, Memulai
- No articles, no inflections

## Example Headings That Pass Through Current Filter

### German
- "Claude Code einrichten und aktualisieren" (setup and update)
- "Plugins verwalten und organisieren" (manage and organize)
- "Schnellstart-Anleitung für Entwickler" (quick start guide)

### Spanish
- "Guía de inicio rápido" (quick start guide)
- "Gestionar y organizar plugins" (manage and organize)
- "Habilitar actualizaciones automáticas" (enable automatic updates)

### French
- "Installez et gérez les plugins" (install and manage)
- "Démarrage rapide: Guide du développeur" (quick start)
- "Activez les mises à jour automatiques" (enable auto updates)

### Portuguese
- "Instale e gerencie plugins" (install and manage)
- "Guia de início rápido" (quick start guide)
- "Ative atualizações automáticas" (enable auto updates)

### Italian
- "Installare e gestire i plugin" (install and manage)
- "Guida rapida per sviluppatori" (quick guide)
- "Abilitare aggiornamenti automatici" (enable auto updates)

### Indonesian
- "Menginstal dan mengelola plugin" (install and manage)
- "Panduan memulai cepat" (quick start guide)
- "Mengaktifkan pembaruan otomatis" (enable auto updates)

## Recommendations

### Immediate Actions (High Priority)
1. **Add verb forms** for all 6 major languages (German, French, Spanish, Portuguese, Italian, Indonesian)
2. **Add question words** for section headings
3. **Add documentation terminology** (guide, overview, step, quick start)

### Per-Language Indicator Files (Proposed Structure)
```
crates/blz-core/src/language_filter/
├── mod.rs                    # Main filter logic
├── indicators/
│   ├── mod.rs               # Indicator loading
│   ├── german.rs            # German strong/weak indicators
│   ├── french.rs            # French indicators
│   ├── spanish.rs           # Spanish indicators
│   ├── portuguese.rs        # Portuguese indicators
│   ├── italian.rs           # Italian indicators
│   ├── indonesian.rs        # Indonesian indicators
│   └── cjk.rs              # CJK script detection
└── tests/
    └── anthropic_validation.rs  # Test against real anthropic headings
```

### Durable Validation Script
Create `.agents/scripts/validate-language-filter.sh`:
- Searches anthropic source with blz-dev for known multilingual patterns
- Extracts headings from top 50 results
- Tests each heading against language filter
- Reports pass/fail rate by language
- Saves results to `.agents/reports/language-filter-validation-{date}.json`

## Search Queries Used

1. `blz-dev search "npm install -g @anthropic-ai/claude-code" --source anthropic --json`
2. `blz-dev search "GitHub Actions" --source anthropic --json`
3. `blz-dev search "plugins" --source anthropic --json`
4. `blz-dev search "marketplace" --source anthropic --json`
5. `blz-dev search "auto update" --source anthropic --json`
6. `blz-dev search "configuration" --source anthropic --json`

## Files Generated by Subagents

### Subagent 1 (npm install patterns)
- `/tmp/LANGUAGE_REPORT.md`
- `/tmp/DETAILED_INDICATOR_WORDS.md`
- `/tmp/language_report.json`

### Subagent 2 (GitHub Actions)
- `/tmp/FINDINGS_SUMMARY.md`
- `/tmp/action_verbs_matrix.md`
- `/tmp/github_actions_analysis.md`

### Subagent 3 (Plugin/Marketplace)
- `/tmp/README_ANTHROPIC_FINDINGS.md`
- `/tmp/anthropic_findings_comprehensive.md`
- `/tmp/anthropic_verb_patterns.txt`

### Subagent 4 (Update/Configuration)
- `/Users/mg/Developer/outfitter/blz/ANTHROPIC_LANGUAGE_FINDINGS.md`

## Next Steps

1. **Expand indicators** using findings from this audit
2. **Create per-language modules** for better maintainability
3. **Build validation script** that runs against anthropic source
4. **Establish CI check** that fails if non-English content exceeds threshold
5. **Document indicator selection criteria** for future additions
