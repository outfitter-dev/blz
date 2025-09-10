# Homebrew Installation

Install `blz` via Homebrew using our tap.

```bash
brew tap outfitter-dev/homebrew-tap
brew install outfitter-dev/tap/blz
```

The tap bump workflow triggers on GitHub Releases and opens a PR to `outfitter-dev/homebrew-tap` to bump the `blz` formula to the new version.

Requirements:

1. A Personal Access Token with access to `outfitter-dev/homebrew-tap` set as `HOMEBREW_TAP_TOKEN` in this repository’s secrets
2. The tap repository exists: https://github.com/outfitter-dev/homebrew-tap
3. Tag a release (e.g., `v0.1.0`) to trigger the bump

The first time, you may need to add an initial formula to the tap; subsequent versions are updated by the workflow.
