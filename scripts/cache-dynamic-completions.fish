#!/usr/bin/env fish

# Dynamic completions for cache command
# This provides runtime completions for aliases and other dynamic values

# Complete aliases for commands that need them
function __fish_cache_complete_aliases
    # Get aliases from cache sources command
    cache sources --format json 2>/dev/null | python3 -c "
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
complete -c cache -n "__fish_seen_subcommand_from search" -l alias -xa "(__fish_cache_complete_aliases)"

# Complete for get command  
complete -c cache -n "__fish_seen_subcommand_from get" -xa "(__fish_cache_complete_aliases)"

# Complete for update command
complete -c cache -n "__fish_seen_subcommand_from update" -xa "(__fish_cache_complete_aliases)"

# Complete for diff command
complete -c cache -n "__fish_seen_subcommand_from diff" -xa "(__fish_cache_complete_aliases)"

# Add descriptions for main commands
complete -c cache -n "__fish_cache_needs_command" -a add -d "Add a new llms.txt source"
complete -c cache -n "__fish_cache_needs_command" -a search -d "Search with 6ms latency!"
complete -c cache -n "__fish_cache_needs_command" -a get -d "Get exact line ranges"
complete -c cache -n "__fish_cache_needs_command" -a sources -d "List cached sources"
complete -c cache -n "__fish_cache_needs_command" -a update -d "Update sources with ETag"
complete -c cache -n "__fish_cache_needs_command" -a diff -d "View changes"
complete -c cache -n "__fish_cache_needs_command" -a completions -d "Generate shell completions"