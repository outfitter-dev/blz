# Agent Rules Files

## Best Practices

- Use "@path/to/file.md" for referencing documentation with the intent of embedding the file as context.
- Use `[link text](path/to/file.md)` for inline links to documentation without resulting in embedding the context.
- Use backticks `@path/to/file.md` for file mentions to indicate it's a reference.
- Keep your rules DRY
  - Smaller rules files
  - Embed or reference docs or other rules files as standard practice

## Documentation Maintenance

### Keep AGENTS.md/CLAUDE.md in sync

When making significant changes:

1. Update `./AGENTS.md` with new patterns or architecture changes
2. Run `./.agents/scripts/sync-agents-md.sh` to sync to CLAUDE.md files
3. AGENTS.md is the source of truth - always edit it, not CLAUDE.md
