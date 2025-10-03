# blz add Batch Manifest Design

**Date:** 2025-10-03
**Author:** Codex agent (GPT-5)
**Status:** Proposed (seeking feedback)

## Context

Recent work on the v1.0.0-beta.1 line expanded the CLI surface area (`blz stats`, multi-source search, registry tooling), but `blz add` still only supports single-source additions via `<alias> <url>` or manual registry lookups. Matt requested a batch-oriented workflow that lets teams pin the exact set of docs their agents depend on, with first-class support for local source files and parity with the curated registry metadata. Reviewer feedback highlighted the need for a design pass before implementation so that `blz update` remains compatible and the manifest format is sustainable.

## Goals

- Provide an ergonomic, human-editable manifest format for adding multiple sources in one command.
- Capture the same descriptive metadata the curated registry exposes (description, category, tags, npm, github aliases).
- Support both remote URLs and local file paths while preserving `blz update` semantics.
- Persist enough origin information so that future `blz update` runs understand how each source was seeded (manifest path + entry identifier).
- Minimize churn for existing CLI users (`blz add <alias> <url>` continues to work unchanged).

## Non-Goals

- Automatically syncing manifests back into the public registry (out of scope).
- Replacing the current registry build process or file layout.
- Designing the full UX for team sharing/sync (this spec focuses on local CLI behavior).

## Manifest Format Proposal

- **File type:** TOML for parity with `registry/sources/*.toml` and readability.
- **Top-level structure:** `version` (string) + one or more `[[source]]` tables.
- **Source table fields:**
  - `alias` *(string, required)* – canonical cache identifier; must satisfy existing alias validation.
  - `name` *(string, optional)* – human-friendly name; defaults to `alias` if omitted.
  - `description` *(string, optional but recommended)* – matches registry semantics.
  - `url` *(string, optional)* – HTTP(S) endpoint for llms(.full).txt.
  - `path` *(string, optional)* – absolute or manifest-relative filesystem path for local docs.
    - Exactly one of `url` or `path` must be present.
  - `category` *(string, optional)* – freeform category (framework, runtime, etc.).
  - `tags` *(array<string>, optional)* – descriptive tags; persisted into metadata.
  - `aliases` *(table, optional)* – nested tables containing additional alias sets:
    - `aliases.npm` *(array<string>)* – npm package names.
    - `aliases.github` *(array<string>)* – org/repo slugs.
  - `registry` *(table, optional)* – knobs to future-proof hydration from curated registry (fields TBD, currently `id` to record canonical registry id when imported).
  - `notes` *(string, optional)* – freeform comment retained for documentation but ignored by CLI logic.

Example (`docs/blz.sources.toml`):

```toml
version = "1"

[[source]]
alias = "bun"
name = "Bun"
description = "Fast all-in-one JavaScript runtime"
url = "https://bun.sh/llms-full.txt"
category = "runtime"
tags = ["javascript", "runtime", "bundler", "package-manager"]

  [source.aliases]
  npm = ["bun"]
  github = ["oven-sh/bun"]

[[source]]
alias = "internal-sdk"
name = "Internal SDK"
path = "./docs/internal-sdk.llms.txt"
description = "Private SDK docs"
category = "internal"
tags = ["sdk", "private"]
notes = "Generated nightly by CI"
```

### Parsing Rules

- Relative paths (`path`, `url`) are resolved against the manifest file directory.
- Empty or comment-only lines are ignored (standard TOML behavior).
- Unknown fields trigger a warning (preserved in the in-memory structure) but do not abort unless `--strict` is supplied (future enhancement).

### Per-Source Descriptor Files

- Regardless of whether a source originates from a batch manifest, a single `blz add`, or registry tooling, we persist a **descriptor** file per source under the config directory:
  - Location: `${CONFIG_DIR}/sources/<alias>.toml` (mirrors the existing registry naming, default `~/.config/blz/sources/<alias>.toml` on macOS/Linux when XDG dirs are in play).
  - Schema: same fields as manifest entry plus CLI-resolved defaults (normalized `alias`, resolved `url` vs. `path`, derived metadata).
  - When a manifest entry is imported, each resulting descriptor records the original manifest path/version in `source.origin.manifest` (see below) so the relationship remains intact.
