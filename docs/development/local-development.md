# Local Development Setup

This guide covers how to run `blz` from source without disturbing an existing release installation. It focuses on the opt-in developer profile (`blz-dev`) and complementary tooling.

## When To Use The Dev Profile

Use the `blz-dev` binary when you want to:

- Test changes locally while keeping the stable `blz` binary on your PATH.
- Maintain isolated config/cache data (`blz-dev` writes to `~/.blz-dev/` or platform equivalents).
- Exercise new functionality without touching production indexes.

The dev profile is gated behind the `dev-profile` cargo feature and never ships with release artifacts. You must install it manually.

## Installing `blz-dev`

```bash
# From the repository root
./install-dev.sh --root "$HOME/.local/share/blz-dev"
```

The script wraps `cargo install --features dev-profile --bin blz-dev --path crates/blz-cli` and passes through any extra flags you supply (`--root`, `--force`, `--locked`, etc).

After installation, add the target `bin` directory to your PATH *ahead* of other blz binaries:

```bash
export PATH="$HOME/.local/share/blz-dev/bin:$PATH"
```

Alternatively, call the binary directly via absolute path.

## Hydrating From Existing Installation

If you already have sources configured in your production `blz` installation, you can copy them to `blz-dev` to start testing immediately:

```bash
# Copy everything (config + sources)
./hydrate-dev.sh

# Preview what would be copied
./hydrate-dev.sh --dry-run

# Copy only configuration files
./hydrate-dev.sh --config-only

# Copy only source data and indices
./hydrate-dev.sh --sources-only

# Overwrite existing blz-dev data
./hydrate-dev.sh --force
```

The script is XDG-aware and handles both macOS and Linux paths automatically. It copies:

- **Config files**: `config.toml`, `data.json`, `history.jsonl`
- **Source data**: All cached `llms.txt` files and search indices

This is particularly useful when:
- Testing migrations or upgrades against real data
- Benchmarking performance with your actual source set
- Developing features that depend on existing indices

## Where Data Lives

The profile-aware path logic in `blz-core` puts dev metadata under dedicated directories:

- Config: `$XDG_CONFIG_HOME/blz-dev/` or `~/.blz-dev/` (non-XDG)
- Data/indexes: `$XDG_DATA_HOME/blz-dev/` or `~/.blz-dev/`
- Preferences/history: same root as config

These locations are separate from the stable profile (`blz`) to avoid cross-contamination.

## Building & Testing

| Task | Command |
| --- | --- |
| Format | `cargo fmt` |
| Check primary binary | `cargo check -p blz-cli` |
| Check dev binary | `cargo check -p blz-cli --features dev-profile --bin blz-dev` |
| Run tests | `cargo test --workspace` |

No additional features are required to run the standard test suite, but the second `cargo check` ensures the dev entrypoint compiles.

## Cleaning Up

Remove the dev installation by deleting the install root and the profile directories:

```bash
rm -rf "$HOME/.local/share/blz-dev"
rm -rf "${XDG_CONFIG_HOME:-$HOME/.blz-dev}"
rm -rf "${XDG_DATA_HOME:-$HOME/.blz-dev}"
```

Be careful to double-check paths before running the commands above.

## Related Docs

- [Development Workflow](./workflow.md)
- [CI/CD Pipeline](./ci-cd.md)
- [Contributing Guide](./contributing.md)
