# Multilingual Plugin & Marketplace Patterns in Anthropic LLMs.txt

## Overview
Analysis of the Anthropic documentation reveals comprehensive multilingual content covering plugin installation, development, management, and marketplace operations across 6+ languages. The documentation is fully translated with consistent structural patterns.

---

## 1. Languages Detected

### Primary Languages Found:
1. **English** (EN) - 881,070 total lines, extensive
2. **French** (FR) - Full translation
3. **German** (DE) - Full translation
4. **Portuguese/Brazilian** (PT) - Full translation
5. **Italian** (IT) - Full translation
6. **Indonesian** (ID) - Full translation

### Language Coverage in Search Results:
- "plugins" query: 50 results, 6 language variants visible
- "marketplace" query: 50 results, 7 language variants visible

---

## 2. Plugin-Related Verbs by Language

### Installation/Addition Verbs

| English | French | German | Portuguese | Italian | Indonesian |
|---------|--------|--------|------------|---------|-----------|
| Install | Installez | installieren | Instale | Installare | Menginstal |
| Add | Ajoutez | hinzufügen | Adicione | Aggiungere | Menambahkan |
| Manage | Gérez | verwalten | Gerencie | Gestire | Mengelola |
| Enable | (enable) | (enable) | - | - | - |
| Verify | - | überprüfen | - | Verificare | Memverifikasi |

**Pattern Notes:**
- French: -ez imperative endings (vous form)
- German: -en infinitive endings
- Portuguese: -e imperative endings
- Italian: -are/-ere/-ire infinitives
- Indonesian: meng- prefix pattern

### Development/Creation Verbs

| English | French | German | Portuguese | Italian | Indonesian |
|---------|--------|--------|------------|---------|-----------|
| Develop | Développez | entwickeln | Desenvolva | - | - |
| Create | (create) | (create) | (create) | Creare | Membuat |
| Organize | Organisez | organisieren | Organize | (Organizzare) | - |
| Share | Partagez | teilen | Compartilhe | - | - |
| Test | Testez | testen | Teste | - | - |
| Build | - | - | - | - | - |

### Marketplace Management Verbs

| English | French | German | Portuguese | Italian | Indonesian |
|---------|--------|--------|------------|---------|-----------|
| Remove | - | - | - | Rimuovere | Menghapus |
| Update | Update | - | - | Aggiornare | Memperbarui |
| List | - | - | - | Elencare | Daftar |
| Host | - | - | - | Ospitare | Hosting |
| Distribute | - | - | - | Distribuire | Mendistribusikan |
| Configure | - | (configure) | - | Configurare | Mengonfigurasi |

---

## 3. Navigation/Menu Terms by Language

### English
```
Plugins
├── Install and manage plugins
│   ├── Install plugins
│   └── Add marketplaces
├── Develop more complex plugins
│   ├── Test your plugins locally
│   ├── Organize complex plugins
│   └── Share your plugins
└── Prochaines étapes / Next steps

Plugin marketplaces
├── Manage marketplace operations
│   ├── Remove a marketplace
│   └── Update marketplace metadata
└── ...
```

### French (Français)
```
Plugins
├── Installez et gérez les plugins
│   ├── Installez les plugins
│   └── Ajoutez des marketplaces
├── Développez des plugins plus complexes
│   ├── Testez vos plugins localement
│   ├── Organisez les plugins complexes
│   └── Partagez vos plugins
└── Prochaines étapes

Marketplaces de plugins (structure implied)
```

### German (Deutsch)
```
Plugins
├── Plugins installieren und verwalten
│   ├── Plugins installieren
│   ├── Installation überprüfen
│   └── Marketplaces hinzufügen
├── Komplexere Plugins entwickeln
│   ├── Ihre Plugins lokal testen
│   ├── Komplexe Plugins organisieren
│   └── Ihre Plugins teilen
└── (Weitere Schritte implied)
```

### Portuguese (Português)
```
Plugins
├── Instale e gerencie plugins
│   ├── Instale plugins
│   └── Adicione marketplaces
├── Desenvolva plugins mais complexos
│   ├── Teste seus plugins localmente
│   ├── Organize plugins complexos
│   └── Compartilhe seus plugins
└── Próximos passos

Mercados de plugins (implied structure)
```

