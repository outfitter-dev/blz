# Verbatim GitHub Actions Documentation Excerpts
## By Language - Setup Sections

---

## ENGLISH
Source: anthropic:117445-117445

```
### Claude Code Action

This GitHub Action allows you to run Claude Code within your GitHub Actions workflows. 
You can use this to build any custom workflow on top of Claude Code.

[View repository →](https://github.com/anthropics/claude-code-action)

## Setup

## Quick setup

The easiest way to set up this action is through Claude Code in the terminal. Just open 
claude and run `/install-github-app`.

This command will guide you through setting up the GitHub app and required secrets.

## Manual setup

If the `/install-github-app` command fails or you prefer manual setup, please follow 
these manual setup instructions:

1. **Install the Claude GitHub app** to your repository: 
   [https://github.com/apps/claude](https://github.com/apps/claude)
2. **Add ANTHROPIC_API_KEY** to your repository secrets
3. **Copy the workflow file** from examples/claude.yml to your repository's .github/workflows/
```

---

## MANDARIN CHINESE (Simplified)
Source: anthropic:779354

```
### Claude Code Action

这个 GitHub Action 允许您在 GitHub Actions 工作流程中运行 Claude Code。您可以使用它在 Claude Code 之上构建任何自定义工作流程。

[查看仓库 →](https://github.com/anthropics/claude-code-action)

## 设置

## 快速设置

设置此 action 的最简单方法是通过终端中的 Claude Code。只需打开 claude 并运行 `/install-github-app`。

此命令将指导您设置 GitHub 应用程序和所需的密钥。

## 手动设置

如果 `/install-github-app` 命令失败或您更喜欢手动设置，请按照以下手动设置说明操作：

1. **将 Claude GitHub 应用程序安装**到您的仓库：[https://github.com/apps/claude](https://github.com/apps/claude)

   Claude GitHub 应用程序需要以下仓库权限：

   * **内容**：读写（修改仓库文件）
   * **Issues**：读写（响应 issues）
   * **拉取请求**：读写（创建 PR 和推送更改）

2. **将 ANTHROPIC_API_KEY 添加**到您的仓库密钥
3. **复制工作流程文件**从 examples/claude.yml 到您仓库的 `.github/workflows/`
```

---

## TRADITIONAL CHINESE (Taiwan)
Source: anthropic:852768

```
### Claude Code Action

這個 GitHub Action 允許您在 GitHub Actions 工作流程中執行 Claude Code。您可以使用它在 Claude Code 之上建立任何自訂工作流程。

[查看儲存庫 →](https://github.com/anthropics/claude-code-action)

## 設定

## 快速設定

設定此 action 最簡單的方式是透過終端機中的 Claude Code。只需開啟 claude 並執行 `/install-github-app`。

此指令將引導您設定 GitHub 應用程式和必要的密鑰。

## 手動設定

如果 `/install-github-app` 指令失敗或您偏好手動設定，請遵循這些手動設定說明：

1. **安裝 Claude GitHub 應用程式**到您的儲存庫：[https://github.com/apps/claude](https://github.com/apps/claude)

   Claude GitHub 應用程式需要以下儲存庫權限：

   * **內容**：讀寫（修改儲存庫檔案）
   * **Issues**：讀寫（回應 issues）
   * **拉取請求**：讀寫（建立 PR 和推送變更）

2. **新增 ANTHROPIC_API_KEY** 到您的儲存庫密鑰
3. **複製工作流程檔案**從 examples/claude.yml 到您儲存庫的 `.github/workflows/`
```

---

## JAPANESE
Source: anthropic:485409

