# Anthropic Documentation: Update & Configuration Patterns Across Languages

## Search Summary
- **Source**: Anthropic llms-full.txt (881,070 lines, 24,640 headings)
- **Search Queries**: "auto update", "automatic", "configuration", "settings", "enable/disable", question words (wie/warum, cómo/qué, comment/pourquoi, come/perché, como/por que), "environment variable", "configure/set", "upgrade"
- **Total Results Analyzed**: 100+ results across 8 language variants

---

## 1. UPDATE-RELATED TERMINOLOGY BY LANGUAGE

### English (Base)
- **Auto Updates**: Automatically keeps itself up to date
- **Update checks**: Performed on startup and periodically while running
- **Update process**: Downloads and installs automatically in the background
- **Notifications**: You'll see a notification when updates are installed
- **Applying updates**: Updates take effect the next time you start Claude Code
- **Manual update**: `claude update` command
- **Upgrade**: Moving to latest versions (e.g., v1.0 from beta)
- **Related terms**: Enable/Disable, Install, Uninstall

### Indonesian
- **Auto updates**: Auto-updates (secara otomatis menjaga dirinya tetap terbaru)
- **Pemeriksaan update**: Dilakukan saat startup dan secara berkala saat berjalan
- **Proses update**: Mengunduh dan menginstal secara otomatis di latar belakang
- **Notifikasi**: Anda akan melihat notifikasi ketika update diinstal
- **Menerapkan update**: Update berlaku saat Anda memulai Claude Code berikutnya
- **Upgrade dari Beta**: Beta upgrade terminology
- **Langkah upgrade**: Upgrade steps

### Portuguese
- **Upgrade ke versi alat terbaru**: Upgrade to latest tool version
- **Langkah upgrade**: Upgrade steps

### German (Deutsch)
- **Upgrade-Schritte**: Upgrade steps
- **Den Beta-Header aktualisieren**: Update the beta header
- **Upgrade auf die neueste Tool-Version**: Upgrade to latest tool version

### Italian
- **Auto-Installazione**: Auto-installation (plugin auto-installation)
- **Il plugin può anche essere auto-installato**: Plugin can also be auto-installed

### French
- **Mise à jour**: Update (standard term)

### Spanish
- **Auto-Installazione**: Auto-installation
- **Upgrade desde Beta**: Upgrade from Beta

---

## 2. CONFIGURATION VERBS & NOUNS

### Core Configuration Verbs (Present Across All Languages)
| Verb | English | German | Spanish | French | Italian | Portuguese |
|------|---------|--------|---------|--------|---------|------------|
| Enable | Enable | Aktivieren | Habilitar | Activer | Abilitare | Ativar |
| Disable | Disable | Deaktivieren | Deshabilitar | Désactiver | Disabilitare | Desativar |
| Configure | Configure | Konfigurieren | Configurar | Configurer | Configurare | Configurar |
| Set | Set | Einstellen | Establecer | Définir | Impostare | Definir |
| Install | Install | Installieren | Instalar | Installer | Installare | Instalar |
| Uninstall | Uninstall | Deinstallieren | Desinstalar | Désinstaller | Disinstallare | Desinstalar |
| Update | Update | Aktualisieren | Actualizar | Mettre à jour | Aggiornare | Atualizar |
| Upgrade | Upgrade | Aktualisierung | Actualización | Mise à niveau | Aggiornamento | Atualização |

### Configuration Nouns
- **Settings**: settings.json, settings files
- **Configuration**: configuration files, model configuration, proxy configuration
- **Environment variables**: Variable environment, environment variable expansion
- **Managed policies**: managed-settings.json
- **Plugin settings**: enabledPlugins, extraKnownMarketplaces
- **User settings**: ~/.claude/settings.json
- **Project settings**: .claude/settings.json
- **Local settings**: .claude/settings.local.json
- **Preferences**: Plugin preferences, personal preferences
- **Scopes**: User scope, project scope, local scope

### Plugin-Specific Commands (Multilingual)
```
/plugin install formatter@your-org
/plugin enable plugin-name@marketplace-name
/plugin disable plugin-name@marketplace-name
/plugin uninstall plugin-name@marketplace-name
```

### Environment Configuration Verbs
- **Set**: `export DISABLE_AUTOUPDATER=1`
- **Configure**: Bedrock integration, proxy settings, GCP credentials
- **Enable**: `CLAUDE_CODE_USE_BEDROCK=1`
- **Override**: Model name overrides

---

## 3. QUESTION WORD PATTERNS BY LANGUAGE

### German (Deutsch)
**Question words found**: "Wie", "Warum"
- **Warum Sandboxing wichtig ist**: Why sandboxing is important
- **Warum Claude nicht denken lassen?**: Why not let Claude think?
- **Warum Prompts verketten?**: Why chain prompts?
- **Warum die Umbenennung?**: Why the renaming?
- **Warum Beispiele verwenden?**: Why use examples?

