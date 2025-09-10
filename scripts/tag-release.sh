#!/usr/bin/env bash
set -euo pipefail

version="${1:-}"

if [[ -z "${version}" ]]; then
  echo "Usage: $0 vX.Y.Z" >&2
  exit 1
fi

if [[ ! "${version}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Error: version must look like vX.Y.Z (got: ${version})" >&2
  exit 1
fi

current_branch="$(git rev-parse --abbrev-ref HEAD)"
if [[ "${current_branch}" != "main" ]]; then
  echo "Error: please run from 'main' (current: ${current_branch})" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree not clean. Commit or stash changes first." >&2
  exit 1
fi

echo "Tagging ${version}…"
git tag -a "${version}" -m "Release ${version}"
git push origin "${version}"

echo "Tag ${version} pushed. Create a GitHub Release from this tag to trigger Homebrew bump."
echo "Note: crates.io publishing requires CARGO_REGISTRY_TOKEN to be set in CI or your environment."
