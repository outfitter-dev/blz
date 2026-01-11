# MCP Tools Reference

Complete reference for all BLZ MCP tools with schemas, examples, and error handling.

## Tool Catalog

BLZ uses an action-based dispatch pattern with two consolidated tools:

| Tool | Purpose | Actions |
|------|---------|---------|
| [`find`](#find) | Search, retrieve & browse documentation | `search`, `get`, `toc` |
| [`blz`](#blz) | Source management & metadata | `list`, `add`, `remove`, `refresh`, `info`, `validate`, `history`, `help` |

---

## `find`

Unified tool for searching documentation, retrieving exact content spans, and browsing table of contents.

### Schema

```json
{
  "name": "find",
  "description": "Search & retrieve documentation snippets",
  "inputSchema": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": ["search", "get", "toc"],
        "description": "Action to execute (auto-inferred if omitted)"
      },
      "query": {
        "type": "string",
        "description": "Search text (triggers search action)"
      },
      "snippets": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Citation strings e.g. 'bun:10-20,30-40' (triggers get action)"
      },
      "source": {
        "oneOf": [
          {"type": "string"},
          {"type": "array", "items": {"type": "string"}}
        ],
        "description": "Source filter: 'all', single alias, or array of aliases"
      },
      "contextMode": {
        "type": "string",
        "enum": ["none", "symmetric", "all"],
        "default": "none",
        "description": "Snippet expansion mode"
      },
      "context": {
        "type": "integer",
        "minimum": 0,
        "maximum": 50,
        "default": 0,
        "description": "Lines of padding (alias: linePadding)"
      },
      "maxResults": {
        "type": "integer",
        "minimum": 1,
        "maximum": 1000,
        "default": 10,
        "description": "Limit search hits (default: 10)"
      },
      "format": {
        "type": "string",
        "enum": ["concise", "detailed"],
        "default": "concise",
        "description": "Response format (default: concise)"
      },
      "headingsOnly": {
        "type": "boolean",
        "default": false,
        "description": "Restrict matches to heading text only"
      },
      "maxLines": {
        "type": "integer",
        "description": "Maximum lines to return for snippet retrieval"
      },
      "headings": {
        "type": "string",
        "description": "Filter TOC by heading levels (e.g., '1,2' or '<=2')"
      },
      "tree": {
        "type": "boolean",
        "default": false,
        "description": "Return hierarchical TOC tree (default: false)"
      },
      "maxDepth": {
        "type": "integer",
        "description": "Maximum heading depth to include"
      }
    }
  }
}
```

### Actions

The tool auto-infers the action based on parameters:

| Action | Trigger | Purpose |
|--------|---------|---------|
| `search` | `query` provided | Full-text search across sources |
| `get` | `snippets` provided | Retrieve content by citation |
| `toc` | No query/snippets, or explicit action | Browse table of contents |

### Response Format

```typescript
{
  action: "search" | "get" | "toc";
  searchResults?: Array<{
    source: string;      // Source identifier
    lines: string;       // Line range "start-end"
    score: number;       // BM25 relevance score
    snippet: string;     // Text preview (~160 chars in concise mode)
    headingPath?: string; // Hierarchical path (detailed mode only)
  }>;
  snippetResults?: Array<{
    source: string;
    content: string;     // Retrieved content
    lineStart: number;   // Starting line (1-based)
    lineEnd: number;     // Ending line (1-based, inclusive)
  }>;
  toc?: {
    source: string;
    entries: Array<{
      headingPath: string[];
      lines: string;
      anchor?: string;
      children?: Array<...>; // Tree mode only
    }>;
    tree: boolean;
  };
  executed: {
    searchExecuted: boolean;
    snippetsExecuted: boolean;
    tocExecuted: boolean;
  };
}
```

### Examples

#### Search

```json
{
  "name": "find",
  "arguments": {
    "query": "test runner",
    "source": "bun",
    "maxResults": 5
  }
}
```

#### Retrieve Snippet with Context

```json
{
  "name": "find",
  "arguments": {
    "snippets": ["bun:304-324"],
    "contextMode": "symmetric"
  }
}
```

#### Browse TOC

```json
{
  "name": "find",
  "arguments": {
    "action": "toc",
    "source": "bun",
    "headings": "<=2",
    "tree": true
  }
}
```

#### Multi-Source Search

```json
{
  "name": "find",
  "arguments": {
    "query": "authentication",
    "source": ["bun", "node", "deno"],
    "maxResults": 10
  }
}
```

### Error Handling

| Error Code | Reason | Example |
|------------|--------|---------|
| `-32000` | Source not found | `{"source": "invalid"}` |
| `-32002` | Invalid citation format | `{"snippets": ["bun:abc-def"]}` |
| `-32010` | Index error | Corrupted index |
| `-32602` | Invalid params | `{"context": 100}` (exceeds max) |

---

## `blz`

Unified tool for source management and metadata operations.

### Schema

```json
{
  "name": "blz",
  "description": "Manage documentation sources",
  "inputSchema": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": ["list", "add", "remove", "refresh", "info", "validate", "history", "help"],
        "description": "Action to execute (auto-inferred if omitted)"
      },
      "alias": {
        "type": "string",
        "description": "Source identifier (for add/remove/refresh/info/validate/history)"
      },
      "url": {
        "type": "string",
        "description": "Custom URL (for add, uses registry if omitted)"
      },
      "force": {
        "type": "boolean",
        "description": "Overwrite existing source (for add)"
      },
      "kind": {
        "type": "string",
        "enum": ["installed", "registry", "all"],
        "description": "Filter for list"
      },
      "query": {
        "type": "string",
        "description": "Search filter for list"
      },
      "reindex": {
        "type": "boolean",
        "description": "Re-index without fetching (for refresh)"
      },
      "all": {
        "type": "boolean",
        "description": "Refresh all sources"
      }
    }
  }
}
```

### Actions

| Action | Required Params | Purpose |
|--------|-----------------|---------|
| `list` | — | List installed and registry sources |
| `add` | `alias` | Add source from registry or custom URL |
| `remove` | `alias` | Remove source and cached data |
| `refresh` | `alias` or `all` | Update cached sources |
| `info` | `alias` | Show detailed source information |
| `validate` | `alias` (optional) | Validate source data integrity |
| `history` | `alias` | Show archive history |
| `help` | — | Return usage guidance |

The action is auto-inferred from parameters:
- `url` or `force` provided → `add`
- `reindex` or `all` provided → `refresh`
- `alias` provided alone → `info`
- Nothing provided → `list`

### Response Format

```typescript
{
  action: "list" | "add" | "remove" | "refresh" | "info" | "validate" | "history" | "help";

  // For list action
  list?: {
    sources: Array<{
      alias: string;
      title?: string;
      url: string;
      kind: "installed" | "registry";
      fetchedAt?: string;
      suggestedCommand?: string;
      metadata?: { totalLines: number; headings: number };
    }>;
  };

  // For add action
  add?: {
    alias: string;
    url: string;
    message: string;
  };

  // For remove action
  remove?: {
    alias: string;
    message: string;
    info?: {
      alias: string;
      url: string;
      totalLines: number;
      fetchedAt: string;
    };
  };

  // For refresh action
  refresh?: {
    results: Array<{
      alias: string;
      status: "refreshed" | "unchanged" | "reindexed" | "error";
      headings?: number;
      lines?: number;
      headingsBefore?: number;
      headingsAfter?: number;
      filtered?: number;
      message?: string;
    }>;
    refreshed: number;
    unchanged: number;
    reindexed: number;
    errors: number;
  };

  // For info action
  info?: {
    alias: string;
    url: string;
    variant: string;
    aliases: string[];
    lines: number;
    headings: number;
    sizeBytes: number;
    lastUpdated?: string;
    etag?: string;
    checksum?: string;
    cachePath: string;
    filterStats?: { ... };
  };

  // For validate/history actions
  validate?: { stdout: string; stderr: string; exitCode: number };
  history?: { stdout: string; stderr: string; exitCode: number };

  // For help action
  help?: object;
}
```

### Examples

#### List Sources

```json
{
  "name": "blz",
  "arguments": {
    "action": "list",
    "kind": "installed"
  }
}
```

#### Add from Registry

```json
{
  "name": "blz",
  "arguments": {
    "action": "add",
    "alias": "astro"
  }
}
```

#### Add Custom URL

```json
{
  "name": "blz",
  "arguments": {
    "action": "add",
    "alias": "my-docs",
    "url": "https://example.com/llms.txt"
  }
}
```

#### Refresh All Sources

```json
{
  "name": "blz",
  "arguments": {
    "action": "refresh",
    "all": true
  }
}
```

#### Reindex Without Fetching

```json
{
  "name": "blz",
  "arguments": {
    "action": "refresh",
    "alias": "bun",
    "reindex": true
  }
}
```

#### Get Source Info

```json
{
  "name": "blz",
  "arguments": {
    "alias": "bun"
  }
}
```

### Error Handling

| Error Code | Reason | Solution |
|------------|--------|----------|
| `-32000` | Source not found | Check alias or add source first |
| `-32001` | Source already exists | Use `force: true` |
| `-32602` | Missing required parameter | Provide `alias` for action |
| `-32010` | Fetch/index failed | Check network, URL validity |

---

## Common Patterns

### Pattern 1: Search Then Retrieve

```javascript
// Step 1: Search
const search = await callTool("find", {
  query: "test runner",
  source: "bun",
  maxResults: 5
});

// Step 2: Get full content
const citation = `${search.searchResults[0].source}:${search.searchResults[0].lines}`;
const content = await callTool("find", {
  snippets: [citation],
  contextMode: "symmetric"
});
```

### Pattern 2: Check Before Add

```javascript
// Step 1: Check if available
const sources = await callTool("blz", {
  action: "list",
  query: "astro"
});

// Step 2: Add if in registry
const astro = sources.list.sources.find(s => s.alias === "astro");
if (astro?.kind === "registry") {
  await callTool("blz", {action: "add", alias: "astro"});
}
```

### Pattern 3: Multi-Source Search

```javascript
// Cross-source search with array
const results = await callTool("find", {
  query: "authentication",
  source: ["bun", "react", "next"],
  maxResults: 15
});

// Results are merged and re-ranked globally
```

### Pattern 4: Incremental Context

```javascript
// Start with minimal context
let content = await callTool("find", {
  snippets: ["bun:304-324"],
  contextMode: "none"
});

// If not enough info, expand to full section
if (needsMoreContext(content)) {
  content = await callTool("find", {
    snippets: ["bun:304-324"],
    contextMode: "all",
    maxLines: 200
  });
}
```

### Pattern 5: Browse Structure First

```javascript
// Get high-level TOC
const toc = await callTool("find", {
  action: "toc",
  source: "bun",
  headings: "<=2",
  tree: true
});

// Find relevant section, then search within it
const section = toc.toc.entries.find(e =>
  e.headingPath.includes("Testing")
);
```

---

## Next Steps

- [README.md](README.md) - Overview and capabilities
- [SETUP.md](SETUP.md) - Client configuration
- [CLI Documentation](../cli/commands.md) - CLI equivalents
