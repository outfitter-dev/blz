#!/usr/bin/env fish

# Dynamic completions for blz command
# This provides runtime completions for aliases and other dynamic values

# Complete aliases for commands that need them
function __fish_blz_complete_aliases
    # Get sources from blz list --output json and print canonical + metadata aliases
    blz list --output json 2>/dev/null | python3 -c "
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

# Complete for search command
complete -c blz -n "__fish_seen_subcommand_from search" -l alias -xa "(__fish_blz_complete_aliases)"

# Complete for get command  
complete -c blz -n "__fish_seen_subcommand_from get" -xa "(__fish_blz_complete_aliases)"

# Complete for update command
complete -c blz -n "__fish_seen_subcommand_from update" -xa "(__fish_blz_complete_aliases)"

# Complete for diff command
complete -c blz -n "__fish_seen_subcommand_from diff" -xa "(__fish_blz_complete_aliases)"

# Complete for remove command
complete -c blz -n "__fish_seen_subcommand_from remove" -xa "(__fish_blz_complete_aliases)"

# Complete for anchors command
complete -c blz -n "__fish_seen_subcommand_from anchors" -xa "(__fish_blz_complete_aliases)"

# Add descriptions for main commands
complete -c blz -n "__fish_blz_needs_command" -a add -d "Add a new llms.txt source"
complete -c blz -n "__fish_blz_needs_command" -a search -d "Search with 6ms latency!"
complete -c blz -n "__fish_blz_needs_command" -a get -d "Get exact line ranges"
complete -c blz -n "__fish_blz_needs_command" -a sources -d "List blzd sources"
complete -c blz -n "__fish_blz_needs_command" -a update -d "Update sources with ETag"
complete -c blz -n "__fish_blz_needs_command" -a diff -d "View changes"
complete -c blz -n "__fish_blz_needs_command" -a completions -d "Generate shell completions"
