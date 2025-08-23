# Registry Lookup Feature

The `cache lookup` command allows users to search a built-in registry of popular documentation sources and easily add them to their cache.

## Usage

```bash
cache lookup <query>
```

## Examples

### Basic Search
```bash
$ cache lookup "claude"
Searching registries...
Found 3 matches:

1. Claude Code (claude-code)
   Anthropic's AI coding assistant documentation
   https://docs.anthropic.com/claude-code/llms.txt

2. Anthropic Claude API (anthropic)
   Claude API documentation and guides
   https://docs.anthropic.com/llms.txt

3. React (react)
   JavaScript library for building user interfaces
   https://react.dev/llms.txt

To add any of these sources, use:
  1. cache add claude-code https://docs.anthropic.com/claude-code/llms.txt
  2. cache add anthropic https://docs.anthropic.com/llms.txt
  3. cache add react https://react.dev/llms.txt
```

### JavaScript Ecosystem Search
```bash
$ cache lookup "javascript"
Searching registries...
Found 5 matches:

1. Node.js (node)
   JavaScript runtime built on Chrome's V8 JavaScript engine
   https://nodejs.org/docs/llms.txt

2. React (react)
   JavaScript library for building user interfaces
   https://react.dev/llms.txt

3. Bun (bun)
   Fast all-in-one JavaScript runtime and package manager
   https://bun.sh/docs/llms.txt

4. Deno (deno)
   Modern runtime for JavaScript and TypeScript
   https://docs.deno.com/llms.txt

5. Vue.js (vue)
   Progressive JavaScript framework for building UIs
   https://vuejs.org/llms.txt
```

### Interactive Mode (in a real terminal)
When run in an interactive terminal, the command provides arrow key navigation and interactive prompts:

```bash
$ cache lookup "claude"
Searching registries...
Found 2 matches:

> Select documentation to add (↑/↓ to navigate):
  1. Claude Code (claude-code) - Anthropic's AI coding assistant documentation  
  2. Anthropic Claude API (anthropic) - Claude API documentation and guides

> Enter alias [claude-code]: 
Adding claude-code from https://docs.anthropic.com/claude-code/llms.txt...
✓ Added claude-code (15 headings, 342 lines)
```

## Built-in Registry

The following documentation sources are currently available:

| Name | Slug | Aliases | Description |
|------|------|---------|-------------|
| Bun | bun | bun, bunjs | Fast all-in-one JavaScript runtime and package manager |
| Node.js | node | node, nodejs, js | JavaScript runtime built on Chrome's V8 JavaScript engine |
| Deno | deno | deno | Modern runtime for JavaScript and TypeScript |
| React | react | react, reactjs | JavaScript library for building user interfaces |
| Vue.js | vue | vue, vuejs | Progressive JavaScript framework for building UIs |
| Next.js | nextjs | nextjs, next | React framework for production with hybrid static & server rendering |
| Claude Code | claude-code | claude-code, claude | Anthropic's AI coding assistant documentation |
| Pydantic | pydantic | pydantic | Data validation library using Python type hints |
| Anthropic Claude API | anthropic | anthropic, claude-api | Claude API documentation and guides |
| OpenAI API | openai | openai, gpt | OpenAI API documentation and guides |

## Search Algorithm

The lookup uses fuzzy matching powered by the `SkimMatcherV2` algorithm, the same used by fuzzy finders like `skim` and `fzf`. It searches across:

1. **Name** (highest priority)
2. **Slug** (high priority)  
3. **Aliases** (high priority)
4. **Description** (lower priority - score is halved)

Results are ranked by relevance score and duplicate entries are filtered out.

## Non-Interactive Mode

When run in non-interactive environments (like CI/CD or when stdout is redirected), the command displays the search results and provides copy-pasteable `cache add` commands rather than prompting for user input.

## Adding New Registry Entries

The registry is currently hardcoded in `crates/cache-core/src/registry.rs`. To add new entries:

1. Add a new `RegistryEntry::new()` call to the `Registry::new()` method
2. Specify the name, slug (kebab-case), description, and llms.txt URL
3. Optionally add aliases using `.with_aliases(vec!["alias1", "alias2"])`

Future versions may support remote registries and user-defined registry entries.

## Features

- **Fuzzy Search**: Finds matches even with partial or misspelled queries
- **Interactive Selection**: Arrow key navigation in terminal environments  
- **Smart Defaults**: Suggests kebab-case slug as default alias
- **Validation**: Prevents reserved keywords from being used as aliases
- **Fallback Mode**: Works in both interactive and non-interactive environments
- **Integration**: Seamlessly integrates with existing `cache add` workflow

## Examples of Good Queries

- `cache lookup "react"` - Exact match
- `cache lookup "js"` - Alias match  
- `cache lookup "javascript runtime"` - Description match
- `cache lookup "anthropic"` - Company/organization match
- `cache lookup "api"` - Partial match across multiple entries
- `cache lookup "claude code"` - Multi-word match