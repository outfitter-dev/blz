#!/usr/bin/env fish

# Auto-updating Fish completions for blz
# Add this to your config.fish:
#   source /path/to/blz-fish-init.fish

function __blz_check_completions --on-variable PATH --on-event fish_prompt
    # Only check occasionally to avoid overhead
    if test -z "$CACHE_COMPLETION_CHECK"
        set -g CACHE_COMPLETION_CHECK 0
    end
    
    set -g CACHE_COMPLETION_CHECK (math "$CACHE_COMPLETION_CHECK + 1")
    
    # Check every 50 prompts
    if test (math "$CACHE_COMPLETION_CHECK % 50") -ne 0
        return
    end
    
    set -l blz_bin (which blz 2>/dev/null)
    if test -z "$blz_bin"
        return
    end
    
    set -l completion_file "$HOME/.config/fish/completions/blz.fish"
    
    # Check if completions exist or if binary is newer
    if not test -f "$completion_file"; or test "$blz_bin" -nt "$completion_file"
        echo "🔄 Updating blz completions..."
        blz completions fish > "$completion_file"
        source "$completion_file"
    end
end

# Initial check
if which blz >/dev/null 2>&1
    set -l completion_file "$HOME/.config/fish/completions/blz.fish"
    if not test -f "$completion_file"
        blz completions fish > "$completion_file"
    end
end