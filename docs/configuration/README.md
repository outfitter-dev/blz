## Configuration Overview

This section documents all configuration mechanisms for blz in one place: global config files, per-source settings, and environment variables. It also explains precedence and where files live on each platform.

Index:
- Global config: [global-config.md](./global-config.md)
- Per‑source settings: [per-source.md](./per-source.md)
- Environment variables: [env-vars.md](./env-vars.md)
- Defaults & precedence: [defaults.md](./defaults.md)
- CLI preferences (history/defaults): see [`blz history`](../commands/history.md)

### Precedence

From lowest → highest priority:
1) Built-in defaults
2) Global config `config.toml`
3) Optional `config.local.toml` in the same directory as `config.toml`
4) Per-source `settings.toml` (only for that source)
5) Environment variables (BLZ_*)
6) CLI flags

### Locations

- Global config file:
  - Linux: `~/.config/blz/config.toml`
  - macOS: `~/Library/Application Support/dev.outfitter.blz/config.toml`
  - Windows: `%APPDATA%\dev.outfitter.blz\config.toml`
  - Or use `BLZ_CONFIG` (file) or `BLZ_CONFIG_DIR` (dir with `config.toml`)

- Cache root (data dir):
  - Linux: `~/.local/share/dev.outfitter.blz/`
  - macOS: `~/Library/Application Support/dev.outfitter.blz/`
  - Windows: `%APPDATA%\dev.outfitter.blz\`

See global-config.md for a full example.
