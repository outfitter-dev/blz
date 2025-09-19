## Defaults & Precedence

Built-in defaults are applied when no configuration is provided. They can be overridden by the global config, per-source settings, environment variables, and CLI flags.

Key defaults:
- refresh_hours = 24
- max_archives = 10
- fetch_enabled = true
- follow_links = first_party
- allowlist = []
- prefer_llms_full = false
- CLI presentation defaults: `show = []`, `snippet_lines = 3`, `score_precision = 1` (persisted per scope in `blz.json`)
- Per-source flavor override (`blz.json` → `sources[alias].preferred_flavor`) = _unset_ (falls back to resolved defaults)

Precedence (lowest → highest):
1) Built-in defaults
2) Global `config.toml`
3) `config.local.toml` next to the global file
4) Per-source `settings.toml`
5) Environment variables (BLZ_*)
6) CLI flags

Per-source overrides live in `blz.json` alongside CLI presentation preferences. When set (e.g., `blz update docs --flavor full`), they win over config files and environment variables for that alias.
