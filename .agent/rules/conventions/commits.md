# Commit Rules

- This repo uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages
- Conventional commits are enforced by [commitlint](https://commitlint.js.org/)
- Include `(scope)` in the commit message if the change is related to a specific part of the codebase
- Use all lowercase for commit messages

  ```text
  feat(scope): add new feature
  fix(api): resolve timeout issue
  docs: update readme
  ```

## Commit Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to the build process or auxiliary tools and libraries such as documentation generation

## Scope Examples for Cache Project

- **core**: Core cache functionality
- **cli**: CLI interface changes
- **mcp**: MCP server implementation
- **index**: Search index related changes
- **storage**: Storage backend changes
- **config**: Configuration management
- **parser**: Parser improvements
- **fetcher**: Fetcher module changes
