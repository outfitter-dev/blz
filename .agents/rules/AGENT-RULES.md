# Agent Rules Files

## Best Practices

- Prefer `@path/to/file.md` to embed docs as context, `[text](path/to/file.md)` for inline links without embedding, backticks ``@path/to/file.md`` when merely mentioning a file path.
- Keep your rules DRY
  - Smaller rules files
  - Embed or reference docs or other rules files as standard practice

## Documentation Maintenance

### Keep AGENTS.md/CLAUDE.md in sync

When making significant changes:

1. Update `./AGENTS.md` with new patterns or architecture changes
2. Run `./.agents/scripts/sync-agents-md.sh` to sync to CLAUDE.md files
3. AGENTS.md is the source of truth - always edit it, not CLAUDE.md
