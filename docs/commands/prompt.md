# Prompt Mode

Emit JSON guidance for the CLI or a specific subcommand. This replaces the legacy `blz instruct` command.

```bash
blz --prompt [TARGET]
```

- No `TARGET` → general overview (`blz --prompt`)
- Named target → command-specific workflow (`blz --prompt search`, `blz --prompt add`, `blz --prompt alias`)
- Nested targets use dot notation (`blz --prompt alias.add`, `blz --prompt registry.create-source`)

Outputs are always JSON so agents can feed them directly into prompt templates or store them in tool metadata.

Example:

```bash
blz --prompt search | jq '.summary'
```

> Legacy compatibility: `blz instruct` now prints a deprecation notice directing you to `blz --prompt`.
