# BLZ Get JSON Migration

Last updated: 2025-10-16

## Summary

- `blz get` now accepts multiple `alias[:range]` targets and emits a `requests[]` array in JSON/JSONL mode.
- Single-span responses surface `snippet`, `lineStart`, and `lineEnd`; multi-range requests populate `ranges[]` inside the entry; multi-source batches add one entry per alias.
- Execution metadata (`executionTimeMs`, `totalSources`) is included to help downstream automation measure cost.

## Updated Consumers

| Consumer | Status | Notes |
| --- | --- | --- |
| `blz-cli` (CLI users & scripts) | ✅ Complete | Implemented in BLZ-199 with regression coverage. Prompts/docs refreshed in BLZ-200/BLZ-201. |
| Factory command templates (`docs/factory`) | ✅ Complete | Prompt snippets now reference `requests[]` fields. |
| Bundled llms-full docs (`docs/llms/blz/`) | ✅ Complete | User guide updated to describe the new payload. |
| `blz-mcp` server | ⚠️ Follow-up | Needs updated serializers and prompt guidance. Filed Linear issue BLZ-205. |
| External agents relying on `.content` | ⚠️ Follow-up | Communicate via release notes; advise switching to `requests[*].snippet`. |

## Migration Guidance

1. Replace references to `.content`/`.lineNumbers` with the appropriate `requests[]` shapes.
2. For multi-range scenarios, iterate each `requests[i].ranges[]` entry to gather the individual snippets for every source.
3. When batching multiple sources, check each `requests[]` entry separately before merging into a prompt or report.
4. Capture `checksum` alongside the snippet to detect stale caches.

## Release Communication

- Changelog entry added under `[Unreleased]`.
- Agent instructions and CLI prompts updated to demonstrate multi-source usage.
- Include a migration callout in the next release blog/announcement referencing this document.
