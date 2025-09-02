#!/bin/bash
# Script to push draft branches without running hooks
# Usage: ./scripts/push-draft.sh [branch-name]

set -e

BRANCH=${1:-$(git rev-parse --abbrev-ref HEAD)}

echo "🚀 Pushing draft branch: $BRANCH (skipping hooks)"
git push --no-verify origin "$BRANCH"

echo "✅ Draft branch pushed successfully"
echo "💡 Remember to run 'make ci' before marking PR ready for review"