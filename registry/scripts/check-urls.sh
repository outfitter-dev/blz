#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../sources"

echo "Checking registry source URLs..."
echo ""

success=0
failed=0

for file in *.toml; do
  url=$(grep '^url = ' "$file" | cut -d'"' -f2)
  id=$(grep '^id = ' "$file" | cut -d'"' -f2)

  status=$(curl -sL -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "FAIL")

  printf "%-20s " "$id"

  if [ "$status" = "200" ]; then
    echo "✓ $status"
    ((success++))
  elif [ "$status" = "404" ]; then
    echo "✗ $status (NOT FOUND) - $url"
    ((failed++))
  elif [ "$status" = "FAIL" ]; then
    echo "✗ CONNECTION FAILED - $url"
    ((failed++))
  else
    echo "? $status - $url"
    ((failed++))
  fi
done

echo ""
echo "Summary: $success succeeded, $failed failed"

if [ $failed -gt 0 ]; then
  exit 1
fi