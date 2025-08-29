#!/usr/bin/env bash
set -euo pipefail

# new-log.sh — Create a log from a template into .agents/logs/
# - Strips the example section (the trailing '---' separator and '## Example' and below)
# - Fills date (UTC by default, --local for local), branch, slug, and pr if applicable
# - Names the file: YYYYMMDDHHmm-[type]-<desc>.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
LOGS_DIR="${REPO_ROOT}/.agents/logs"
TEMPLATES_DIR="${LOGS_DIR}/templates"
GET_DATE="${REPO_ROOT}/.agents/scripts/get-date.sh"

usage() {
  cat << 'EOF'
Usage: new-log.sh --type <checkpoint|review|debug|feature|migration|recap|refactor> \
                  --desc "short description" [--pr <number>] [--linear <TICKET>] [--local]

Creates a new log file in .agents/logs/ using the corresponding template.

Behavior:
  - Filename: YYYYMMDDHHmm-[type]-<slug(desc)>.md
  - Front matter fields auto-filled when present in the template:
      date:     2025-08-29 19:30 UTC (or local with --local)
      branch:   current git branch (if template has 'branch:')
      slug:     <type>-<slug(desc)> (if template has 'slug:')
      pr:       #<number> [/ <TICKET>] if provided or auto-detected

Options:
  --type <t>     One of: checkpoint, review, debug, feature, migration, recap, refactor
  --desc <d>     Short description; used for filename slug and 'slug:' field when applicable
  --pr <n>       GitHub PR number (digits only); auto-detect attempted via gh/gt if omitted
  --linear <id>  Linear ticket id (e.g., BLZ-23) to append to pr field as "#123 / BLZ-23"
  --local        Use local time for 'date:' (default: UTC)
  -h, --help     Show this help

Examples:
  ./.agents/scripts/new-log.sh --type checkpoint --desc "agent docs reorg" --pr 61 --linear BLZ-23
  ./.agents/scripts/new-log.sh --type review --desc "pr-61-followups"
  ./.agents/scripts/new-log.sh --type debug --desc "indexer-deadlock" --local
EOF
}

TYPE=""
DESC=""
PR_NUM=""
LINEAR_ID=""
USE_LOCAL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --type)
      TYPE=${2:-}; shift 2 || { echo "--type requires a value" >&2; exit 2; };
      ;;
    --desc)
      DESC=${2:-}; shift 2 || { echo "--desc requires a value" >&2; exit 2; };
      ;;
    --pr)
      PR_NUM=${2:-}; shift 2 || { echo "--pr requires a number" >&2; exit 2; };
      ;;
    --linear)
      LINEAR_ID=${2:-}; shift 2 || { echo "--linear requires an id" >&2; exit 2; };
      ;;
    --local)
      USE_LOCAL=1; shift;
      ;;
    -h|--help)
      usage; exit 0;
      ;;
    *)
      echo "Unknown argument: $1" >&2; usage; exit 2;
      ;;
  esac
done

normalize_type() {
  case "$1" in
    checkpoint|review|debug|feature|migration|recap|refactor) echo "$1" ;;
    *) echo "" ;;
  esac
}

die() { echo "Error: $*" >&2; exit 1; }

TYPE=$(normalize_type "$TYPE")
[[ -n "$TYPE" ]] || die "--type must be one of: checkpoint, review, debug, feature, migration, recap, refactor"
[[ -n "$DESC" ]] || die "--desc is required"

[[ -x "$GET_DATE" ]] || die "Missing helper: $GET_DATE"
[[ -d "$TEMPLATES_DIR" ]] || die "Missing templates dir: $TEMPLATES_DIR"

# Map type -> template filename
TEMPLATE_NAME=$(echo "$TYPE" | tr '[:lower:]' '[:upper:]')
TEMPLATE_PATH="${TEMPLATES_DIR}/${TEMPLATE_NAME}.md"
[[ -f "$TEMPLATE_PATH" ]] || die "Template not found: $TEMPLATE_PATH"

# Timestamp (for filename) and date string (for front matter)
if [[ $USE_LOCAL -eq 1 ]]; then
  TS=$("$GET_DATE" --local)
  TS12="$TS"
  DATE_STR="${TS12:0:4}-${TS12:4:2}-${TS12:6:2} ${TS12:8:2}:${TS12:10:2}"