```
### Claude Code Action

このGitHub ActionはGitHub Actionsワークフロー内でClaude Codeを実行することを可能にします。これを使用してClaude Codeの上にカスタムワークフローを構築できます。

[リポジトリを見る →](https://github.com/anthropics/claude-code-action)

## セットアップ

## クイックセットアップ

このアクションをセットアップする最も簡単な方法は、ターミナルでClaude Codeを使用することです。claudeを開いて`/install-github-app`を実行するだけです。

このコマンドはGitHubアプリと必要なシークレットのセットアップをガイドします。

## 手動セットアップ

`/install-github-app`コマンドが失敗した場合や手動セットアップを希望する場合は、以下の手動セットアップ手順に従ってください：

1. **Claude GitHubアプリをインストール**してリポジトリに追加: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   Claude GitHubアプリは以下のリポジトリ権限を必要とします：

   * **Contents**: 読み書き（リポジトリファイルを変更するため）
   * **Issues**: 読み書き（issueに応答するため）
   * **Pull requests**: 読み書き（PRを作成し変更をプッシュするため）

2. **ANTHROPIC_API_KEY**をリポジトリシークレットに追加
3. **ワークフローファイルをコピー**examples/claude.ymlからリポジトリの`.github/workflows/`に
```

---

## RUSSIAN
Source: anthropic:705795

```
### Claude Code Action

Этот GitHub Action позволяет вам запускать Claude Code в ваших рабочих процессах GitHub Actions. Вы можете использовать это для создания любого пользовательского рабочего процесса поверх Claude Code.

[Посмотреть репозиторий →](https://github.com/anthropics/claude-code-action)

## Настройка

## Быстрая настройка

Самый простой способ настроить этот action - через Claude Code в терминале. Просто откройте claude и выполните `/install-github-app`.

Эта команда проведет вас через настройку GitHub приложения и необходимых секретов.

## Ручная настройка

Если команда `/install-github-app` не работает или вы предпочитаете ручную настройку, пожалуйста, следуйте этим инструкциям по ручной настройке:

1. **Установите Claude GitHub приложение** в ваш репозиторий: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   Claude GitHub приложение требует следующие разрешения репозитория:

   * **Contents**: Чтение и запись (для изменения файлов репозитория)
   * **Issues**: Чтение и запись (для ответа на issues)
   * **Pull requests**: Чтение и запись (для создания PR и отправки изменений)

2. **Добавьте ANTHROPIC_API_KEY** в секреты репозитория
3. **Скопируйте файл рабочего процесса** из examples/claude.yml в `.github/workflows/` вашего репозитория
```

---

## GERMAN
Source: anthropic:44902

```
### Claude Code Action

Diese GitHub Action ermöglicht es Ihnen, Claude Code innerhalb Ihrer GitHub Actions-Workflows auszuführen. Sie können dies verwenden, um jeden benutzerdefinierten Workflow auf Basis von Claude Code zu erstellen.

[Repository anzeigen →](https://github.com/anthropics/claude-code-action)

## Einrichtung

## Schnelle Einrichtung

Der einfachste Weg, diese Action einzurichten, ist über Claude Code im Terminal. Öffnen Sie einfach claude und führen Sie `/install-github-app` aus.

Dieser Befehl führt Sie durch die Einrichtung der GitHub-App und der erforderlichen Secrets.

## Manuelle Einrichtung

Wenn der `/install-github-app`-Befehl fehlschlägt oder Sie die manuelle Einrichtung bevorzugen, folgen Sie bitte diesen manuellen Einrichtungsanweisungen:

1. **Installieren Sie die Claude GitHub-App** in Ihrem Repository: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   Die Claude GitHub-App benötigt die folgenden Repository-Genehmigungen:

   * **Contents**: Lesen und Schreiben (zum Ändern von Repository-Dateien)
   * **Issues**: Lesen und Schreiben (zum Beantworten von Issues)
   * **Pull Requests**: Lesen und Schreiben (zum Erstellen von PRs und Übertragen von Änderungen)

2. **Fügen Sie ANTHROPIC_API_KEY** zu Ihren Repository-Secrets hinzu
3. **Kopieren Sie die Workflow-Datei** von examples/claude.yml in das `.github/workflows/`-Verzeichnis Ihres Repositories
```

---

## SPANISH
Source: anthropic:191350

