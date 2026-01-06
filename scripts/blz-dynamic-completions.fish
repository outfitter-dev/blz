#!/usr/bin/env fish

# Dynamic completions for blz command
# This provides runtime completions for aliases and other dynamic values

# Complete aliases for commands that need them
function __fish_blz_complete_aliases
    # Get sources from blz list --format json and print canonical + metadata aliases
    blz list --json 2>/dev/null | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    if isinstance(data, list):
        seen = set()
        for s in data:
            if isinstance(s, dict):
                canon = s.get('alias') or s.get('source')
                if canon and canon not in seen:
                    print(canon)
                    seen.add(canon)
                for a in s.get('aliases', []) or []:
                    if isinstance(a, str) and a not in seen:
                        print(a)
                        seen.add(a)
            elif isinstance(s, str) and s not in seen:
                print(s)
                seen.add(s)
except Exception:
    pass
" 2>/dev/null
end

# Complete headings/anchors for a given alias
function __fish_blz_complete_anchors_for_alias
    set -l alias $argv[1]
    if test -z "$alias"
        return
    end
    blz toc $alias --json 2>/dev/null | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    if isinstance(data, list):
        seen = set()
        for e in data:
            if isinstance(e, dict):
                a = e.get('anchor')
                if isinstance(a, str) and a and a not in seen:
                    print(a)
                    seen.add(a)
except Exception:
    pass
" 2>/dev/null
end

# Extract alias token for `blz anchor get <alias> ...`
function __fish_blz_get_alias_for_anchor_get
    set -l tokens (commandline -opc)
    for i in (seq (count $tokens))
        if test $tokens[$i] = 'get'
            set -l j (math $i + 1)
            if test $j -le (count $tokens)
                echo $tokens[$j]
                return 0
            end
        end
    end
    return 1
end

function __fish_blz_have_anchor_alias
    set -l a (__fish_blz_get_alias_for_anchor_get)
    test -n "$a"
end

# Complete for search command
complete -c blz -n "__fish_seen_subcommand_from search" -l alias -xa "(__fish_blz_complete_aliases)"

# Complete for get command  
complete -c blz -n "__fish_seen_subcommand_from get" -xa "(__fish_blz_complete_aliases)"

# Complete for update command
complete -c blz -n "__fish_seen_subcommand_from refresh" -xa "(__fish_blz_complete_aliases)"
complete -c blz -n "__fish_seen_subcommand_from update" -xa "(__fish_blz_complete_aliases)"

# Complete for diff command
complete -c blz -n "__fish_seen_subcommand_from diff" -xa "(__fish_blz_complete_aliases)"

# Complete for remove command
complete -c blz -n "__fish_seen_subcommand_from remove" -xa "(__fish_blz_complete_aliases)"

# Complete for toc command (alias: anchors)
complete -c blz -n "__fish_seen_subcommand_from toc; or __fish_seen_subcommand_from anchors" -xa "(__fish_blz_complete_aliases)"

# Complete for anchor list|get (nested subcommands)
complete -c blz -n "__fish_seen_subcommand_from anchor; and __fish_seen_subcommand_from list" -xa "(__fish_blz_complete_aliases)"

# For `blz anchor get`, first complete alias, then anchors for the given alias
complete -c blz -n "__fish_seen_subcommand_from anchor; and __fish_seen_subcommand_from get; and not __fish_blz_have_anchor_alias" -xa "(__fish_blz_complete_aliases)"
complete -c blz -n "__fish_seen_subcommand_from anchor; and __fish_seen_subcommand_from get; and __fish_blz_have_anchor_alias" -xa "(__fish_blz_complete_anchors_for_alias (__fish_blz_get_alias_for_anchor_get))"

# Add descriptions for main commands
complete -c blz -n "__fish_blz_needs_command" -a add -d "Add a new llms.txt source"
complete -c blz -n "__fish_blz_needs_command" -a search -d "Search with 6ms latency!"
complete -c blz -n "__fish_blz_needs_command" -a get -d "Get exact line ranges"
complete -c blz -n "__fish_blz_needs_command" -a sources -d "List blzd sources"
complete -c blz -n "__fish_blz_needs_command" -a update -d "Update sources with ETag"
complete -c blz -n "__fish_blz_needs_command" -a diff -d "View changes"
complete -c blz -n "__fish_blz_needs_command" -a toc -d "Show table of contents for a source"
complete -c blz -n "__fish_blz_needs_command" -a completions -d "Generate shell completions"
