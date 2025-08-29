#!/usr/bin/env bash
set -euo pipefail

# branchwork.sh — Manage per-branch worklog
# Commands:
#   create   Create CURRENT symlink and branchwork doc from template
#   update   Append items/blocks or log an update entry
#   refresh  Refresh PR Stack Context from `gt log`
#   archive  Move CURRENT to timestamped archive in branchwork/

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
LOGS_DIR="${ROOT}/.agents/logs"
BRANCH_DIR="${LOGS_DIR}/branchwork"
CURRENT_LINK="${LOGS_DIR}/CURRENT.md"
TEMPLATE="${LOGS_DIR}/templates/BRANCHWORK.md"
GET_DATE="${ROOT}/.agents/scripts/get-date.sh"

# Optionally run markdownlint-cli2 --fix on a file
run_markdownlint() {
  local f="$1"
  (
    cd "$ROOT" >/dev/null 2>&1 || exit 0
    if command -v markdownlint-cli2 >/dev/null 2>&1; then
      markdownlint-cli2 --fix "$f" >/dev/null 2>&1 || true
    elif command -v npx >/dev/null 2>&1; then
      npx -y markdownlint-cli2 --fix "$f" >/dev/null 2>&1 || true
    fi
  )
}

usage() {
  cat << 'EOF'
Usage: branchwork <command> [options]

Commands:
  create            Create branchwork file and .agents/logs/CURRENT.md symlink
    Options: [--local] [--status <draft|in-review|changes-requested|approved>]

  update            Update CURRENT with structured content
    Options:
      --section <Heading>   Required for --item/--subitem/--code
      --item "text"         Append list item under section
      --subitem "text"      Append nested list item under last list
      --code <path|->       Append fenced code block under section (stdin if -)
      --lang <id>           Language id for code fence (optional)
      --log "summary"       Add an entry to Updates with timestamp and agent

  refresh           Refresh PR Stack Context using `gt log`

  archive           Move CURRENT to branchwork/YYYYMMDDHHmm-<branch>.md and remove symlink

Examples:
  ./.agents/scripts/branchwork.sh create --status draft
  ./.agents/scripts/branchwork.sh update --section "Merge Checklist" --item "Squash commits"
  ./.agents/scripts/branchwork.sh update --section "Decisions" --code ./decision.txt --lang markdown
  ./.agents/scripts/branchwork.sh update --log "Addressed review: fixed deadlock ordering"
  ./.agents/scripts/branchwork.sh refresh
  ./.agents/scripts/branchwork.sh archive
EOF
}

ensure_tools() {
  [[ -x "$GET_DATE" ]] || { echo "Missing $GET_DATE" >&2; exit 1; }
}