```
### Claude Code Action

Esta GitHub Action te permite ejecutar Claude Code dentro de tus flujos de trabajo de GitHub Actions. Puedes usar esto para construir cualquier flujo de trabajo personalizado sobre Claude Code.

[Ver repositorio →](https://github.com/anthropics/claude-code-action)

## Configuración

## Configuración rápida

La forma más fácil de configurar esta action es a través de Claude Code en la terminal. Solo abre claude y ejecuta `/install-github-app`.

Este comando te guiará a través de la configuración de la aplicación de GitHub y los secretos requeridos.

## Configuración manual

Si el comando `/install-github-app` falla o prefieres la configuración manual, por favor sigue estas instrucciones de configuración manual:

1. **Instala la aplicación Claude GitHub** en tu repositorio: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   La aplicación Claude GitHub requiere los siguientes permisos de repositorio:

   * **Contents**: Lectura y escritura (para modificar archivos del repositorio)
   * **Issues**: Lectura y escritura (para responder a issues)
   * **Pull requests**: Lectura y escritura (para crear PR y enviar cambios)

2. **Agrega ANTHROPIC_API_KEY** a tus secretos del repositorio
3. **Copia el archivo de flujo de trabajo** desde examples/claude.yml al directorio `.github/workflows/` de tu repositorio
```

---

## FRENCH
Source: anthropic:264872

```
### Claude Code Action

Cette GitHub Action vous permet d'exécuter Claude Code dans vos flux de travail GitHub Actions. Vous pouvez l'utiliser pour construire n'importe quel flux de travail personnalisé sur Claude Code.

[Voir le dépôt →](https://github.com/anthropics/claude-code-action)

## Configuration

## Configuration rapide

La façon la plus simple de configurer cette action est via Claude Code dans le terminal. Ouvrez simplement claude et exécutez `/install-github-app`.

Cette commande vous guidera dans la configuration de l'application GitHub et des secrets requis.

## Configuration manuelle

Si la commande `/install-github-app` échoue ou si vous préférez une configuration manuelle, veuillez suivre ces instructions de configuration manuelle :

1. **Installez l'application GitHub Claude** sur votre dépôt : 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   L'application GitHub Claude nécessite les permissions de dépôt suivantes :

   * **Contents** : Lecture et écriture (pour modifier les fichiers du dépôt)
   * **Issues** : Lecture et écriture (pour répondre aux issues)
   * **Pull requests** : Lecture et écriture (pour créer des PR et pousser des changements)

2. **Ajoutez ANTHROPIC_API_KEY** à vos secrets du dépôt
3. **Copiez le fichier de flux de travail** de examples/claude.yml vers le répertoire `.github/workflows/` de votre dépôt
```

---

## INDONESIAN
Source: anthropic:338336

```
### Claude Code Action

GitHub Action ini memungkinkan Anda menjalankan Claude Code dalam alur kerja GitHub Actions Anda. Anda dapat menggunakan ini untuk membangun alur kerja kustom apa pun di atas Claude Code.

[Lihat repositori →](https://github.com/anthropics/claude-code-action)

## Pengaturan

## Pengaturan cepat

Cara termudah untuk mengatur action ini adalah melalui Claude Code di terminal. Cukup buka claude dan jalankan `/install-github-app`.

Perintah ini akan memandu Anda melalui pengaturan aplikasi GitHub dan secret yang diperlukan.

## Pengaturan manual

Jika perintah `/install-github-app` gagal atau Anda lebih suka pengaturan manual, silakan ikuti instruksi pengaturan manual ini:

1. **Instal aplikasi Claude GitHub** ke repositori Anda: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   Aplikasi Claude GitHub memerlukan izin repositori berikut:

   * **Contents**: Baca & tulis (untuk memodifikasi file repositori)
   * **Issues**: Baca & tulis (untuk merespons issues)
   * **Pull requests**: Baca & tulis (untuk membuat PR dan mendorong perubahan)

2. **Tambahkan ANTHROPIC_API_KEY** ke rahasia repositori Anda
3. **Salin file alur kerja** dari examples/claude.yml ke direktori `.github/workflows/` repositori Anda
```

---

## PORTUGUESE (Brazilian)
Source: anthropic:632233

