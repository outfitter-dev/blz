# GitHub Actions - Action Verbs Translation Matrix

## Core Configuration Verbs

| Action | English | Mandarin | Traditional Chinese | Japanese | Russian |
|--------|---------|----------|-------------------|----------|---------|
| **Setup** | setup | 设置 | 設定 | セットアップ | Настройка |
| **Install** | install | 安装 | 安裝 | インストール | установить |
| **Configure** | configure | 配置 | 配置 | 設定 | конфигурация |
| **Add/Create** | add | 添加 | 新增 | 追加 | добавить |
| **Execute** | run/execute | 执行 | 執行 | 実行 | выполнять |
| **Copy** | copy | 复制 | 複製 | コピー | скопировать |
| **Open** | open | 打开 | 開啟 | 開く | откройте |

| Action | German | Spanish | French | Portuguese | Indonesian | Korean | Italian |
|--------|--------|---------|--------|------------|-----------|--------|---------|
| **Setup** | Einrichtung | configuración | configuration | configuração | pengaturan | 설정 | configurazione |
| **Install** | installieren | instalar | installer | instalar | menginstal | 설치 | installare |
| **Configure** | konfigurieren | configurar | configurer | configurar | mengkonfigurasi | 구성 | configurare |
| **Add/Create** | hinzufügen | agregar | ajouter | adicionar | menambah | 추가 | aggiungere |
| **Execute** | ausführen | ejecutar | exécuter | executar | menjalankan | 실행 | eseguire |
| **Copy** | kopieren | copiar | copier | copiar | menyalin | 복사 | copiare |
| **Open** | öffnen | abrir | ouvrir | abrir | membuka | 열기 | aprire |

---

## Code Manipulation Verbs

| Action | English | Mandarin | Traditional Chinese | Japanese | Russian | German |
|--------|---------|----------|-------------------|----------|---------|--------|
| **Analyze** | analyze | 分析 | 分析 | 分析 | анализировать | analysieren |
| **Create** | create | 创建 | 建立 | 作成 | создавать | erstellen |
| **Implement** | implement | 实现 | 實作 | 実装 | реализовывать | implementieren |
| **Fix** | fix | 修复 | 修正 | 修正 | исправлять | beheben |
| **Follow** | follow | 遵循 | 遵循 | 従う | следовать | folgen |

| Action | Spanish | French | Portuguese | Indonesian | Korean | Italian |
|--------|---------|--------|------------|-----------|--------|---------|
| **Analyze** | analizar | analyser | analisar | menganalisis | 분석 | analizzare |
| **Create** | crear | créer | criar | membuat | 생성 | creare |
| **Implement** | implementar | implémenter | implementar | mengimplementasikan | 구현 | implementare |
| **Fix** | corregir | corriger | corrigir | memperbaiki | 수정 | correggere |
| **Follow** | seguir | suivre | seguir | mengikuti | 따르 | seguire |

---

## Configuration Nouns

| Term | English | Mandarin | Traditional Chinese | Japanese | Russian |
|------|---------|----------|-------------------|----------|---------|
| **Repository** | repository | 仓库 | 儲存庫 | リポジトリ | репозиторий |
| **Secret** | secret | 密钥 | 密鑰 | シークレット | секреты |
| **API Key** | API key | API 密钥 | API 密鑰 | API キー | API ключ |
| **Permissions** | permissions | 权限 | 權限 | 権限 | разрешения |
| **Workflow** | workflow | 工作流程 | 工作流程 | ワークフロー | рабочий процесс |

| Term | German | Spanish | French | Portuguese | Indonesian | Korean | Italian |
|------|--------|---------|--------|------------|-----------|--------|---------|
| **Repository** | Repository | repositorio | dépôt | repositório | repositori | 저장소 | repository |
| **Secret** | Secrets | secretos | secrets | segredos | secret | 시크릿 | segreti |
| **API Key** | API-Schlüssel | clave API | clé API | chave de API | kunci API | API 키 | chiave API |
| **Permissions** | Genehmigungen | permisos | permissions | permissões | izin | 권한 | permessi |
| **Workflow** | Workflow-Datei | archivo de flujo | fichier de flux | arquivo de fluxo | file workflow | 워크플로우 | file del flusso |

---

## Frequently Paired Phrases

### Setup Pathways
| English | Mandarin | Japanese | Russian | German | Spanish | French |
|---------|----------|----------|---------|--------|---------|--------|
| Quick setup | 快速设置 | クイックセットアップ | Быстрая настройка | Schnelle Einrichtung | Configuración rápida | Configuration rapide |
| Manual setup | 手动设置 | 手動セットアップ | Ручная настройка | Manuelle Einrichtung | Configuración manual | Configuration manuelle |

### Permission Pairs
| English | Mandarin | Japanese | Russian | German | Spanish |
|---------|----------|----------|---------|--------|---------|
| Read/Write | 读写 | 読み書き | чтение/запись | Lesen/Schreiben | lectura/escritura |

### Common Phrases in Context
| Concept | English | Mandarin | Traditional | Japanese |
|---------|---------|----------|-------------|----------|
| Admin requirement | Must be admin | 必须是管理员 | 必須是管理員 | 管理者である必要 |
| Repository access | Repository permissions | 仓库权限 | 儲存庫權限 | リポジトリ権限 |
| Secrets management | Add to secrets | 添加到密钥 | 新增到密鑰 | シークレットに追加 |

---

## Language Group Patterns

### Tier 1: Universal Concepts (All 11 Languages)
These actions appear in every language variant:
- Setup / Configure
- Install
- Execute / Run
- Add / Create
- Copy
- Open

### Tier 2: Common Concepts (90%+ Languages)
- Analyze
- Implement
- Fix
- Follow/Adhere
- Permissions
- Repository
- Workflow

### Tier 3: Secondary Concepts (70%+ Languages)
- Guide / Assist
- Modify / Update
- Access
- Deploy

### Language-Specific Extensions
- **Japanese**: Heavy use of katakana loanwords (アプリ, ワークフロー, キー)
- **Russian**: Aspect distinction in verbs (установить vs устанавливать)
- **Chinese**: Compound morphemes (安装 = 安 + 装)
- **Indonesian**: Affixation patterns (me- prefix for actions)
- **Korean**: Particle markers (에 for location, 을 for objects)

---

## Cross-Language Consistency Notes

1. **Commands remain identical**: `/install-github-app` is used in all 11 languages
2. **Entity names stay English**: GitHub, API, PR, issue often appear as-is
3. **Structural parallels**: Quick Setup → Manual Setup pathway mirrored exactly
4. **Permission sets**: Contents/Issues/Pull Requests structure preserved everywhere
5. **Compound formations**: Languages create compounds from primitives (German: GitHub-App, Chinese: GitHub 应用程序)

