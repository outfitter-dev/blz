#!/usr/bin/env bash
# Format docs helpers with markdownlint-cli2 + Prettier (for MDX/JSON/etc.)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

IFS=';' read -r -a MARKDOWN_PATTERNS <<< "${FMT_DOCS_MARKDOWN:-AGENTS.md;README.md;docs/**/*.md;.agents/**/*.md}"
IFS=';' read -r -a PRETTIER_PATTERNS <<< "${FMT_DOCS_PRETTIER:-docs/**/*.{mdx,json,ts,tsx,js,jsx,css,scss,yml,yaml};*.{mdx,json,yml,yaml}}"

run_with_fallback() {
  local binary="$1"; shift
  if command -v "$binary" >/dev/null 2>&1; then
    "$binary" "$@"
    return 0
  fi
  return 1
}

run_markdownlint() {
  if [ "${MARKDOWN_PATTERNS[*]}" = "" ]; then
    return 0
  fi
  if run_with_fallback markdownlint-cli2 --fix "${MARKDOWN_PATTERNS[@]}"; then
    return 0
  fi
  if run_with_fallback npx --yes markdownlint-cli2 --fix "${MARKDOWN_PATTERNS[@]}"; then
    return 0
  fi
  if run_with_fallback bunx markdownlint-cli2 --fix "${MARKDOWN_PATTERNS[@]}"; then
    return 0
  fi
  echo "markdownlint-cli2 unavailable; skipping markdown lint." >&2
}

run_prettier() {
  if [ "${PRETTIER_PATTERNS[*]}" = "" ]; then
    return 0
  fi
  if run_with_fallback prettier --write "${PRETTIER_PATTERNS[@]}"; then
    return 0
  fi
  if run_with_fallback npx --yes prettier --write "${PRETTIER_PATTERNS[@]}"; then
    return 0
  fi
  if run_with_fallback bunx prettier --write "${PRETTIER_PATTERNS[@]}"; then
    return 0
  fi
  echo "prettier unavailable; skipping Prettier formatting." >&2
}

run_markdownlint
run_prettier