```
### Claude Code Action

Esta GitHub Action permite que você execute Claude Code dentro dos seus fluxos de trabalho do GitHub Actions. Você pode usar isso para construir qualquer fluxo de trabalho personalizado sobre Claude Code.

[Ver repositório →](https://github.com/anthropics/claude-code-action)

## Configuração

## Configuração rápida

A maneira mais fácil de configurar esta action é através do Claude Code no terminal. Apenas abra claude e execute `/install-github-app`.

Este comando irá guiá-lo através da configuração do app GitHub e segredos necessários.

## Configuração manual

Se o comando `/install-github-app` falhar ou você preferir configuração manual, por favor siga estas instruções de configuração manual:

1. **Instale o app Claude GitHub** no seu repositório: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   O app Claude GitHub requer as seguintes permissões de repositório:

   * **Contents**: Leitura e escrita (para modificar arquivos do repositório)
   * **Issues**: Leitura e escrita (para responder a issues)
   * **Pull requests**: Leitura e escrita (para criar PRs e fazer push de mudanças)

2. **Adicione ANTHROPIC_API_KEY** aos segredos do seu repositório
3. **Copie o arquivo de fluxo de trabalho** de examples/claude.yml para o diretório `.github/workflows/` do seu repositório
```

---

## KOREAN
Source: anthropic:558795

```
### Claude Code Action

이 GitHub Action을 사용하면 GitHub Actions 워크플로우 내에서 Claude Code를 실행할 수 있습니다. 이를 사용하여 Claude Code 위에 사용자 정의 워크플로우를 구축할 수 있습니다.

[저장소 보기 →](https://github.com/anthropics/claude-code-action)

## 설정

## 빠른 설정

이 액션을 설정하는 가장 쉬운 방법은 터미널에서 Claude Code를 통하는 것입니다. claude를 열고 `/install-github-app`을 실행하기만 하면 됩니다.

이 명령은 GitHub 앱과 필요한 시크릿 설정을 안내합니다.

## 수동 설정

`/install-github-app` 명령이 실패하거나 수동 설정을 선호하는 경우 다음 수동 설정 지침을 따르세요:

1. **Claude GitHub 앱을 설치**하여 저장소에: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   Claude GitHub 앱에는 다음 저장소 권한이 필요합니다:

   * **Contents**: 읽기 및 쓰기 (저장소 파일 수정)
   * **Issues**: 읽기 및 쓰기 (이슈에 응답)
   * **Pull requests**: 읽기 및 쓰기 (PR 생성 및 변경 사항 푸시)

2. **ANTHROPIC_API_KEY를 추가**하여 저장소 시크릿에
3. **워크플로우 파일을 복사**하여 examples/claude.yml에서 저장소의 `.github/workflows/` 디렉터리로
```

---

## ITALIAN
Source: anthropic:411860

```
### Claude Code Action

Questa GitHub Action ti permette di eseguire Claude Code all'interno dei tuoi flussi di lavoro GitHub Actions. Puoi usarla per costruire qualsiasi flusso di lavoro personalizzato sopra Claude Code.

[Visualizza repository →](https://github.com/anthropics/claude-code-action)

## Configurazione

## Configurazione rapida

Il modo più semplice per configurare questa action è attraverso Claude Code nel terminale. Apri semplicemente claude ed esegui `/install-github-app`.

Questo comando ti guiderà attraverso la configurazione dell'app GitHub e dei segreti richiesti.

## Configurazione manuale

Se il comando `/install-github-app` fallisce o preferisci la configurazione manuale, segui queste istruzioni di configurazione manuale:

1. **Installa l'app GitHub Claude** nel tuo repository: 
   [https://github.com/apps/claude](https://github.com/apps/claude)

   L'app GitHub Claude richiede i seguenti permessi del repository:

   * **Contents**: Lettura e scrittura (per modificare i file del repository)
   * **Issues**: Lettura e scrittura (per rispondere alle issues)
   * **Pull requests**: Lettura e scrittura (per creare PR e spingere le modifiche)

2. **Aggiungi ANTHROPIC_API_KEY** ai segreti del tuo repository
3. **Copia il file del flusso di lavoro** da examples/claude.yml alla directory `.github/workflows/` del tuo repository
```

---

## Notes

All excerpts are verbatim from the Anthropic documentation as retrieved via the blz CLI search tool. The structure, verb usage, and terminology are preserved exactly as published.

Key observations:
- Commands (`/install-github-app`) remain identical across all languages
- Structural flow is preserved (Quick Setup → Manual Setup)
- Permission terminology is consistent (Contents, Issues, Pull Requests)
- Native language verbs are used throughout (설정, configurar, konfigurieren, etc.)

