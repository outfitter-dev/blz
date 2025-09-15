## Global Config (config.toml)

The global config defines defaults for all sources. Example:

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party" # "none" | "first_party" | "allowlist"
allowlist = ["developer.mozilla.org", "docs.rs"]
prefer_llms_full = false     # Prefer llms-full.txt on update when available

[paths]
# Override cache root (optional)
# root = "/absolute/path/to/cache"
```

Locations:
- Linux: `~/.config/blz/config.toml`
- macOS: `~/Library/Application Support/dev.outfitter.blz/config.toml`
- Windows: `%APPDATA%\dev.outfitter.blz\config.toml`

Overrides:
- `config.local.toml` in the same directory is merged on top of `config.toml`.
- See env-vars.md for environment variable overrides.