### Spanish (Español)
**Question words found**: "Cómo", "Qué"
- **¿Qué puede hacer Claude?**: What can Claude do?
- **Cómo funciona**: How it works
- **Cómo ser claro, contextual y específico**: How to be clear, contextual, and specific
- **Cómo implementar el uso de herramientas**: How to implement tool usage

### French (Français)
**Question words found**: "Comment", "Pourquoi"
- **Pourquoi enchaîner les prompts?**: Why chain prompts?
- **Pourquoi le renommage?**: Why the renaming?
- **Pourquoi utiliser Claude Code GitHub Actions?**: Why use Claude Code GitHub Actions?
- **Pourquoi utiliser des exemples?**: Why use examples?

### Italian (Italiano)
**Question words found**: "Come", "Perché"
- **Come funziona**: How it works
- **Perché non lasciare pensare Claude?**: Why not let Claude think?
- **Perché il sandboxing è importante**: Why sandboxing is important
- **Perché la Ridenominazione?**: Why the renaming?
- **Perché concatenare i prompt?**: Why chain prompts?

### Portuguese (Português)
**Question words found**: "Como", "Por que"
- **O que o Claude Code faz por você**: What Claude Code does for you
- **Por que o isolamento de segurança é importante**: Why security isolation is important
- **Por que não deixar o Claude pensar?**: Why not let Claude think?
- **Por que usar Claude Code GitHub Actions?**: Why use Claude Code GitHub Actions?

---

## 4. SETTINGS & ENVIRONMENT VARIABLE TERMINOLOGY

### Key Environment Variables (Multilingual)
| Variable | Purpose | Languages Found |
|----------|---------|-----------------|
| `DISABLE_AUTOUPDATER` | Disable automatic updates | EN, ID, DE |
| `ANTHROPIC_API_KEY` | Authentication | EN, DE |
| `CLAUDE_CODE_USE_BEDROCK` | Enable Bedrock integration | EN, DE, ID |
| `AWS_REGION` | AWS region configuration | EN |
| `ANTHROPIC_MODEL` | Model selection | EN |
| `CLAUDE_CODE_REMOTE` | Remote environment detection | EN |
| `CLAUDE_ENV_FILE` | Persistent environment variables | EN |

### Settings File Hierarchy (All Languages)
1. **Enterprise managed policies** (`managed-settings.json`) - Highest priority
2. **Command line arguments** - Temporary overrides
3. **User settings** (`~/.claude/settings.json`) - Personal configuration
4. **Project settings** (`.claude/settings.json`) - Team configuration
5. **Local settings** (`.claude/settings.local.json`) - Machine-specific

### Precedence Concepts (Multilingual)
- **Settings precedence**: Order in which settings are applied
- **Priority levels**: Highest to lowest
- **Overrides**: Can override default behavior
- **Scopes**: User, project, local, enterprise

---

## 5. DISABLE/ENABLE PATTERNS

### Activation Commands (All Languages)
```bash
/sandbox                                    # Enable sandboxing
/plugin enable plugin-name@marketplace-name # Enable specific plugin
/plugin disable plugin-name@marketplace-name # Disable specific plugin
export DISABLE_AUTOUPDATER=1               # Disable auto-updates (env var)
```

### Settings-Based Enable/Disable
```json
{
  "enabledPlugins": {
    "formatter@company-tools": true,
    "deployer@company-tools": true,
    "analyzer@security-plugins": false
  }
}
```

### Command Line Disable Patterns
- Slash commands: `/plugin disable <name>`
- Environment variables: `DISABLE_*` format
- Settings JSON: Boolean true/false values

---

## 6. SETUP & INSTALLATION PATTERNS

### Multi-Language Setup Terms
| English | German | Spanish | French | Italian | Portuguese |
|---------|--------|---------|--------|---------|------------|
| Setup | Einrichtung | Configuración | Configuration | Configurazione | Configuração |
| Install | Installieren | Instalar | Installer | Installare | Instalar |
| Configure | Konfigurieren | Configurar | Configurer | Configurare | Configurar |
| Enable | Aktivieren | Habilitar | Activer | Abilitare | Ativar |
| Auto-install | Auto-Installation | Auto-instalación | Auto-installation | Auto-installazione | Auto-instalação |

### GCP Setup Example (All Languages)
```bash
gcloud config set project YOUR-PROJECT-ID
gcloud services enable aiplatform.googleapis.com
```

### Bedrock Configuration
- **Set environment variables**: Multiple language support
- **Enable integration**: `CLAUDE_CODE_USE_BEDROCK=1`
- **Configure credentials**: AWS authentication

---

## 7. CONFIGURATION SYSTEM OVERVIEW

