# `blz config`

Manage CLI configuration and user preferences. Without subcommands, `blz config` launches an interactive menu for toggling settings per scope (global, local, or project).

## Usage

```bash
blz config [SUBCOMMAND]
```

### Subcommands

- `set <key> <value> [--scope global|local|project]` – Set a configuration value. Scope defaults to `global`.
- `get [<key>] [--scope global|local|project]` – Show one or all values. Without arguments, prints a summary table and the effective value.

Supported keys:

- `add.prefer_full` – Whether `blz add` should automatically select `llms-full.txt` when available.

### Examples

```bash
# Toggle prefer_full globally
blz config set add.prefer_full true

# Override prefer_full for the current directory only
blz config set add.prefer_full false --scope local

# Inspect all scopes
blz config get
```

### Scope behaviour

- **global** – Writes to the global `config.toml` (e.g. `~/.config/blz/config.toml`).
- **project** – Writes to the project configuration (the directory pointed to by `BLZ_CONFIG_DIR`/`BLZ_CONFIG`, or `./.blz/config.toml` if none is defined).
- **local** – Stores per-directory overrides in `blz.json` under the global state directory.

The effective value used by `blz add` is resolved in the following order: local override → project config → global config → default (`false`).
