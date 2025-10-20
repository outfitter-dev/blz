---
description: "Search, retrieve, and manage documentation with BLZ"
argument-hint: "<query> | add <source> <url> | list | get <citation> | refresh"
---

# BLZ Documentation Search

**Request**: $ARGUMENTS

Invoke the `@blz:trailblazer` agent with this request. The agent handles all BLZ operations:

- **Search**: `/blz "test runner"` or `/blz how do I write tests in Bun`
- **Add source**: `/blz add bun https://bun.sh/llms-full.txt`
- **List sources**: `/blz list`
- **Retrieve content**: `/blz get bun:304-324`
- **Refresh sources**: `/blz refresh` or `/blz refresh bun`
- **Complex research**: `/blz Compare React hooks vs Vue composition API`

The agent will interpret the request and execute the appropriate blz operations.
