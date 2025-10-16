#!/usr/bin/env bash
#
# Convert label delimiters from : to /
# Keeps release:* and queue:* labels unchanged
#

set -euo pipefail

# Prefixes to convert
CONVERT_PREFIXES=(
  "area"
  "bug"
  "needs"
  "semver"
  "size"
  "source"
  "status"
)

# Prefixes to skip (keep with :)
SKIP_PREFIXES=(
  "release"
  "queue"
)

echo "Fetching all labels..."
labels_json=$(gh label list --limit 200 --json name,description,color)

converted=0
skipped=0
errors=0

for prefix in "${CONVERT_PREFIXES[@]}"; do
  echo ""
  echo "Processing ${prefix}:* labels..."

  # Find all labels with this prefix and : delimiter
  matching_labels=$(echo "$labels_json" | jq -r --arg prefix "${prefix}:" \
    '.[] | select(.name | startswith($prefix)) | @json')

  if [[ -z "$matching_labels" ]]; then
    echo "  No ${prefix}:* labels found"
    continue
  fi

  while IFS= read -r label_json; do
    if [[ -z "$label_json" ]]; then
      continue
    fi

    old_name=$(echo "$label_json" | jq -r '.name')
    description=$(echo "$label_json" | jq -r '.description')
    color=$(echo "$label_json" | jq -r '.color')

    # Convert : to /
    new_name="${old_name//:/\/}"

    echo "  Converting: $old_name → $new_name"

    # Create new label
    if gh label create "$new_name" \
      --description "$description" \
      --color "$color" \
      --force 2>/dev/null; then

      # Get all issues with the old label and update them
      echo "    Updating issues/PRs with $old_name..."
      issue_numbers=$(gh issue list --label "$old_name" --state all --limit 1000 --json number --jq '.[].number')

      if [[ -n "$issue_numbers" ]]; then
        count=0
        while IFS= read -r issue_num; do
          if [[ -n "$issue_num" ]]; then
            gh issue edit "$issue_num" --add-label "$new_name" --remove-label "$old_name" >/dev/null 2>&1 || true
            ((count++))
          fi
        done <<< "$issue_numbers"
        echo "    Updated $count issues/PRs"
      else
        echo "    No issues/PRs to update"
      fi

      # Delete old label
      echo "    Deleting old label: $old_name"
      gh label delete "$old_name" --yes >/dev/null 2>&1 || echo "    Warning: Could not delete $old_name"

      ((converted++))
    else
      echo "    Error: Failed to create $new_name"
      ((errors++))
    fi
  done <<< "$matching_labels"
done

echo ""
echo "===== Summary ====="
echo "Converted: $converted labels"
echo "Skipped: $skipped labels"
echo "Errors: $errors labels"
echo ""
echo "Kept unchanged: release:* and queue:* labels"