### Three-Tier Configuration Architecture
1. **Memory files (CLAUDE.md)**: Instructions and context loaded at startup
2. **Settings files (JSON)**: Permissions, environment variables, tool behavior
3. **Slash commands**: Custom commands invoked during session

### JSON Configuration Files
- `settings.json` - Main settings file
- `managed-settings.json` - Enterprise-managed policies
- `.mcp.json` - MCP server configuration with env var expansion
- `.claude/settings.json` - Project-level settings
- `~/.claude/settings.json` - User-level settings

### Configuration Features (Multilingual Support)
- **Environment variable expansion**: `${VAR}` syntax support
- **Hierarchical application**: Precedence rules enforced
- **Team-based configuration**: Repository-level settings
- **Machine-specific overrides**: Local settings for personalization

---

## 8. LANGUAGE DISTRIBUTION IN CONTENT

### Languages Detected
1. **English** (EN) - Primary/base documentation
2. **German** (DE) - Full coverage
3. **Spanish** (ES) - Full coverage
4. **French** (FR) - Full coverage
5. **Italian** (IT) - Full coverage
6. **Portuguese** (PT) - Full coverage
7. **Indonesian** (ID) - Partial coverage
8. **Traditional Chinese** (ZH-TW) - Partial coverage
9. **Korean** (KO) - Partial coverage

### Content Structure Across Languages
- Parallel sections with consistent structure
- Identical code examples across languages
- Translated headings and explanations
- Language-specific paths in headings (e.g., `/en/`, `/de/`, `/es/`)

---

## 9. KEY FINDINGS SUMMARY

### Update Terminology
- **Consistent across languages**: Auto update, update checks, update process
- **Manual vs. automatic**: Explicit commands vs. background processes
- **Disable mechanism**: Environment variable `DISABLE_AUTOUPDATER`
- **Notifications**: User-facing feedback on update status

### Configuration Philosophy
- **Hierarchical settings**: Enterprise > CLI > User > Project > Local
- **Environment variables**: Primary override mechanism for automation
- **JSON settings**: Persistent configuration storage
- **Flexibility**: Multiple ways to configure same functionality

### Multi-Language Support
- All major configuration terms have direct translations
- Question-and-answer patterns consistent across languages
- Plugin management commands identical across all languages
- Error messages and guides localized

### Best Practices Identified
1. Settings files should be treated as team assets (in git)
2. Local settings for machine-specific secrets
3. Environment variables for automation and CI/CD
4. Plugin system uses consistent naming (plugin-name@marketplace-name)
5. Enterprise policies cannot be overridden by users

---

## 10. EXAMPLE USE CASES

### Disabling Auto-Updates
**English**: `export DISABLE_AUTOUPDATER=1`
**German**: `export DISABLE_AUTOUPDATER=1` (same command, translated explanation)
**Spanish**: Same command across all languages
**French**: Same command, translated context

### Configuring Plugins
**All languages**:
```
/plugin enable formatter@company-tools
/plugin disable analyzer@security-plugins
```

### Setting Model
**Configuration options** (all languages):
1. During session: `/model <alias|name>`
2. At startup: `claude --model <alias|name>`
3. Environment: `ANTHROPIC_MODEL=<name>`
4. Settings: `settings.json` file

### Team Plugin Setup
**In `.claude/settings.json`** (all languages):
```json
{
  "extraKnownMarketplaces": {
    "team-tools": {
      "source": {
        "source": "github",
        "repo": "your-org/team-tools"
      }
    }
  }
}
```

---

## Appendix: Full Text Examples

### Auto Updates (English)
```
### Auto updates

Claude Code automatically keeps itself up to date to ensure you have the latest features and security fixes.

* **Update checks**: Performed on startup and periodically while running
* **Update process**: Downloads and installs automatically in the background
* **Notifications**: You'll see a notification when updates are installed
* **Applying updates**: Updates take effect the next time you start Claude Code

**Disable auto-updates:**

Set the `DISABLE_AUTOUPDATER` environment variable in your shell or settings.json file:

export DISABLE_AUTOUPDATER=1
```

### Auto Updates (Indonesian)
```
### Auto updates

Claude Code secara otomatis menjaga dirinya tetap terbaru untuk memastikan Anda memiliki fitur terbaru dan perbaikan keamanan.

* **Pemeriksaan update**: Dilakukan saat startup dan secara berkala saat berjalan
* **Proses update**: Mengunduh dan menginstal secara otomatis di latar belakang
* **Notifikasi**: Anda akan melihat notifikasi ketika update diinstal
* **Menerapkan update**: Update berlaku saat Anda memulai Claude Code berikutnya

**Nonaktifkan auto-updates:**

Atur variabel lingkungan `DISABLE_AUTOUPDATER` di shell Anda atau file settings.json:

export DISABLE_AUTOUPDATER=1
```

