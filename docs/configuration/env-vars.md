## Environment Variables

These variables override configuration at runtime. All booleans accept: 1, true, yes, on (case-insensitive).

- `BLZ_CONFIG` — Absolute path to `config.toml`
- `BLZ_CONFIG_DIR` — Directory containing `config.toml`
- `BLZ_GLOBAL_CONFIG_DIR` — Override the global configuration directory used for `config.toml`
- `BLZ_ROOT` — Override cache root directory
- `BLZ_REFRESH_HOURS` — Integer hours between refresh checks
- `BLZ_MAX_ARCHIVES` — Integer count of archived versions to keep per source
- `BLZ_FETCH_ENABLED` — Enable/disable network fetches (bool)
- `BLZ_FOLLOW_LINKS` — Link policy: `none` | `first_party` | `allowlist`
- `BLZ_ALLOWLIST` — Comma-separated list of domains (used with `allowlist`)
- `BLZ_PREFER_LLMS_FULL` — No longer used. BLZ automatically prefers llms-full.txt when available.
- `BLZ_SUPPRESS_DEPRECATIONS` — Suppress CLI deprecation warnings (bool)
- `BLZ_FORCE_NON_INTERACTIVE` — Force CLI subcommands to skip confirmation prompts (bool)
- `BLZ_DISABLE_GUARD` — Disable the parent process watchdog thread (bool)
- `BLZ_PARENT_GUARD_INTERVAL_MS` — Poll interval in milliseconds for the parent watchdog (100-10000; defaults to 500)
- `BLZ_PARENT_GUARD_TIMEOUT_MS` — Optional watchdog timeout in milliseconds before forcing an exit
- `BLZ_PARENT_GUARD_TIMEOUT_SECS` — Alternative timeout expressed in seconds (ignored if `_MS` is set)
- `BLZ_OUTPUT_FORMAT` — Default CLI output: `json` | `text` | `jsonl`
- `NO_COLOR` — Disable ANSI colors in output
