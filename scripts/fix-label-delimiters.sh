#!/usr/bin/env bash
#
# Fix incorrectly named labels by removing backslashes
#

set -euo pipefail

echo "Fetching all labels with backslashes..."
labels_json=$(gh label list --limit 200 --json name,description,color)

fixed=0
errors=0

# Find all labels with backslashes
escaped_labels=$(echo "$labels_json" | jq -c '.[] | select(.name | contains("\\/"))')

if [[ -z "$escaped_labels" ]]; then
  echo "No labels with backslashes found!"
  exit 0
fi

echo "Found labels to fix:"
echo "$escaped_labels" | jq -r '.name' | sort
echo ""

while IFS= read -r label_json; do
  if [[ -z "$label_json" ]]; then
    continue
  fi

  wrong_name=$(echo "$label_json" | jq -r '.name')
  description=$(echo "$label_json" | jq -r '.description')
  color=$(echo "$label_json" | jq -r '.color')

  # Remove backslashes to get correct name
  correct_name="${wrong_name//\\/}"

  echo "Fixing: $wrong_name → $correct_name"

  # Create correctly named label
  if gh label create "$correct_name" \
    --description "$description" \
    --color "$color" \
    --force 2>/dev/null; then

    # Get all issues with the wrong label and update them
    echo "  Updating issues/PRs..."
    issue_numbers=$(gh issue list --label "$wrong_name" --state all --limit 1000 --json number --jq '.[].number' 2>/dev/null || echo "")

    if [[ -n "$issue_numbers" ]]; then
      count=0
      while IFS= read -r issue_num; do
        if [[ -n "$issue_num" ]]; then
          if gh issue edit "$issue_num" --add-label "$correct_name" --remove-label "$wrong_name" 2>/dev/null; then
            ((count++))
          fi
        fi
      done <<< "$issue_numbers"
      echo "  Updated $count issues/PRs"
    else
      echo "  No issues/PRs to update"
    fi

    # Delete wrong label
    echo "  Deleting incorrect label: $wrong_name"
    gh label delete "$wrong_name" --yes 2>/dev/null || echo "  Warning: Could not delete $wrong_name"

    ((fixed++))
  else
    echo "  Error: Failed to create $correct_name"
    ((errors++))
  fi
done <<< "$escaped_labels"

echo ""
echo "===== Summary ====="
echo "Fixed: $fixed labels"
echo "Errors: $errors labels"