branch_name() { git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown"; }
slugify() { echo "$*" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'; }

pr_number() {
  local pr=""
  if command -v gh >/dev/null 2>&1; then
    pr=$(gh pr view --json number -q .number 2>/dev/null || echo "")
  fi
  if [[ -z "$pr" ]] && command -v gt >/dev/null 2>&1; then
    pr=$(gt log 2>/dev/null | sed -n '/(current)/,/^$/p' | sed -n 's/.*PR #\([0-9][0-9]*\).*/\1/p' | head -n1 || true)
  fi
  printf "%s" "$pr"
}

branch_stack_pos_total() {
  # Best-effort parse of gt log for position/total within current stack group
  if ! command -v gt >/dev/null 2>&1; then echo ","; return; fi
  local block; block=$(gt log 2>/dev/null | sed -n '/(current)/,/^$/p')
  local total; total=$(echo "$block" | grep -E '^\s*◯|^\s*◉' | wc -l | tr -d ' ')
  local idx; idx=$(echo "$block" | nl -ba | grep -n "current" | head -n1 | awk -F: '{print $1}' | sed 's/^\s*//')
  if [[ -n "$total" && "$total" -gt 0 && -n "$idx" ]]; then
    echo "${idx},${total}"
  else
    echo ","
  fi
}

create_cmd() {
  ensure_tools
  mkdir -p "$BRANCH_DIR"
  local USE_LOCAL=0 STATUS="draft"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --local) USE_LOCAL=1; shift;;
      --status) STATUS=${2:-draft}; shift 2;;
      *) echo "Unknown option: $1" >&2; exit 2;;
    esac
  done
  local BRANCH; BRANCH=$(branch_name)
  local BRANCH_SLUG; BRANCH_SLUG=$(slugify "$BRANCH")
  local TS; if [[ $USE_LOCAL -eq 1 ]]; then TS=$("$GET_DATE" --local); else TS=$("$GET_DATE"); fi
  local DATE_STR; if [[ $USE_LOCAL -eq 1 ]]; then DATE_STR="${TS:0:4}-${TS:4:2}-${TS:6:2} ${TS:8:2}:${TS:10:2}"; else DATE_STR="${TS:0:4}-${TS:4:2}-${TS:6:2} ${TS:8:2}:${TS:10:2} UTC"; fi
  local PR; PR=$(pr_number)
  local PT; PT=$(branch_stack_pos_total) || PT=","; local POS="${PT%,*}"; local TOT="${PT#*,}"
  local OUT="${BRANCH_DIR}/CURRENT-${BRANCH_SLUG}.md"

  # Fill template front matter
  awk -v date="$DATE_STR" -v pr="$PR" -v branch="$BRANCH" -v status="$STATUS" -v pos="$POS" -v tot="$TOT" -v slug="branchwork-${BRANCH_SLUG}" '
    BEGIN{front=1}
    /^---/ { print; next }
    front==1 && /^date:/ { print "date: " date; next }
    front==1 && /^slug:/ { print "slug: " slug; next }
    front==1 && /^status:/ { print "status: " status; next }
    front==1 && /^pr:/ { if (pr!="") print "pr: " pr; else print $0; next }
    front==1 && /^last_updated:/ { print "last_updated: " date; next }
    front==1 && /^branch:/ { print $0; getline; print "  name: " branch; getline; print "  base: main"; getline; if(pos!="") print "  position: " pos; else print "  position: "; getline; if(tot!="") print "  total: " tot; else print "  total: "; next }
    { print }
  ' "$TEMPLATE" > "$OUT"

  # Strip example section from template copy and remove stray horizontal rules after front matter
  # Strip example section from template copy
  sed -i '' -e '/^## Example/,$d' "$OUT" 2>/dev/null || sed -i -e '/^## Example/,$d' "$OUT"
  # Remove any '---' beyond the initial two front-matter separators
  awk 'BEGIN{d=0} { if ($0=="---") { d++; if (d>2) next } print }' "$OUT" > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"

  # Set title: PR title if available, else WIP from branch name
  local PR_TITLE="" PR_NUM_ONLY=""
  if command -v gh >/dev/null 2>&1; then
    PR_TITLE=$(gh pr view --json title -q .title 2>/dev/null || echo "")
    PR_NUM_ONLY=$(gh pr view --json number -q .number 2>/dev/null || echo "")
  fi
  if [[ -z "$PR_NUM_ONLY" ]] && command -v gt >/dev/null 2>&1; then
    PR_NUM_ONLY=$(gt log 2>/dev/null | sed -n '/(current)/,/^$/p' | sed -n 's/.*PR #\([0-9][0-9]*\).*/\1/p' | head -n1 || true)
  fi
  local NEW_TITLE
  if [[ -n "$PR_NUM_ONLY" ]]; then
    if [[ -z "$PR_TITLE" ]]; then PR_TITLE="PR #$PR_NUM_ONLY"; fi
    NEW_TITLE="# PR #${PR_NUM_ONLY}: ${PR_TITLE}"
  else
    # Preserve exact branch formatting, wrap in backticks
    NEW_TITLE="# [WIP] \`${BRANCH}\`"
  fi
  awk -v title="$NEW_TITLE" '
    BEGIN{done=0}
    NR==1 && $0 ~ /^# / { print title; done=1; next }
    $0 ~ /^# PR / && done==0 { print title; done=1; next }
    $0 ~ /^# \[WIP\]/ && done==0 { print title; done=1; next }
    { print }
  ' "$OUT" > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"

  # Create/refresh symlink
  ln -sfn "${OUT}" "${CURRENT_LINK}"
  echo "Created: $OUT"
  echo "Symlink: ${CURRENT_LINK} -> $(readlink ${CURRENT_LINK})"

  # Lint and fix formatting if tool available
  run_markdownlint "$OUT"
}

append_under_section() {
  local file="$1" heading="$2" content_line="$3"
  awk -v heading="$heading" -v block="$content_line" '
    BEGIN{found=0; skip_blank=0}
    $0==("## " heading){print; print block; found=1; skip_blank=1; next}
    { if (skip_blank==1 && $0 ~ /^\s*$/) { skip_blank=0; next } print }
    END{ if(found==0){ print "\n## " heading "\n\n" block } }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
}