### Italian (Italiano)
```
Marketplace dei plugin
├── Aggiungere e utilizzare marketplace
│   ├── Aggiungere marketplace GitHub
│   ├── Aggiungere marketplace locali per lo sviluppo
│   ├── Installare plugin dai marketplace
│   └── Verificare l'installazione del marketplace
├── Creare il tuo marketplace
│   ├── Creare il file marketplace
│   └── Schema del marketplace
├── Gestire le operazioni del marketplace
│   ├── Aggiornare i metadati del marketplace
│   ├── Elencare marketplace conosciuti
│   └── Rimuovere un marketplace
├── Ospitare e distribuire marketplace
├── Configurare marketplace del team
├── Pemecahan masalah marketplace
└── Prossimi passi
```

### Indonesian (Bahasa Indonesia)
```
Marketplace plugin
├── Menambahkan dan menggunakan marketplace
│   ├── Menambahkan marketplace GitHub
│   ├── Menambahkan marketplace lokal untuk pengembangan
│   ├── Menginstal plugin dari marketplace
│   └── Memverifikasi instalasi marketplace
├── Membuat marketplace Anda sendiri
│   ├── Membuat file marketplace
│   └── Skema marketplace
├── Mengelola operasi marketplace
│   ├── Daftar marketplace yang dikenal
│   ├── Memperbarui metadata marketplace
│   └── Menghapus marketplace
├── Hosting dan mendistribusikan marketplace
├── Mengonfigurasi marketplace tim
├── Pemecahan masalah marketplace
└── Langkah selanjutnya
```

---

## 4. Strong Language Indicators (Morphological)

### French
- **Verb endings:** -ez (Installez, Ajoutez, Gérez, Développez, Testez)
- **Articles:** les, des, du, de la
- **Accents:** é, è, ê, ç, à
- **Keywords:** étendez, configurez, partagez, prochaines
- **Example:** "Installez et gérez les plugins avec des commandes simples"

### German
- **Capitalization:** All nouns capitalized (Plugins, Marketplaces, Verwaltung)
- **Verb infinitives:** -en (installieren, entwickeln, testen, teilen, organisieren)
- **Umlauts:** ü, ö, ä
- **Compounds:** Plugin-Marketplaces, Marketplace-Verwaltung
- **Example:** "Plugins installieren und verwalten"

### Portuguese
- **Imperative endings:** -e (Instale, Adicione, Gerencie, Compartilhe, Teste, Desenvolva)
- **Nasals:** ã, õ (não, próximos)
- **Accents:** á, é, í, ó, ú
- **Keywords:** gerencie, desenvolva, compartilhe, próximos, equipes
- **Example:** "Instale e gerencie plugins com comandos simples"

### Italian
- **Infinitive endings:** -are, -ere, -ire (installare, aggiungere, gestire, creare, rimuovere, ospitare)
- **Plural articles:** del, dei, della, delle
- **Plurals:** -i, -e (i metadati, dei plugin, dei marketplace)
- **Keywords:** creare, verificare, rimuovere, gestire, ospitare, distribuire
- **Example:** "Installare e gestire i plugin con comandi semplici"

### Indonesian
- **Meng- prefix (infinitive marker):** Menginstal, Menambahkan, Mengelola, Memperbarui, Menghapus, Memverifikasi
- **-kan suffix:** Forms from root verbs (buat -> membuat, hapus -> menghapus)
- **No inflections:** Same word form regardless of subject/tense
- **Keywords:** buat, kelola, daftar, gunakan, buat, bagikan
- **Example:** "Menginstal dan mengelola plugin dengan perintah sederhana"

---

## 5. Core Plugin Operations (Consistent Across All Languages)

### Installation Path
1. Discover plugins (find, search)
2. Add marketplace (add, import source)
3. Install plugin (enable, activate)
4. Verify installation (check, test)

### Development Path
1. Create plugin file/structure
2. Organize complex plugins
3. Test locally
4. Share with community
5. Distribute via marketplace

