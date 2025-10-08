# Registry: Audit All Sources for Quality

Instructions: $ARGUMENTS
- Invoke the `@agent-registry-handler` subagent to audit all registry sources and identify index files that may have full documentation alternatives
- Provide the subagent with any specific focus areas from $ARGUMENTS (e.g., "focus on sources with <100 lines" or "check AI/LLM sources")

Instructions to provide subagent:
- Analyze all sources in the registry to identify potential index files:
  - Sources with "index" tag
  - Sources with "(Index)" in the name
  - Sources with contentType "index" or very low line counts (< 100 lines)

- For each identified index source:
  - Run `./registry/scripts/discover-links.sh <source-id>` to check for linked documentation
  - Note sources that have no .txt links (like Vercel, Redis)
  - Note sources that have discoverable full documentation alternatives
  - Document any 404s or broken links

- Provide a comprehensive audit report organized by priority:
  1. **High Priority**: Index sources with discoverable full documentation (ready to add)
  2. **Medium Priority**: Mixed content sources (500-1000 lines) that might benefit from alternatives
  3. **Low Priority**: Index sources with no .txt links (need manual investigation)
  4. **No Action**: Full sources (>1000 lines) that are already comprehensive

Follow-up steps after audit:
1. Present the audit report with statistics:
   - Total sources in registry: X
   - Index sources found: X
   - Discoverable alternatives: X
   - Sources needing manual review: X
   - Sources that are fine as-is: X

2. For each high-priority finding:
   - Show the discovered sources
   - Provide recommended `blz registry create-source` commands
   - Note estimated time to add all sources

3. For medium-priority findings:
   - List sources with justification for review
   - Suggest investigation approach

4. For low-priority findings:
   - List index sources with no alternatives found
   - Suggest next steps (manual inspection, GitHub issue, etc.)

5. Ask the user:
   - "Would you like me to proceed with adding the high-priority sources?"
   - "Should I investigate any of the medium-priority sources?"
   - "Would you like me to update index entries with proper tags and suffixes?"

Example usage:
- `/registry audit` - Audit all sources
- `/registry audit focus on AI sources` - Audit only AI/LLM category
- `/registry audit check for new .txt files` - Re-check known indexes