append_file_under_section() {
  local file="$1" heading="$2" content_file="$3"
  awk -v heading="$heading" -v cfile="$content_file" '
    BEGIN{found=0; skip_blank=0}
    $0==("## " heading){print; while((getline line < cfile)>0) print line; close(cfile); found=1; skip_blank=1; next}
    { if (skip_blank==1 && $0 ~ /^\s*$/) { skip_blank=0; next } print }
    END{ if(found==0){ print "\n## " heading "\n"; while((getline line < cfile)>0) print line; close(cfile); } }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
}

update_cmd() {
  local FILE
  if [[ -L "$CURRENT_LINK" ]]; then FILE=$(readlink "$CURRENT_LINK"); else FILE="$CURRENT_LINK"; fi
  [[ -f "$FILE" ]] || { echo "CURRENT not found. Run branchwork create first." >&2; exit 1; }
  local SECTION="" ITEM="" SUBITEM="" CODE="" LANG="" LOG_MSG=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --section) SECTION=${2:-}; shift 2;;
      --item) ITEM=${2:-}; shift 2;;
      --subitem) SUBITEM=${2:-}; shift 2;;
      --code) CODE=${2:-}; shift 2;;
      --lang) LANG=${2:-}; shift 2;;
      --log) LOG_MSG=${2:-}; shift 2;;
      *) echo "Unknown option: $1" >&2; exit 2;;
    esac
  done
  if [[ -n "$LOG_MSG" ]]; then
    local TS; TS=$("$GET_DATE")
    local DATE_STR="${TS:0:4}-${TS:4:2}-${TS:6:2} ${TS:8:2}:${TS:10:2} UTC"
    local AGENT="@codex"
    # Insert update entry after '## Updates' heading
    awk -v d="$DATE_STR" -v a="$AGENT" -v msg="$LOG_MSG" '
      BEGIN{ins=0}
      /^## Updates/ {print; print ""; print "### " d ": [" a "] " msg;  ins=1; next}
      {print}
      END{ if(ins==0){ print "\n## Updates\n\n### " d ": [" a "] " msg "\n\n- ..." } }
    ' "$FILE" > "$FILE.tmp" && mv "$FILE.tmp" "$FILE"
    # Lint after logging an update
    run_markdownlint "$FILE"
    exit 0
  fi
  [[ -n "$SECTION" ]] || { echo "--section is required for item/subitem/code" >&2; exit 2; }
  if [[ -n "$ITEM" ]]; then
    append_under_section "$FILE" "$SECTION" "- $ITEM"
  elif [[ -n "$SUBITEM" ]]; then
    append_under_section "$FILE" "$SECTION" "  - $SUBITEM"
  elif [[ -n "$CODE" ]]; then
    local CONTENT_TMP; CONTENT_TMP=$(mktemp)
    if [[ "$CODE" == "-" ]]; then cat - > "$CONTENT_TMP"; else cat "$CODE" > "$CONTENT_TMP"; fi
    local FENCE="~~~"; local LANGF=""; if [[ -n "$LANG" ]]; then LANGF="$LANG"; fi
    {
      echo "${FENCE}${LANGF}";
      cat "$CONTENT_TMP";
      echo "${FENCE}";
    } > "${CONTENT_TMP}.wrapped"
    append_file_under_section "$FILE" "$SECTION" "${CONTENT_TMP}.wrapped"
    rm -f "$CONTENT_TMP" "${CONTENT_TMP}.wrapped"
  else
    echo "Nothing to update. Provide --item, --subitem, --code, or --log." >&2; exit 2
  fi

  # Lint after update
  run_markdownlint "$FILE"
}

