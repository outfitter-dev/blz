# Check: Documentation

Instructions: $ARGUMENTS
- Invoke the `@agent-docs-checker` subagent to do a comprehensive check of the project documentation, user guides, etc.
- Provide the subagent with:
  - the instructions below
  - anything specific to the project or recent changes that you would like audited

Instructions to provide subagent:
- Go through all docs (`./docs/**`, `./README.md`, `./CHANGELOG.md`, etc.) and ensure they are up-to-date
- Make sure if there are links to other docs, those docs exist
- Use `blz ?<command> --help` to check documentation within the tool, and ensure the project docs are consistent with the tool's output
- Ensure all `bash` command blocks are correct and reflect the current output
