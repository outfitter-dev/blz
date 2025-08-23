#!/usr/bin/env fish

# Dynamic completions for blz command
# This provides runtime completions for aliases and other dynamic values

# Complete aliases for commands that need them
function __fish_blz_complete_aliases
    # Get aliases from blz sources command
    blz sources --format json 2>/dev/null | python3 -c "
import json, sys
try:
    sources = json.load(sys.stdin)
    for s in sources:
        print(s)
except:
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

# Add descriptions for main commands
complete -c blz -n "__fish_blz_needs_command" -a add -d "Add a new llms.txt source"
complete -c blz -n "__fish_blz_needs_command" -a search -d "Search with 6ms latency!"
complete -c blz -n "__fish_blz_needs_command" -a get -d "Get exact line ranges"
complete -c blz -n "__fish_blz_needs_command" -a sources -d "List blzd sources"
complete -c blz -n "__fish_blz_needs_command" -a update -d "Update sources with ETag"
complete -c blz -n "__fish_blz_needs_command" -a diff -d "View changes"
complete -c blz -n "__fish_blz_needs_command" -a completions -d "Generate shell completions"