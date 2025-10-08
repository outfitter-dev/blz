# 20250919 — Dual Flavor Ingest/Search Slice

## Summary
- Centralized flavor handling (`utils/flavor.rs`) and added `Storage::available_flavors` so we can list cached variants.
- Refactored `blz update` to ingest, archive, and reindex every available flavor, persist per-source overrides, and reuse alias metadata.
- Updated Tantivy queries and the CLI search path to respect resolved default flavors; introduced `search_flavor` regression coverage.
- Added per-source override persistence tests plus docs describing the multi-flavor flow and configuration precedence.

## Validation
- `cargo test -p blz-core`
- `cargo test -p blz-cli`
- `cargo test -p blz-cli --test search_flavor`

## Next
- Audit CLI flag coverage and normalize shared switches across commands.
- Consider adding `blz search --flavor` for manual overrides once the flag matrix settles.
- Roll findings into release notes after the flag audit.
