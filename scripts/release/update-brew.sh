#!/usr/bin/env bash
set -euo pipefail

# Updates the Homebrew tap formula for blz.
#
# Required environment variables:
# - TAP_DIR: path to the checked out tap repo (default: homebrew-tap)
# - REPO: GitHub repo in owner/name form (e.g., outfitter-dev/blz)
# - VERSION: version string without leading v (e.g., 0.2.0)
# - SHA_ARM64: sha256 for blz-${VERSION}-darwin-arm64.tar.gz
# - SHA_X64: sha256 for blz-${VERSION}-darwin-x64.tar.gz

TAP_DIR=${TAP_DIR:-homebrew-tap}
REPO=${REPO:?REPO is required (e.g., outfitter-dev/blz)}
VERSION=${VERSION:?VERSION is required (e.g., 0.2.0)}
if [[ "$VERSION" =~ ^v ]]; then
  echo "VERSION must not start with 'v' (got: $VERSION)" >&2
  exit 1
fi
if [[ ! "$VERSION" =~ ^[0-9]+(\.[0-9]+){1,2}([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "VERSION must look like 0.2.0 or 0.2.0-beta.1 (got: $VERSION)" >&2
  exit 1
fi
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
TMP_FORMULA="$(mktemp)"
trap 'rm -f "$TMP_FORMULA"' EXIT

cat > "$TMP_FORMULA" <<EOF
class Blz < Formula
  desc "Fast local search for llms.txt"
  homepage "https://blz.run"
  license "Apache-2.0"
  version "${VERSION}"

  livecheck do
    url "https://github.com/${REPO}/releases/latest"
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/blz-#{version}-darwin-arm64.tar.gz"
      sha256 "${SHA_ARM64}"
    end
    on_intel do
      url "https://github.com/${REPO}/releases/download/v#{version}/blz-#{version}-darwin-x64.tar.gz"
      sha256 "${SHA_X64}"
    end
  end

  def install
    bin.install "blz"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/blz --version")
    assert_match "blz", shell_output("#{bin}/blz --help")
  end
end
EOF
mv -f "$TMP_FORMULA" "$FORMULA_PATH"
trap - EXIT
echo "Updated formula at: $FORMULA_PATH"
