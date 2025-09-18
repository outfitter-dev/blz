## Environment Variables

These variables override configuration at runtime. All booleans accept: 1, true, yes, on (case-insensitive).

- `BLZ_CONFIG` — Absolute path to `config.toml`
- `BLZ_CONFIG_DIR` — Directory containing `config.toml`
- `BLZ_ROOT` — Override cache root directory
- `BLZ_REFRESH_HOURS` — Integer hours between refresh checks
- `BLZ_MAX_ARCHIVES` — Integer count of archived versions to keep per source
- `BLZ_FETCH_ENABLED` — Enable/disable network fetches (bool)
- `BLZ_FOLLOW_LINKS` — Link policy: `none` | `first_party` | `allowlist`
- `BLZ_ALLOWLIST` — Comma-separated list of domains (used with `allowlist`)
- `BLZ_PREFER_LLMS_FULL` — Prefer llms-full.txt when available during updates (bool)
- `BLZ_OUTPUT_FORMAT` — Default CLI output: `json` | `text` | `jsonl`
- `NO_COLOR` — Disable ANSI colors in output
