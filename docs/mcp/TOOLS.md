# MCP Tools Reference

Complete reference for all BLZ MCP tools with schemas, examples, and error handling.

## Tool Catalog

| Tool | Purpose | Mutates State |
|------|---------|---------------|
| [`find`](#find) | Search & retrieve documentation | No |
| [`list-sources`](#list-sources) | List installed and registry sources | No |
| [`source-add`](#source-add) | Add documentation source | Yes |
| [`run-command`](#run-command) | Execute whitelisted commands | No |
| [`learn-blz`](#learn-blz) | Get reference data | No |

---

## `find`

Unified tool for searching documentation and retrieving exact content spans.

### Schema

```json
{
  "name": "find",
  "description": "Search & retrieve",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search text"
      },
      "snippets": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Source refs (e.g., 'bun:120-145')"
      },
      "source": {
        "type": "string",
        "description": "Alias of the documentation source to search (required with query)"
      },
      "contextMode": {
        "type": "string",
        "enum": ["none", "symmetric", "all"],
        "description": "Snippet expansion mode"
      },
      "linePadding": {
        "type": "integer",
        "minimum": 0,
        "maximum": 50,
        "description": "Lines to add before/after"
      },
      "maxResults": {
        "type": "integer",
        "minimum": 1,
        "maximum": 50,
        "description": "Limit search hits"
      }
    }
  }
}
```

### Modes

The `find` tool operates in two modes:

1. **Search mode**: Provide `query` to search documentation
2. **Retrieval mode**: Provide `snippets` to retrieve exact content

You can use both modes simultaneously.

### Parameters

#### `query` (string, optional)

Search text for full-text search.

**Examples:**

```javascript
{query: "test runner"}
{query: "async await error handling"}
{query: "configuration options"}
```

#### `snippets` (array of strings, optional)

Citation references to retrieve. Format: `"source:start-end"` or `"source:range1,range2"`.

**Examples:**

```javascript
{snippets: ["bun:304-324"]}
{snippets: ["bun:100-200", "react:500-550"]}
{snippets: ["bun:100-120,130-150"]} // Multiple ranges from same source
```

#### `source` (string, optional)

Alias of the documentation source to search. This field is **required** whenever
`query` is provided because search is currently single-source. When retrieving
snippets, the source is inferred from each citation string.

**Examples:**

```javascript
{source: "bun"}
{source: "react"} // Search React docs
```

**Tip:** To search multiple sources, call the tool repeatedly with different
`source` values.

#### `headingsOnly` (boolean, optional)

Restrict search matches to heading text. Defaults to `false`.

When set to `true`, the search ignores body content and only considers heading
paths. Pair with heading-based queries (optionally prefixed with `#`) to create
stable anchors that survive line shifts. Works with any `contextMode`, and is
ignored when only retrieving `snippets`.

**Examples:**
```javascript
{query: "# Skip tests with the Bun test runner", source: "bun", headingsOnly: true, contextMode: "all"}
{query: "Deploy on Cloudflare", source: "astro", headingsOnly: true}
```

#### `contextMode` (enum, optional)

Controls snippet expansion. Default: `"none"`.

**Options:**

- `"none"`: Return only the requested line range
- `"symmetric"`: Expand to the full heading section
- `"all"`: Return the entire document

**Examples:**

```javascript
// Minimal - just the lines requested
{snippets: ["bun:304-324"], contextMode: "none"}

// Full section - useful for documentation sections
{snippets: ["bun:304-324"], contextMode: "symmetric"}

// Entire doc - when you need all context
{snippets: ["bun:304-324"], contextMode: "all"}
```

**CLI equivalent:** `-C <N>` (symmetric with linePadding), `--context all` (all)

#### `linePadding` (integer, optional)

Add N lines before and after the snippet. Range: 0-50. Default: 0.

Only applies when `contextMode` is `"none"`.

**Examples:**

```javascript
// No padding
{snippets: ["bun:304-324"], linePadding: 0}

// Add 5 lines before and after
{snippets: ["bun:304-324"], linePadding: 5}

// Maximum padding
{snippets: ["bun:304-324"], linePadding: 50}
```

**CLI equivalent:** `-C 5` (grep-style)

#### `maxResults` (integer, optional)

Limit number of search hits. Range: 1-50. Default: 10.

**Examples:**

```javascript
{query: "test", maxResults: 5}   // Top 5 results
{query: "test", maxResults: 20}  // More comprehensive
```

### Response Format

```typescript
{
  snippets: Array<{
    alias: string;           // Source identifier
    lines: string;           // Line range (e.g., "304-324")
    content: string;         // The actual content
    headingPath: string[];   // Breadcrumb trail
    truncated?: boolean;     // If content was capped
  }>;
  hits: Array<{
    alias: string;
    lines: string;
    headingPath: string[];
    snippet: string;         // Preview text (~200 chars)
    score: number;           // BM25 score (0-100)
    sourceUrl?: string;      // Source URL if available
  }>;
  executed: {
    searched: boolean;              // Did we run a search?
    retrievedSnippets: boolean;     // Did we retrieve snippets?
  };
}
```

### Examples

#### Example 1: Simple Search

**Request:**

```json
{
  "name": "find",
  "arguments": {
    "query": "test runner",
    "source": "bun",
    "maxResults": 3
  }
}
```

**Response:**

```json
{
  "snippets": [],
  "hits": [
    {
      "alias": "bun",
      "lines": "304-324",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"],
      "snippet": "Bun includes a fast built-in test runner. Write tests with Jest-compatible APIs...",
      "score": 92.5,
      "sourceUrl": "https://bun.sh/llms.txt"
    },
    {
      "alias": "bun",
      "lines": "1234-1256",
      "headingPath": ["Bun Documentation", "API", "Test API"],
      "snippet": "The test() function defines a test case...",
      "score": 78.3,
      "sourceUrl": "https://bun.sh/llms.txt"
    }
  ],
  "executed": {
    "searched": true,
    "retrievedSnippets": false
  }
}
```

**CLI equivalent:**

```bash
blz search "test runner" --source bun --json
```

#### Example 2: Retrieve Snippet with Context

**Request:**

```json
{
  "name": "find",
  "arguments": {
    "snippets": ["bun:304-324"],
    "contextMode": "symmetric"
  }
}
```

**Response:**

```json
{
  "snippets": [
    {
      "alias": "bun",
      "lines": "304-350",
      "content": "# Test runner\n\nBun includes a fast built-in test runner...\n\n## Installation\n\nNo installation needed...",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"]
    }
  ],
  "hits": [],
  "executed": {
    "searched": false,
    "retrievedSnippets": true
  }
}
```

**CLI equivalent:**

```bash
blz find bun:304-324 --context block --json
```

#### Example 3: Search and Retrieve

**Request:**

```json
{
  "name": "find",
  "arguments": {
    "query": "test runner",
    "snippets": ["bun:304-324"],
    "source": "bun",
    "contextMode": "symmetric",
    "maxResults": 5
  }
}
```

**Response:**

```json
{
  "snippets": [
    {
      "alias": "bun",
      "lines": "304-350",
      "content": "# Test runner\n\nBun includes...",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"]
    }
  ],
  "hits": [
    {
      "alias": "bun",
      "lines": "304-324",
      "snippet": "Bun includes a fast built-in test runner...",
      "score": 92.5
    }
  ],
  "executed": {
    "searched": true,
    "retrievedSnippets": true
  }
}
```

#### Example 4: Multiple Ranges

**Request:**

```json
{
  "name": "find",
  "arguments": {
    "snippets": ["bun:100-120,130-150"],
    "linePadding": 2
  }
}
```

**Response:**

```json
{
  "snippets": [
    {
      "alias": "bun",
      "lines": "98-122",
      "content": "...\nFirst section content\n...",
      "headingPath": ["Bun Documentation", "Section 1"]
    },
    {
      "alias": "bun",
      "lines": "128-152",
      "content": "...\nSecond section content\n...",
      "headingPath": ["Bun Documentation", "Section 2"]
    }
  ],
  "hits": [],
  "executed": {
    "searched": false,
    "retrievedSnippets": true
  }
}
```

**CLI equivalent:**

```bash
blz find bun:100-120,130-150 -C 2 --json
```

### Error Handling

| Error Code | Reason | Example |
|------------|--------|---------|
| `-32000` | Source not found | `{"alias": "invalid"}` |
| `-32002` | Invalid citation format | `{"snippets": ["bun:abc-def"]}` |
| `-32010` | Index error | Corrupted index |
| `-32602` | Invalid params | `{"linePadding": 100}` (exceeds max) |

---

## `list-sources`

List installed documentation sources and registry candidates.

### Schema

```json
{
  "name": "list-sources",
  "description": "List docs",
  "inputSchema": {
    "type": "object",
    "properties": {
      "filter": {
        "type": "string",
        "description": "Filter sources by name (case-insensitive)"
      }
    }
  }
}
```

### Parameters

#### `filter` (string, optional)

Case-insensitive substring filter for source names.

**Examples:**

```javascript
{filter: "react"}     // Matches "react", "react-native", etc.
{filter: "bun"}       // Matches "bun", "bunyan", etc.
{}                    // No filter - returns all sources
```

### Response Format

```typescript
{
  sources: Array<{
    alias: string;
    title?: string;
    url: string;
    kind: "installed" | "registry";
    fetchedAt?: string;           // ISO 8601 timestamp
    suggestedCommand?: string;    // For registry sources
    metadata?: {
      totalLines: number;
      headings: number;
    };
  }>;
}
```

### Examples

#### Example 1: List All Sources

**Request:**

```json
{
  "name": "list-sources",
  "arguments": {}
}
```

**Response:**

```json
{
  "sources": [
    {
      "alias": "bun",
      "title": "Bun runtime docs",
      "url": "https://bun.sh/llms-full.txt",
      "kind": "installed",
      "fetchedAt": "2025-10-15T14:30:00Z",
      "metadata": {
        "totalLines": 42000,
        "headings": 156
      }
    },
    {
      "alias": "react",
      "url": "https://react.dev/llms-full.txt",
      "kind": "registry",
      "suggestedCommand": "blz add react"
    }
  ]
}
```

**CLI equivalent:**

```bash
blz list --json
```

#### Example 2: Filter Sources

**Request:**

```json
{
  "name": "list-sources",
  "arguments": {
    "filter": "react"
  }
}
```

**Response:**

```json
{
  "sources": [
    {
      "alias": "react",
      "url": "https://react.dev/llms-full.txt",
      "kind": "registry",
      "suggestedCommand": "blz add react"
    },
    {
      "alias": "react-native",
      "url": "https://reactnative.dev/llms.txt",
      "kind": "registry",
      "suggestedCommand": "blz add react-native"
    }
  ]
}
```

### Error Handling

This tool typically does not error. Returns empty array if no matches.

---

## `source-add`

Add documentation source from registry or custom URL.

### Schema

```json
{
  "name": "source-add",
  "description": "Add docs",
  "inputSchema": {
    "type": "object",
    "properties": {
      "alias": {
        "type": "string",
        "description": "Source identifier"
      },
      "url": {
        "type": "string",
        "description": "Custom URL (uses registry if omitted)"
      },
      "force": {
        "type": "boolean",
        "description": "Overwrite existing source"
      }
    },
    "required": ["alias"]
  }
}
```

### Parameters

#### `alias` (string, required)

Source identifier. Must be URL-safe (lowercase, alphanumeric, hyphens).

**Examples:**

```javascript
{alias: "bun"}
{alias: "react-native"}
{alias: "my-custom-docs"}
```

#### `url` (string, optional)

Custom documentation URL. If omitted, uses registry lookup.

**Examples:**

```javascript
// From registry
{alias: "bun"}

// Custom URL
{alias: "my-docs", url: "https://example.com/llms.txt"}
```

#### `force` (boolean, optional)

Overwrite existing source. Default: `false`.

**Examples:**

```javascript
// Add new source
{alias: "bun"}

// Force overwrite
{alias: "bun", force: true}
```

### Response Format

```typescript
{
  alias: string;
  url: string;
  message: string;    // Human-readable success message
}
```

### Examples

#### Example 1: Add from Registry

**Request:**

```json
{
  "name": "source-add",
  "arguments": {
    "alias": "astro"
  }
}
```

**Response:**

```json
{
  "alias": "astro",
  "url": "https://docs.astro.build/llms.txt",
  "message": "Added astro (2,451 headings, 18,732 lines) in 450ms"
}
```

**CLI equivalent:**

```bash
blz add astro
```

#### Example 2: Add Custom URL

**Request:**

```json
{
  "name": "source-add",
  "arguments": {
    "alias": "my-docs",
    "url": "https://example.com/docs/llms.txt"
  }
}
```

**Response:**

```json
{
  "alias": "my-docs",
  "url": "https://example.com/docs/llms.txt",
  "message": "Added my-docs (1,234 headings, 8,567 lines) in 320ms"
}
```

**CLI equivalent:**

```bash
blz add my-docs https://example.com/docs/llms.txt
```

#### Example 3: Force Overwrite

**Request:**

```json
{
  "name": "source-add",
  "arguments": {
    "alias": "bun",
    "force": true
  }
}
```

**Response:**

```json
{
  "alias": "bun",
  "url": "https://bun.sh/llms-full.txt",
  "message": "Updated bun (1,926 headings, 43,150 lines) in 890ms"
}
```

### Error Handling

| Error Code | Reason | Solution |
|------------|--------|----------|
| `-32001` | Source exists | Use `force: true` |
| `-32000` | Not in registry, no URL | Provide `url` parameter |
| `-32002` | Invalid URL format | Check URL syntax |
| `-32010` | Fetch/index failed | Check network, URL validity |

---

## `run-command`

Execute whitelisted read-only BLZ commands.

### Schema

```json
{
  "name": "run-command",
  "description": "Run safe cmd",
  "inputSchema": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string",
        "description": "Whitelisted command to execute"
      },
      "source": {
        "type": "string",
        "description": "Optional documentation alias for commands that operate on a source"
      }
    },
    "required": ["command"]
  }
}
```

### Parameters

#### `command` (string, required)

Whitelisted command name to execute.

**Whitelisted commands:**

- `stats` - Index statistics
- `history` - Update history
- `list` - List sources
- `validate` - Validate integrity
- `inspect` - Inspect metadata
- `schema` - JSON schema

#### `source` (string, optional)

Alias of the documentation source when the command operates on a specific
source (e.g., `history`, `validate`, `inspect`).

**Examples:**

```javascript
{command: "stats"}
{command: "history", source: "bun"}
{command: "validate", source: "bun"}
```

### Response Format

```typescript
{
  stdout: string;
  stderr: string;
  exitCode: number;
}
```

**Note:** Paths in output are sanitized (`$HOME` → `~`, absolute paths → `<project>`).

### Examples

#### Example 1: Get Statistics

**Request:**

```json
{
  "name": "run-command",
  "arguments": {
    "command": "stats"
  }
}
```

**Response:**

```json
{
  "stdout": "Total sources: 3\nTotal indexed lines: 87,523\nIndex size: 12.4 MB\nAverage search time: 6.2ms\n",
  "stderr": "",
  "exitCode": 0
}
```

**CLI equivalent:**

```bash
blz stats
```

#### Example 2: Check History

**Request:**

```json
{
  "name": "run-command",
  "arguments": {
    "command": "history",
    "source": "bun"
  }
}
```

**Response:**

```json
{
  "stdout": "bun update history:\n2025-10-15 14:30:00Z - Updated (890ms)\n2025-10-10 09:15:00Z - Added (1.2s)\n",
  "stderr": "",
  "exitCode": 0
}
```

**CLI equivalent:**

```bash
blz history bun
```

### Error Handling

| Error Code | Reason | Example |
|------------|--------|---------|
| `-32002` | Command not whitelisted | `{command: "remove"}` |
| `-32602` | Invalid arguments | `{command: ""}` (empty) |

**Note:** Write operations (add, update, remove) must use dedicated tools or CLI directly.

---

## `learn-blz`

Returns curated reference data about BLZ capabilities.

### Schema

```json
{
  "name": "learn-blz",
  "description": "Learn BLZ",
  "inputSchema": {
    "type": "object"
  }
}
```

### Parameters

No parameters required.

### Response Format

```typescript
{
  prompts: Array<{
    name: string;
    summary: string;
  }>;
  flags: Record<string, string[]>;
  examples: string[];
}
```

### Example

**Request:**

```json
{
  "name": "learn-blz",
  "arguments": {}
}
```

**Response:**

```json
{
  "prompts": [
    {
      "name": "discover-docs",
      "summary": "Find and add project docs"
    }
  ],
  "flags": {
    "contextMode": ["none", "symmetric", "all"],
    "source": ["bun", "react", "tanstack"]
  },
  "examples": [
    "find(query='test runner', source='bun')",
    "find(snippets=['bun:304-324'], contextMode='symmetric')",
    "list-sources(filter='react')",
    "source-add(alias='astro')"
  ]
}
```

**No CLI equivalent** - MCP-specific tool.

### Use Case

Agents can call `learn-blz` to understand:

- What prompts are available
- What values are valid for enum parameters
- Example usage patterns

---

## Common Patterns

### Pattern 1: Search → Retrieve

```javascript
// Step 1: Search
const search = await callTool("find", {
  query: "test runner",
  source: "bun",
  maxResults: 5
});

// Step 2: Pick best result
const citation = search.hits[0];
const ref = `${citation.alias}:${citation.lines}`;

// Step 3: Get full content
const content = await callTool("find", {
  snippets: [ref],
  contextMode: "symmetric"
});
```

### Pattern 2: Check Before Add

```javascript
// Step 1: Check if available
const sources = await callTool("list-sources", {
  filter: "astro"
});

// Step 2: Add if in registry
if (sources.sources.some(s => s.kind === "registry")) {
  await callTool("source-add", {alias: "astro"});
}
```

### Pattern 3: Multi-Source Search

```javascript
// Search across multiple sources by calling find per alias
const aliases = ["bun", "react", "next"];
const bySource = {};

for (const alias of aliases) {
  const results = await callTool("find", {
    query: "authentication",
    source: alias,
    maxResults: 5
  });

  if (results.hits) {
    bySource[alias] = results.hits;
  }
}
```

### Pattern 4: Incremental Context

```javascript
// Start with minimal context
let content = await callTool("find", {
  snippets: ["bun:304-324"],
  contextMode: "none"
});

// If not enough info, expand to section
if (needsMoreContext(content)) {
  content = await callTool("find", {
    snippets: ["bun:304-324"],
    contextMode: "symmetric"
  });
}
```

## Next Steps

- [README.md](README.md) - Overview and capabilities
- [SETUP.md](SETUP.md) - Client configuration
- [CLI Documentation](../cli/commands.md) - CLI equivalents
