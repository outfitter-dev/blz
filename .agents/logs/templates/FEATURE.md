---
date: # e.g. 2025-08-29 22:00 UTC
feature: # e.g. update-command, mcp-server
issue: # GitHub and Linear (if applicable) e.g. #123 / BLZ-45
agent: # e.g. claude, codex, cursor, etc.
---

# Feature - [Name]

## Goal

## Requirements

### Functional

### Non-Functional

## Design

### Architecture

### API

### Data Model

## Implementation

### Tasks

- [ ] …

### Dependencies

## Testing Strategy

### Unit Tests

### Integration Tests

### Manual Testing

## Documentation

## Validation Criteria

---

## Example

```markdown
---
date: 2025-08-29 22:00 UTC
feature: update-command
issue: #33
agent: claude
---

# Feature - Update Command

## Goal

Implement `blz update` command to refresh cached documentation with conditional fetching using ETags.

## Requirements

### Functional

- Update single source: `blz update <alias>`
- Update all sources: `blz update --all`
- Show update status and statistics
- Archive previous version before updating
- Rebuild search index after update

### Non-Functional

- Use conditional GET with ETag/Last-Modified
- Minimize bandwidth usage
- Complete update in <5s for typical docs
- Atomic updates (no partial state)

## Design

### Architecture

```text

CLI Command → UpdateService → Fetcher (ETag check)
                ↓
            Storage (archive & write)
                ↓
            Indexer (rebuild)

```text

### API

```rust
pub async fn update_source(alias: &str) -> Result<UpdateResult> {
    // Returns: Updated, NotModified, or Error
}

pub struct UpdateResult {
    pub status: UpdateStatus,
    pub stats: UpdateStats,
}
```text

### Data Model

Store ETag and Last-Modified in `llms.json`:

```json
{
  "etag": "W/\"abc123\"",
  "last_modified": "2025-08-29T10:00:00Z",
  "sha256": "...",
  "updated_at": "2025-08-29T10:00:00Z"
}
```text

## Implementation

### Tasks

- [x] Add ETag support to fetcher
- [x] Implement conditional GET logic
- [x] Add archive functionality
- [x] Create update command in CLI
- [x] Add progress indicators
- [ ] Add --force flag to bypass cache
- [ ] Add --dry-run flag

### Dependencies

- reqwest with `If-None-Match` header support
- Archive functionality in storage module

## Testing Strategy

### Unit Tests

- Mock HTTP responses with 304 Not Modified
- Test ETag comparison logic
- Verify archive creation

### Integration Tests

- Full update flow with mock server
- Test both modified and not-modified paths
- Verify index rebuilding

### Manual Testing

- Test with real llms.txt sources
- Verify bandwidth savings with proxy
- Test interrupted update recovery

## Documentation

- Update CLI help text
- Add examples to README
- Document in architecture.md

## Validation Criteria

- [x] Saves bandwidth with 304 responses
- [x] Archives are created with timestamps
- [x] Index is rebuilt after updates
- [x] Progress shown during update
- [x] Clear status messages

```text