- Default field values when omitted:
  - `name` → Title Case alias
  - `category` → `uncategorized`
  - `description` → empty string (descriptor retains the key; runtime metadata omits it)
  - `tags`, `npmAliases`, `githubAliases` → `[]`
- Benefits:
  - `blz remove <alias>` can delete the descriptor atomically without touching neighboring sources.
  - Local-only adds get durable provenance (path, description, tags) even if the original manifest is transient.
  - Future UX (e.g., `blz list --verbose`) can surface descriptor metadata without reparsing large manifests.
- We still accept a standalone manifest file for batch operations, but internal storage is normalized to per-source descriptors to match the desired "nuke one source" workflow.

## CLI Experience

### New Flags / Modes

- `blz add --manifest <FILE>` – reads the TOML manifest and attempts to add every listed source.
- `blz add --manifest <FILE> --only <alias1,alias2>` – restricts to a subset of entries (comma-separated list).
- Existing positional `<alias> <url>` mode remains default when `--manifest` is absent. Flags `--aliases`, `--yes`, `--dry-run`, and `--quiet` continue to apply.
- `--dry-run` in manifest mode emits a JSON array containing the same analysis payload currently returned by single-source dry runs, keyed by alias.
- Progress reporting: manifest mode shows a multi-step progress bar (aliases processed / total) plus per-alias spinner reuse from the single-source flow.
- New convenience flags mirror descriptor fields for single-source adds:
  - `--name` (defaults to Title Case alias, e.g., `react-hooks` → `React Hooks`)
  - `--description`
  - `--category` (defaults to `uncategorized`)
  - `--tags` (comma-delimited; applied to both descriptor and runtime metadata)

### Exit Semantics

- Success when **all** adds succeed. If any alias fails, exit code is non-zero and a summary table lists successes, failures, and reason codes.
- On partial failure, successes remain cached; users can rerun with `--only` to retry failures.

### Surfacing Descriptor Metadata (`blz list`)

- Add `--details` flag to `blz list` (text mode default remains concise table).
  - Without `--details`, output mirrors current behavior: alias, variant, maybe timestamp.
  - With `--details`, each source prints descriptor metadata (description, category, tags, npm/github aliases, origin type, manifest link, local path) in a readable block.
- `--json` already returns structured output; we expand it to include the full descriptor payload (including `origin`, alias metadata, and timestamps) without requiring `--details`.
- `--raw` (if ever added) would remain unaffected.
- Future enhancement: optional column selection to cherry-pick descriptor fields (`--show description,tags`).

## Storage & Metadata Changes

To support `blz update`, richer descriptors, and per-source config files we extend both the on-disk layout and `blz_core::Source` serialization.

### New Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceOrigin {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ManifestOrigin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<SourceType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestOrigin {
    pub path: String,          // absolute path stored; display-friendly original retained separately if desired
    pub entry_alias: String,   // alias key inside the manifest
    pub version: Option<String>, // manifest version captured for migration logic
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceType {
    Remote { url: String },
    LocalFile { path: String },
}
```

- `Source` gains:
  - `#[serde(default)] pub origin: SourceOrigin`
  - `#[serde(default)] pub description: Option<String>`
  - `#[serde(default)] pub category: Option<String>`
  - `#[serde(default)] pub npm_aliases: Vec<String>`
  - `#[serde(default)] pub github_aliases: Vec<String>`
- The same fields are mirrored inside `llms.json.metadata` because that file already serializes `Source` wholesale.
- Local cache also writes a lightweight `manifest.json` (or piggybacks on `metadata.json`) so humans debugging have direct visibility.
- New helper in `blz_core::storage` provides the config/descriptor root (parallel to the data root) so CLI commands can read/write `${CONFIG_DIR}/sources/<alias>.toml`.
- Descriptor files and `metadata.json` are kept in sync: descriptor is the authoritative intent (URL/path, descriptive metadata), while `metadata.json` captures runtime fetch state (etag/sha256, timestamps).

