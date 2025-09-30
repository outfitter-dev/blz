# Registry: Discover Full Documentation Sources

Instructions: $ARGUMENTS
- Invoke the `@agent-registry-handler` subagent to discover full documentation sources from index files
- Provide the subagent with:
  - The source ID or URL to analyze (from $ARGUMENTS)
  - Any specific instructions about what to look for or focus on
  - Context about recent changes or known issues with the source

Instructions to provide subagent:
- **Resource**: Use https://llmstxthub.com/ as a reference to discover additional llms.txt sources or verify known sources
  - Search the hub for the project name to find official llms.txt URLs
  - Check if the hub lists any additional documentation files beyond the main index
  - Cross-reference discovered URLs with the hub's listings

- If given a source ID (e.g., "redis"):
  - Run `./registry/scripts/discover-links.sh <source-id>` to automate initial discovery
  - Analyze the output and identify all discovered .txt links
  - Test each link with `blz add --dry-run` to classify content type
  - Cross-check with https://llmstxthub.com/ for any additional sources
  - Report findings with recommendations for which sources to add

- If given a URL:
  - First run `blz add temp-<name> <url> --dry-run --quiet` to analyze content
  - If contentType is "index" (< 100 lines), proceed with discovery workflow
  - Extract all .txt references from the index file
  - Resolve relative URLs to absolute URLs
  - Cross-check with https://llmstxthub.com/ to find additional sources not listed in the index
  - Test each discovered URL with dry-run analysis
  - Report findings with actionable recommendations

- For each good candidate (contentType: "full", lines > 1000):
  - Provide a recommended `blz registry create-source` command
  - Include suggested metadata (description, category, tags, aliases)
  - Note any special considerations (redirects, format variations, etc.)

Follow-up steps after discovery:
1. Present a summary report showing:
   - Total sources discovered
   - Good candidates (recommend adding)
   - Mixed content (needs review)
   - Indexes (skip)
   - Any issues or edge cases encountered

2. For each good candidate, show the complete command:
   ```bash
   blz registry create-source <name> \
     --url <url> \
     --description "<description>" \
     --category <category> \
     --tags <tags> \
     --npm <packages> \
     --github <repos> \
     --yes
   ```

3. If the original source is an index that should be kept:
   - Note that it needs "(Index)" suffix in name
   - Note that it needs "index" tag added
   - Provide edit instructions or offer to update it

4. After user confirms which sources to add:
   - Execute the `blz registry create-source` commands
   - Update the original index entry (if keeping it)
   - Run `./registry/scripts/build.sh` to rebuild registry
   - Verify all new sources are in `registry.json`

5. Ask the user:
   - "Would you like me to proceed with adding these sources to the registry?"
   - "Should I keep the original index entry or remove it?"
   - "Are there any other sources you'd like me to analyze?"

Example usage:
- `/registry discover redis` - Discover from existing registry source
- `/registry discover https://anthropic.com/llms.txt` - Discover from URL
- `/registry discover check llmstxthub.com for new sources` - Browse the hub for new projects to add
