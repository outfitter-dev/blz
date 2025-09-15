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

# Validate SHA256 inputs to fail fast on bad data (each must be 64 hex chars)
for var in SHA_ARM64 SHA_X64; do
  val="${!var}"
  if [[ ! "$val" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "Invalid $var: must be 64 hex characters" >&2
    exit 1
  fi
done

mkdir -p "$TAP_DIR/Formula"
FORMULA_PATH="$TAP_DIR/Formula/blz.rb"

cat > "$FORMULA_PATH" <<EOF
class Blz < Formula
  desc "Fast local search for llms.txt"
  homepage "https://blz.run"
  license "Apache-2.0"
  version "${VERSION}"

  livecheck do
    url :stable
    strategy :github_latest
  end

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

  test do
    assert_match version.to_s, shell_output("#{bin}/blz --version")
  end
end
EOF

echo "Updated formula at: $FORMULA_PATH"