### Backward Compatibility

- All new fields use `#[serde(default)]` to keep existing metadata readable.
- When loading metadata missing `origin.source_type`, we infer `Remote { url }` to preserve current behavior.
- `Storage::save_source_metadata` continues writing pretty JSON; no migration step is required beyond lazy defaults.

## Update Flow

1. `blz update <alias>` loads metadata as today.
2. If `metadata.origin.source_type` is `Remote`, we reuse the existing HTTP fetch path (no change).
3. If `LocalFile`, we:
   - Resolve the stored absolute path (falling back to manifest-relative if absolute missing and manifest still present).
   - Read file contents directly (new helper `Fetcher::fetch_local(path)` in `blz_core`).
   - Generate SHA256 and proceed through `apply_update` unchanged.
4. Before fetching we read `${CONFIG_DIR}/sources/<alias>.toml` (descriptor). That descriptor is treated as the **source of truth** for descriptive fields and intended origin:
   - If it references an external manifest (`origin.manifest.path`), and that manifest exists, we optionally re-parse the manifest entry and reconcile any drift back into the descriptor (guarded by `--sync-manifest` later).
   - If the descriptor's URL/path differs from cached metadata, we update `origin.source_type` prior to fetch.
5. Missing descriptor file triggers a warning; we fall back to `metadata.json` (preserving backward compatibility) but prompt users to restore descriptors if possible.
6. After a successful update, we persist any metadata changes back to both descriptor and metadata files so they stay aligned.

## Validation & Error Handling

- Manifest parser validates:
  - Each alias uniqueness (case-insensitive) within the manifest.
  - Exactly one of `url` or `path` supplied; error otherwise.
  - Local paths must exist at add-time unless `--allow-missing-local` (future) is specified.
  - URLs must be HTTP(S); we warn on other schemes but allow `file://` mapping to LocalFile.
- During add, metadata fields (tags, npm, github) are normalized (deduped, sorted, lowercased where appropriate) **before** writing descriptors so they stay tidy.
- On failure to parse manifest entry, we collect the error, skip that alias, and proceed to next entry (unless `--strict` requested). Any descriptors created before the failure remain untouched; we never partially overwrite an existing descriptor unless the same alias is being updated.

## Tooling & Tests

- **Unit tests:**
  - Manifest parser conversions (TOML → internal struct) with success/failure cases.
  - Storage serialization round-trips for new metadata fields.
  - Update path switching between remote and local file origins.
- **Integration tests:**
  - `blz add --manifest` happy path (multiple sources, mixed remote/local).
  - `blz update` for manifest-managed alias reflecting manifest edits (metadata + URL change).
  - Dry-run manifest outputs predictably ordered JSON.
  - `blz list --details` emits descriptor metadata; `--json` includes descriptor fields by default.

## Documentation

- New section in `docs/cli.md` (“Batch add from manifest”).
- Add manifest template under `registry/templates/batch-manifest.example.toml` **and** a descriptor template in `registry/templates/source-descriptor.example.toml`.
- Update `docs/cli.md` and `docs/sources.md` to cover the new `blz list --details` flag and expanded JSON output.
- Update agent instructions (if needed) to highlight manifest support for local docs.
- Document quick-install script in README/getting-started and host it at `install.sh` for curl-based installs.

## Rollout Plan

1. Implement manifest parser + internal structs (behind feature flag until stable if desired).
2. Extend metadata structs and ensure existing commands tolerate new fields.
3. Wire CLI flag plumbing and add execution path.
4. Update `blz update` to honor `SourceOrigin`.
5. Documentation + examples.

## Open Questions

- Do we want to surface registry metadata (description/category) in `blz list` immediately, or leave that for a follow-up UX pass?
- How should we handle manifests committed to repos with relative paths when users run `blz add` from a different working directory (suggest storing canonicalized absolute paths but remembering original relative path for display)?

Feedback welcome before moving into implementation.