refresh_cmd() {
  local FILE
  if [[ -L "$CURRENT_LINK" ]]; then FILE=$(readlink "$CURRENT_LINK"); else FILE="$CURRENT_LINK"; fi
  [[ -f "$FILE" ]] || { echo "CURRENT not found. Run branchwork create first." >&2; exit 1; }
  if ! command -v gt >/dev/null 2>&1; then echo "gt not found" >&2; exit 1; fi
  local LOGTMP; LOGTMP=$(mktemp)
  gt log > "$LOGTMP"
  # Always compact: filter volatile details to keep deterministic
  local CTMP; CTMP=$(mktemp)
  awk '
    # Drop relative times, blank gutters, PR details, URLs, version lines, commit SHAs
    /^[│[:space:]]+[0-9]+ (minute|hour|day)s? ago/ { next }
    /^[│[:space:]]+$/ { next }
    /^[│[:space:]]+PR #/ { next }
    /^[│[:space:]]+https:\/\// { next }
    /^[│[:space:]]+Last submitted version:/ { next }
    /^[│[:space:]]+[0-9a-f]{7} - / { next }
    { sub(/[[:space:]]+$/, ""); print }
  ' "$LOGTMP" |
  # Remove ephemeral markers like (current) or emojis; strip trailing spaces
  sed -E 's/ \(current\)//; s/ ❇️//g; s/[[:space:]]+$//' > "$CTMP"
  mv "$CTMP" "$LOGTMP"
  awk -v logtmp="$LOGTMP" '
    BEGIN{printed=0}
    /^## PR Stack Context/ {
      print; print ""; print "```text";
      while ((getline line < logtmp) > 0) { sub(/[[:space:]]+$/, "", line); print line; }
      close(logtmp); print "```"; print ""; printed=1;
      # Skip existing block until next heading, then ensure a blank line before it
      while (getline > 0) { if ($0 ~ /^## /) { print; break } }
      next
    }
    {print}
    END{
      if (printed==0) {
        print "\n## PR Stack Context\n"; print ""; print "```text";
        while ((getline line < logtmp) > 0) { sub(/[[:space:]]+$/, "", line); print line; }
        close(logtmp); print "```"; print "";
      }
    }
  ' "$FILE" > "$FILE.tmp" && mv "$FILE.tmp" "$FILE"
  rm -f "$LOGTMP"
  # Update title and last_updated
  local TS; TS=$("$GET_DATE")
  local DATE_STR="${TS:0:4}-${TS:4:2}-${TS:6:2} ${TS:8:2}:${TS:10:2} UTC"
  local PR_TITLE="" PR_NUM_ONLY=""
  if command -v gh >/dev/null 2>&1; then
    PR_TITLE=$(gh pr view --json title -q .title 2>/dev/null || echo "")
    PR_NUM_ONLY=$(gh pr view --json number -q .number 2>/dev/null || echo "")
  fi
  if [[ -z "$PR_NUM_ONLY" ]] && command -v gt >/dev/null 2>&1; then
    PR_NUM_ONLY=$(gt log 2>/dev/null | sed -n '/(current)/,/^$/p' | sed -n 's/.*PR #\([0-9][0-9]*\).*/\1/p' | head -n1 || true)
  fi
  local BR; BR=$(branch_name)
  local NEW_TITLE
  if [[ -n "$PR_NUM_ONLY" ]]; then
    if [[ -z "$PR_TITLE" ]]; then PR_TITLE="PR #$PR_NUM_ONLY"; fi
    NEW_TITLE="# PR #${PR_NUM_ONLY}: ${PR_TITLE}"
  else
    NEW_TITLE="# [WIP] \`${BR}\`"
  fi
  awk -v title="$NEW_TITLE" -v now="$DATE_STR" '
    BEGIN{done=0}
    NR==1 && $0 ~ /^# / { print title; next }
    $0 ~ /^# PR / && done==0 { print title; done=1; next }
    $0 ~ /^# \[WIP\]/ && done==0 { print title; done=1; next }
    /^last_updated:/ { print "last_updated: " now; next }
    { print }
  ' "$FILE" > "$FILE.tmp" && mv "$FILE.tmp" "$FILE"
  # Lint after refresh
  run_markdownlint "$FILE"
  echo "Refreshed PR Stack Context and title"
}

archive_cmd() {
  local FILE_TARGET; FILE_TARGET=$(readlink "$CURRENT_LINK" 2>/dev/null || true)
  if [[ -z "$FILE_TARGET" || ! -f "$FILE_TARGET" ]]; then echo "No CURRENT target to archive" >&2; exit 0; fi
  local BRANCH; BRANCH=$(branch_name); local SLUG; SLUG=$(slugify "$BRANCH")
  local TS; TS=$("$GET_DATE")
  local DST="${BRANCH_DIR}/${TS}-${SLUG}.md"
  mv "$FILE_TARGET" "$DST"
  rm -f "$CURRENT_LINK"
  echo "Archived to: $DST"
}

cmd=${1:-}
shift || true
case "$cmd" in
  create) create_cmd "$@" ;;
  update) update_cmd "$@" ;;
  log) update_cmd --log "$*" ;;
  refresh) refresh_cmd "$@" ;;
  archive) archive_cmd "$@" ;;
  -h|--help|help|"") usage ;;
  *) echo "Unknown command: $cmd" >&2; usage; exit 2 ;;
esac
