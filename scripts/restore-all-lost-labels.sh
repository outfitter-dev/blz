#!/usr/bin/env bash
#
# Restore labels that were removed during conversion
#

set -euo pipefail

echo "Restoring labels to issues that lost them..."
echo ""

# Manual list of issues and the labels they should have
# Based on the timeline API showing removed labels

# Format: "issue_number:label1,label2,label3"
declare -a RESTORE_MAP=(
  "157:area/quality"
  "116:area/ci-cd,area/rust,area/technical-debt"
  "99:area/core,area/tooling,area/workflow,source/agent"
  "97:area/ci-cd"
  "86:area/release-prep,status/tracking"
  "75:area/core,area/quality,source/internal"
  "74:area/agent-rules,source/internal"
  "73:area/agent-rules,source/internal"
  "72:area/core,source/internal"
  "71:source/internal"
  "70:source/internal"
  "69:area/quality,source/internal"
  "67:area/tooling,area/quality,source/internal"
  "51:area/ci-build,area/tooling,area/workflow"
  "49:area/agent-rules"
  "48:area/release-prep"
  "36:area/quality"
  "32:area/local-storage"
  "25:area/deployment"
  "24:area/release-prep"
  "280:area/workflow"
  "277:area/tooling"
)

restored_count=0
error_count=0

for entry in "${RESTORE_MAP[@]}"; do
  issue_num="${entry%%:*}"
  labels="${entry#*:}"

  echo "Issue #$issue_num:"

  IFS=',' read -ra LABELS <<< "$labels"
  for label in "${LABELS[@]}"; do
    echo "  Adding: $label"
    if gh issue edit "$issue_num" --add-label "$label" 2>/dev/null; then
      ((restored_count++))
    else
      echo "    Error: Could not add $label"
      ((error_count++))
    fi
  done
done

echo ""
echo "===== Summary ====="
echo "Restored: $restored_count labels"
echo "Errors: $error_count labels"