### Marketplace Management
1. Create marketplace.json
2. Add plugins to marketplace
3. Host on GitHub or local
4. Update metadata
5. Configure for team
6. Troubleshoot issues

---

## 6. Example Phrases with Strong Language Indicators

### French Examples
- "Étendez Claude Code avec des commandements personnalisés" (Extend with custom)
- "Installez et gérez les plugins via l'interface" (Install and manage via interface)
- "Téléchargez depuis une marketplace" (Download from a marketplace)

### German Examples
- "Erweitern Sie Claude Code mit benutzerdefinierten Befehlen" (Extend with custom commands)
- "Plugins installieren und verwalten" (Install and manage plugins)
- "Installation überprüfen" (Verify installation)

### Portuguese Examples
- "Estenda o Claude Code com comandos personalizados" (Extend with custom commands)
- "Instale e gerencie plugins através da interface" (Install and manage via interface)
- "Compartilhe seus plugins com a comunidade" (Share your plugins with community)

### Italian Examples
- "Estendi Claude Code con comandi personalizzati" (Extend with custom commands)
- "Installare e gestire i plugin tramite l'interfaccia" (Install and manage via interface)
- "Ospitare su GitHub (consigliato)" (Host on GitHub (recommended))

### Indonesian Examples
- "Perluas Claude Code dengan perintah khusus" (Extend with custom commands)
- "Menginstal dan mengelola plugin melalui antarmuka" (Install and manage via interface)
- "Bagikan plugin Anda dengan komunitas" (Share plugins with community)

---

## 7. Technical Terminology (Consistent Across Languages)

| Concept | Appears Uniformly | Notes |
|---------|------------------|-------|
| marketplace | Yes | Single term across all languages |
| plugin/plugin(s) | Yes | Plugin or pluralized form |
| metadata | Yes | Same term used universally |
| GitHub | Yes | Brand name unchanged |
| JSON | Yes | File format universal |
| team | Mostly | "équipe" (FR), "team" (EN/DE), "equipo"/"tim" (PT), "team" (IT), "tim" (ID) |
| local | Yes | "local" in all languages |
| install | Yes | Varying verb forms |
| manage | Yes | Varying verb forms |

---

## 8. Structure Patterns

### Document Structure Consistency
- All language versions follow identical hierarchy
- Same section ordering across all translations
- Parallel examples in code blocks
- Identical tables and metadata definitions

### Navigation Structure
- Section headers translate directly
- Breadcrumb trails maintain parallel structure
- "Next steps" / "Prochaines étapes" / "Próximos passos" etc.
- User/developer segmentation appears in all languages

---

## 9. Search Statistics

**Plugin Search:**
- Total results: 50 heading matches
- Languages represented: 6
- Line numbers: 51,000-638,000+ (very distributed)

**Marketplace Search:**
- Total results: 50 heading matches
- Languages represented: 7
- Most concentrated: Italian section (lines 417,516+)

---

## 10. Practical Extraction Recommendations

### For Language Detection:
1. **Look for verb form patterns first** (morphologically most distinctive)
2. **Check for diacritics** (é/ç for French, ü for German, ã/õ for Portuguese, ì for Italian)
3. **Identify articles and determiners** (le/les for French, der/die/das for German, il/la/i for Italian)
4. **Check capitalization of nouns** (strong German indicator)
5. **Look for meng- prefix** (Indonesian specific)

### For Plugin/Marketplace Terms:
1. **Search for "plugin" variants:** plugins, plugin, plugin(s), plugin/i
2. **Search for "marketplace" variants:** marketplace(s), "mercado de plugins", marketplace dei plugin
3. **Search for action verbs:** install*, add*, create*, develop*, manage*, share*
4. **Search for "team" variants:** team, équipe, tim, equipo
5. **Look for navigation patterns:** "next steps", "prochaines étapes", "próximos passos", "prossimi passi"

### Heading Path Construction:
Use the `headingPath` array from search results which maintains the full navigation hierarchy:
```
headingPath: ["Plugins", "Instale e gerencie plugins", "Instale plugins"]
Language: Portuguese (from verb endings)
Section: Installation operations
```

