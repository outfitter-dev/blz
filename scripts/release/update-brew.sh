#!/usr/bin/env bash
set -euo pipefail

# Updates the Homebrew tap formula for blz.
#
# Required environment variables:
# - TAP_DIR: path to the checked out tap repo (default: homebrew-tap)
# - REPO: GitHub repo in owner/name form (e.g., outfitter-dev/blz)
# - VERSION: version string without leading v (e.g., 0.2.0)
# - SHA_ARM64: sha256 for blz-darwin-arm64.tar.gz
# - SHA_X64: sha256 for blz-darwin-x64.tar.gz

TAP_DIR=${TAP_DIR:-homebrew-tap}
REPO=${REPO:?REPO is required (e.g., outfitter-dev/blz)}
VERSION=${VERSION:?VERSION is required (e.g., 0.2.0)}
SHA_ARM64=${SHA_ARM64:?SHA_ARM64 is required}
SHA_X64=${SHA_X64:?SHA_X64 is required}

mkdir -p "$TAP_DIR/Formula"
FORMULA_PATH="$TAP_DIR/Formula/blz.rb"

cat > "$FORMULA_PATH" <<EOF
class Blz < Formula
  desc "Fast local search for llms.txt"
  homepage "https://blz.run"
  version "${VERSION}"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/${REPO}/releases/download/v\#{version}/blz-darwin-arm64.tar.gz"
      sha256 "${SHA_ARM64}"
    else
      url "https://github.com/${REPO}/releases/download/v\#{version}/blz-darwin-x64.tar.gz"
      sha256 "${SHA_X64}"
    end
  end

  def install
    bin.install "blz"
  end
end
EOF

echo "Updated formula at: $FORMULA_PATH"
