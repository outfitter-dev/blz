#!/usr/bin/env fish

# Auto-updating Fish completions for cache
# Add this to your config.fish:
#   source /path/to/cache-fish-init.fish

function __cache_check_completions --on-variable PATH --on-event fish_prompt
    # Only check occasionally to avoid overhead
    if test -z "$CACHE_COMPLETION_CHECK"
        set -g CACHE_COMPLETION_CHECK 0
    end
    
    set -g CACHE_COMPLETION_CHECK (math "$CACHE_COMPLETION_CHECK + 1")
    
    # Check every 50 prompts
    if test (math "$CACHE_COMPLETION_CHECK % 50") -ne 0
        return
    end
    
    set -l cache_bin (which cache 2>/dev/null)
    if test -z "$cache_bin"
        return
    end
    
    set -l completion_file "$HOME/.config/fish/completions/cache.fish"
    
    # Check if completions exist or if binary is newer
    if not test -f "$completion_file"; or test "$cache_bin" -nt "$completion_file"
        echo "🔄 Updating cache completions..."
        cache completions fish > "$completion_file"
        source "$completion_file"
    end
end

# Initial check
if which cache >/dev/null 2>&1
    set -l completion_file "$HOME/.config/fish/completions/cache.fish"
    if not test -f "$completion_file"
        cache completions fish > "$completion_file"
    end
end