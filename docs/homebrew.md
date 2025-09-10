# Homebrew Installation

Install `blz` via Homebrew using our tap.

## Install

```bash
brew tap outfitter-dev/homebrew-tap
brew install outfitter-dev/tap/blz
```

To upgrade:

```bash
brew upgrade outfitter-dev/tap/blz
```

Verify:

```bash
blz --version
```

## How the tap is updated

On every tagged GitHub Release (e.g., `v0.1.0`), a GitHub Actions workflow in this repo
opens a PR to `outfitter-dev/homebrew-tap` to bump the `blz` formula to the new version.

Requirements:

- A Personal Access Token with access to `outfitter-dev/homebrew-tap` set as
  `HOMEBREW_TAP_TOKEN` in repo secrets
- The tap repository exists: https://github.com/outfitter-dev/homebrew-tap

Workflow:

1. Create a new release tag (e.g., `v0.1.0`)
2. The `homebrew-tap.yml` workflow runs and bumps the formula
3. A PR is opened on the tap; merge it to publish

If you need to re-run the bump manually, you can trigger the workflow using
the "Run workflow" button on GitHub under the Actions tab.
