#!/usr/bin/env bash
set -euo pipefail

readonly TEST_REPO="outfitter-dev/blz"
readonly TEST_SHA_ARM64="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
readonly TEST_SHA_X64="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
readonly TEST_SHA_LINUX="cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
readonly TEST_SHA_LINUX_ARM64="dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR

assert_contains() {
  local file="$1"
  local expected="$2"
  grep -Fq "$expected" "$file" || {
    echo "Expected $file to contain: $expected" >&2
    exit 1
  }
}

assert_not_contains() {
  local file="$1"
  local unexpected="$2"
  if grep -Fq "$unexpected" "$file"; then
    echo "Expected $file not to contain: $unexpected" >&2
    exit 1
  fi
}

generate_formula() {
  local tap_dir="$1"
  local version="$2"
  local linux_arm64_sha="${3:-}"
  TAP_DIR="$tap_dir" REPO="$TEST_REPO" VERSION="$version" SHA_ARM64="$TEST_SHA_ARM64" SHA_X64="$TEST_SHA_X64" SHA_LINUX="$TEST_SHA_LINUX" SHA_LINUX_ARM64="$linux_arm64_sha" bash "$SCRIPT_DIR/update-brew.sh"
}

check_formula() {
  local formula="$1"
  local version="$2"
  ruby -c "$formula" >/dev/null
  ruby -e 'abort "Expected formula mode 0644" unless (File.stat(ARGV.fetch(0)).mode & 0777) == 0644' "$formula"
  assert_not_contains "$formula" 'version "'
  assert_not_contains "$formula" '#{version}'
  assert_contains "$formula" "releases/download/v${version}/blz-${version}-darwin-arm64.tar.gz"
  assert_contains "$formula" "releases/download/v${version}/blz-${version}-darwin-x64.tar.gz"
  assert_contains "$formula" "releases/download/v${version}/blz-${version}-linux-x64.tar.gz"
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

prerelease_tap="$tmp_dir/prerelease"
generate_formula "$prerelease_tap" "2.2.0-beta.1"
check_formula "$prerelease_tap/Formula/blz.rb" "2.2.0-beta.1"
assert_contains "$prerelease_tap/Formula/blz.rb" "The blz Linux arm64 binary is not available yet."

stable_tap="$tmp_dir/stable"
generate_formula "$stable_tap" "2.3.0" "$TEST_SHA_LINUX_ARM64"
check_formula "$stable_tap/Formula/blz.rb" "2.3.0"
assert_contains "$stable_tap/Formula/blz.rb" "releases/download/v2.3.0/blz-2.3.0-linux-arm64.tar.gz"
assert_not_contains "$stable_tap/Formula/blz.rb" "The blz Linux arm64 binary is not available yet."

echo "Homebrew formula generation checks passed."
