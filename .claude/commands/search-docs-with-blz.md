# Search documentation with `blz`

Instructions: $ARGUMENTS

## Context

- `blz` is a CLI tool that can search documentation quickly and efficiently. It stores llms.txt and llms-full.txt files locally, and indexes them, for line-accurate search.

## Workflow

1. Install `blz` if "current version" reports it's not installed:
   ```bash
   # Install from source
   curl -fsSL https://blz.run/install.sh | sh

   # Check version
   blz --version
   ```
2. Consider the `Instructions` request for documentation. Ultrathink on what is being asked, and what kinds of information is needed to answer it.
3. Invoke the `@docs-trailblazer` subagent and provide them with:
   - Current `blz` version: !`blz --version`
   - Sources status: !`blz list --status --json`
   - The information being requested
   - A list of likely relevant sources, or tool names
   - Possible search terms related to the information being requested
4. Let the subagent run the search and consider its results.
5. Follow up with your own `blz` searches based on its feedback if necessary.