else
  TS=$("$GET_DATE")
  TS12="$TS"
  DATE_STR="${TS12:0:4}-${TS12:4:2}-${TS12:6:2} ${TS12:8:2}:${TS12:10:2} UTC"
fi

# Slugify description
slugify() { echo "$*" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'; }
SLUG_DESC=$(slugify "$DESC")

# Current branch
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

# Try to auto-detect PR if not provided
if [[ -z "$PR_NUM" ]]; then
  if command -v gh >/dev/null 2>&1; then
    PR_NUM=$(gh pr view --json number -q .number 2>/dev/null || echo "")
  fi
fi
if [[ -z "$PR_NUM" ]] && command -v gt >/dev/null 2>&1; then
  # Heuristic: search current branch block for 'PR #NN'
  PR_NUM=$(gt log 2>/dev/null | sed -n '/(current)/,/^$/p' | sed -n 's/.*PR #\([0-9][0-9]*\).*/\1/p' | head -n1 || true)
fi

PR_FIELD=""
if [[ -n "$PR_NUM" && -n "$LINEAR_ID" ]]; then
  PR_FIELD="#${PR_NUM} / ${LINEAR_ID}"
elif [[ -n "$PR_NUM" ]]; then
  PR_FIELD="#${PR_NUM}"
elif [[ -n "$LINEAR_ID" ]]; then
  PR_FIELD="$LINEAR_ID"
fi

OUT_FILE="${LOGS_DIR}/${TS}-${TYPE}-${SLUG_DESC}.md"

mkdir -p "$LOGS_DIR"

# Build the file: replace known front-matter fields if present; then strip example section
tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT

# Prepare sed expressions conditionally
SED_EXPR=(
  -e "s/^date:.*$/date: ${DATE_STR}/"
)
if grep -q '^branch:' "$TEMPLATE_PATH"; then
  SED_EXPR+=( -e "s/^branch:.*$/branch: ${BRANCH}/" )
fi
if grep -q '^slug:' "$TEMPLATE_PATH"; then
  SED_EXPR+=( -e "s/^slug:.*$/slug: ${TYPE}-${SLUG_DESC}/" )
fi
if grep -q '^pr:' "$TEMPLATE_PATH" && [[ -n "$PR_FIELD" ]]; then
  SED_EXPR+=( -e "s/^pr:.*$/pr: ${PR_FIELD}/" )
fi
if grep -q '^issue:' "$TEMPLATE_PATH" && [[ -n "$PR_FIELD" ]]; then
  # For DEBUG/FEATURE templates which use 'issue:'
  SED_EXPR+=( -e "s/^issue:.*$/issue: ${PR_FIELD}/" )
fi

sed "${SED_EXPR[@]}" "$TEMPLATE_PATH" \
  | sed '/^## Example/,$d' \
  | sed '${/^---$/d;}' > "$tmpfile"

mv "$tmpfile" "$OUT_FILE"
trap - EXIT

# Optionally run markdownlint-cli2 --fix on the created file
run_markdownlint() {
  local f="$1"
  (
    cd "$REPO_ROOT" >/dev/null 2>&1 || exit 0
    if command -v markdownlint-cli2 >/dev/null 2>&1; then
      markdownlint-cli2 --fix "$f" >/dev/null 2>&1 || true
    elif command -v npx >/dev/null 2>&1; then
      npx -y markdownlint-cli2 --fix "$f" >/dev/null 2>&1 || true
    fi
  )
}
run_markdownlint "${OUT_FILE}"

# If branchwork CURRENT exists, log this creation as an update entry
BRANCHWORK_SCRIPT="${REPO_ROOT}/.agents/scripts/branchwork.sh"
if [ -x "$BRANCHWORK_SCRIPT" ] && { [ -L "${REPO_ROOT}/.agents/logs/CURRENT.md" ] || [ -f "${REPO_ROOT}/.agents/logs/CURRENT.md" ]; }; then
  REL_LINK=".agents/logs/$(basename "${OUT_FILE}")"
  TYPE_UPPER=$(echo "$TYPE" | tr '[:lower:]' '[:upper:]')
  "$BRANCHWORK_SCRIPT" log "Created ${TYPE_UPPER} log: [$(basename "${OUT_FILE}")](${REL_LINK})" || true
fi

echo "Created: $OUT_FILE"
