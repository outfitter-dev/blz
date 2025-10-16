#!/usr/bin/env bash
#
# Restore labels to issues that lost them during conversion
#

set -euo pipefail

# Old label prefixes that were converted
PREFIXES=("area" "bug" "needs" "semver" "size" "source" "status")

echo "Scanning issues and PRs for lost labels..."
echo ""

restored=0
checked=0

# Get all issues and PRs (including closed ones)
all_issues=$(gh issue list --state all --limit 500 --json number --jq '.[].number')

for issue_num in $all_issues; do
  ((checked++))

  # Get recent label removals (last 2 hours)
  cutoff_time=$(date -u -v-2H '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date -u -d '2 hours ago' '+%Y-%m-%dT%H:%M:%SZ')

  removed_labels=$(gh api "repos/outfitter-dev/blz/issues/$issue_num/timeline" --jq \
    --arg cutoff "$cutoff_time" \
    '[.[] | select(.event == "unlabeled" and .created_at > $cutoff) | .label.name] | unique | .[]' 2>/dev/null || echo "")

  if [[ -z "$removed_labels" ]]; then
    continue
  fi

  # Check if any removed labels match our conversion patterns
  labels_to_restore=""
  for label in $removed_labels; do
    # Check if it matches one of our prefixes with :
    for prefix in "${PREFIXES[@]}"; do
      if [[ "$label" == "${prefix}:"* ]]; then
        # Convert to new format (replace : with /)
        new_label="${label//:/\/}"
        labels_to_restore="$labels_to_restore $new_label"
        break
      fi
    done
  done

  if [[ -n "$labels_to_restore" ]]; then
    echo "Issue #$issue_num: Restoring labels:$labels_to_restore"
    for label in $labels_to_restore; do
      gh issue edit "$issue_num" --add-label "$label" 2>/dev/null || echo "  Warning: Could not add $label to #$issue_num"
    done
    ((restored++))
  fi

  # Progress indicator
  if ((checked % 50 == 0)); then
    echo "  ... checked $checked issues ..."
  fi
done

echo ""
echo "===== Summary ====="
echo "Checked: $checked issues/PRs"
echo "Restored: $restored issues/PRs"
