#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "This helper requires GitHub CLI ('gh'). Install from https://cli.github.com/" >&2
  exit 1
fi

repo="${1:-outfitter-dev/blz}"

echo "Repository: ${repo}"
echo

echo "Open bugs:" && echo "------------"
gh issue list --repo "${repo}" --label bug --state open --limit 50 || true
echo

echo "Open enhancements:" && echo "-------------------"
gh issue list --repo "${repo}" --label enhancement --state open --limit 50 || true
echo

echo "Unlabeled issues:" && echo "------------------"
gh issue list --repo "${repo}" --state open --limit 50 --label 'no:bug,enhancement,release' || true
echo

echo "Recent PRs:" && echo "-----------"
gh pr list --repo "${repo}" --state open --limit 20 || true
