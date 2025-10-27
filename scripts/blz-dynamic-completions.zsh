# Dynamic completions for blz (Zsh)
#
# Adds runtime alias completion (canonical + metadata aliases) on top of the
# generated `_blz` completions from clap. This augments, not replaces, the
# existing completion behavior by falling back to `_blz` when no dynamic
# completions are applicable.
#
# Usage: Source this file from your ~/.zshrc after installing the static
# completion file (_blz) into your fpath.
#   source /path/to/blz/scripts/blz-dynamic-completions.zsh

# Print aliases (canonical + metadata) as newline-separated list using JSON output.
__blz_aliases() {
  blz list --format json 2>/dev/null | python3 - <<'PY' 2>/dev/null
import json, sys
try:
  data = json.load(sys.stdin)
except Exception:
  sys.exit(0)
seen = set()
def emit(s):
  if isinstance(s, str) and s and s not in seen:
    print(s)
    seen.add(s)
if isinstance(data, list):
  for entry in data:
    if isinstance(entry, dict):
      canon = entry.get('alias') or entry.get('source')
      emit(canon)
      for a in (entry.get('aliases') or []):
        emit(a)
    elif isinstance(entry, str):
      emit(entry)
PY
}

# Print heading anchors for a given alias as newline-separated list by calling
# `blz toc <alias> --format json` and extracting the `anchor` field.
__blz_anchors_for_alias() {
  local alias="$1"
  if [[ -z "$alias" ]]; then
    return
  fi
  blz toc "$alias" --format json 2>/dev/null | python3 - <<'PY' 2>/dev/null
import json, sys
try:
  data = json.load(sys.stdin)
except Exception:
  sys.exit(0)
seen = set()
if isinstance(data, list):
  for entry in data:
    if isinstance(entry, dict):
      a = entry.get('anchor')
      if isinstance(a, str) and a and a not in seen:
        print(a)
        seen.add(a)
PY
}

# Dynamic completion dispatcher that augments the generated _blz
_blz_dynamic() {
  local cur prev sub ret=1
  cur=${words[CURRENT]}
  prev=${words[CURRENT-1]}
  sub=${words[2]}

  # search: complete for --alias/-s/--source value
  if [[ $sub == search ]]; then
    if [[ $prev == --alias || $prev == -s || $prev == --source ]]; then
      local -a aliases
      aliases=(${(f)$(__blz_aliases)})
      compadd -Q -a aliases && ret=0
    fi
  fi

  # positional alias for common subcommands
  if (( ret )); then
    case $sub in
      get|update|remove|toc|anchors)
        if (( CURRENT == 3 )); then
          local -a aliases
          aliases=(${(f)$(__blz_aliases)})
          compadd -Q -a aliases && ret=0
        fi
        ;;
      anchor)
        # Handle nested subcommands: `blz anchor list|get <alias> [<anchor>]`
        local sub2=${words[3]}
        case $sub2 in
          list)
            if (( CURRENT == 4 )); then
              local -a aliases
              aliases=(${(f)$(__blz_aliases)})
              compadd -Q -a aliases && ret=0
            fi
            ;;
          get)
            if (( CURRENT == 4 )); then
              local -a aliases
              aliases=(${(f)$(__blz_aliases)})
              compadd -Q -a aliases && ret=0
            elif (( CURRENT == 5 )); then
              local alias=${words[4]}
              local -a anchors
              anchors=(${(f)$(__blz_anchors_for_alias "$alias")})
              if (( ${#anchors[@]} )); then
                compadd -Q -a anchors && ret=0
              fi
            fi
            ;;
        esac
        ;;
    esac
  fi

  # Fallback to the generated _blz if no dynamic completions were added
  if (( ret )); then
    if whence -w _blz &>/dev/null; then
      _blz
      return
    fi
  fi
  return 0
}

# Register our dynamic completer for `blz`
compdef _blz_dynamic blz
