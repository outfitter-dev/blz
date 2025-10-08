#!/usr/bin/env bash
# Hydrate blz-dev from existing blz installation.
#
# Copies configuration, sources, and indices from your production blz setup
# to the isolated blz-dev directories for testing.
#
# Usage:
#   ./hydrate-dev.sh [options]
#
# Options:
#   --force           Overwrite existing blz-dev data
#   --config-only     Copy only config files (not source data)
#   --sources-only    Copy only source data (not config)
#   --dry-run         Show what would be copied without copying
#   --help            Show this help message

set -euo pipefail

# Colors for output
if [[ -t 1 ]]; then
    BOLD=$(tput bold)
    GREEN=$(tput setaf 2)
    YELLOW=$(tput setaf 3)
    RED=$(tput setaf 1)
    RESET=$(tput sgr0)
else
    BOLD="" GREEN="" YELLOW="" RED="" RESET=""
fi

# Default options
FORCE=false
CONFIG_ONLY=false
SOURCES_ONLY=false
DRY_RUN=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --force)
            FORCE=true
            shift
            ;;
        --config-only)
            CONFIG_ONLY=true
            shift
            ;;
        --sources-only)
            SOURCES_ONLY=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            sed -n '2,/^$/p' "$0" | grep '^#' | cut -c 3-
            exit 0
            ;;
        *)
            echo "${RED}Unknown option: $1${RESET}" >&2
            echo "Use --help for usage information" >&2
            exit 1
            ;;
    esac
done

# Detect platform and set paths
detect_paths() {
    local profile=$1
    local config_dir=""
    local data_dir=""

    if [[ "$OSTYPE" != "darwin"* && "$OSTYPE" != "linux-gnu"* ]]; then
        echo "${RED}Unsupported platform: $OSTYPE${RESET}" >&2
        exit 1
    fi

    local xdg_config="${XDG_CONFIG_HOME:-}"
    local xdg_data="${XDG_DATA_HOME:-}"

    if [[ -n "$xdg_config" ]]; then
        config_dir="$xdg_config/$profile"
    else
        # Fallback mirrors blz-core profile logic
        if [[ "$profile" == "blz-dev" ]]; then
            config_dir="$HOME/.blz-dev"
        else
            config_dir="$HOME/.blz"
        fi
    fi

    if [[ -n "$xdg_data" ]]; then
        data_dir="$xdg_data/$profile"
    else
        if [[ "$profile" == "blz-dev" ]]; then
            data_dir="$HOME/.blz-dev"
        else
            data_dir="$HOME/.blz"
        fi
    fi

    # For production blz, detect legacy dot-directory even if XDG is configured but unused
    if [[ "$profile" == "blz" ]] && [[ ! -d "$config_dir" ]] && [[ -d "$HOME/.blz" ]]; then
        config_dir="$HOME/.blz"
    fi
    if [[ "$profile" == "blz" ]] && [[ ! -d "$data_dir" ]] && [[ -d "$HOME/.blz" ]]; then
        data_dir="$HOME/.blz"
    fi

    echo "$config_dir:$data_dir"
}

# Get source and destination paths
SRC_PATHS=$(detect_paths "blz")
SRC_CONFIG=$(echo "$SRC_PATHS" | cut -d: -f1)
SRC_DATA=$(echo "$SRC_PATHS" | cut -d: -f2)

DEST_PATHS=$(detect_paths "blz-dev")
DEST_CONFIG=$(echo "$DEST_PATHS" | cut -d: -f1)
DEST_DATA=$(echo "$DEST_PATHS" | cut -d: -f2)

echo "${BOLD}BLZ Development Hydration${RESET}"
echo ""
echo "Source (blz):"
echo "  Config: ${GREEN}$SRC_CONFIG${RESET}"
echo "  Data:   ${GREEN}$SRC_DATA${RESET}"
echo ""
echo "Destination (blz-dev):"
echo "  Config: ${YELLOW}$DEST_CONFIG${RESET}"
echo "  Data:   ${YELLOW}$DEST_DATA${RESET}"
echo ""

# Validate source directories exist
if [[ ! -d "$SRC_CONFIG" && ! -d "$SRC_DATA" ]]; then
    echo "${RED}Error: No blz installation found${RESET}" >&2
    echo "Expected config at: $SRC_CONFIG" >&2
    echo "Expected data at: $SRC_DATA" >&2
    exit 1
fi

if [[ ! -d "$SRC_CONFIG" ]]; then
    echo "${YELLOW}Warning: Config directory not found at $SRC_CONFIG${RESET}" >&2
    SRC_CONFIG=""
fi

if [[ ! -d "$SRC_DATA" ]]; then
    echo "${YELLOW}Warning: Data directory not found at $SRC_DATA${RESET}" >&2
    SRC_DATA=""
fi

# Check if destination already has data
DEST_HAS_DATA=false
if [[ -d "$DEST_CONFIG" ]] && [[ -n "$(ls -A "$DEST_CONFIG" 2>/dev/null)" ]]; then
    DEST_HAS_DATA=true
fi
if [[ -d "$DEST_DATA" ]] && [[ -n "$(ls -A "$DEST_DATA" 2>/dev/null)" ]]; then
    DEST_HAS_DATA=true
fi

if [[ "$DEST_HAS_DATA" == true ]] && [[ "$FORCE" == false ]]; then
    echo "${RED}Error: blz-dev already has data${RESET}" >&2
    echo "Use --force to overwrite existing data" >&2
    exit 1
fi

# Copy function
copy_item() {
    local src=$1
    local dest=$2
    local description=$3

    if [[ ! -e "$src" ]]; then
        return 0
    fi

    if [[ "$DRY_RUN" == true ]]; then
        echo "${YELLOW}[DRY RUN]${RESET} Would copy: $description"
        echo "  From: $src"
        echo "  To:   $dest"
        return 0
    fi

    echo "${GREEN}✓${RESET} Copying: $description"

    # Create parent directory if needed
    mkdir -p "$(dirname "$dest")"

    if [[ -d "$src" ]]; then
        cp -R "$src" "$dest"
    else
        cp "$src" "$dest"
    fi
}

# Perform copies
COPIED_ANYTHING=false

# Config files
if [[ "$SOURCES_ONLY" == false ]] && [[ -n "$SRC_CONFIG" ]]; then
    if [[ -f "$SRC_CONFIG/config.toml" ]]; then
        copy_item "$SRC_CONFIG/config.toml" "$DEST_CONFIG/config.toml" "Configuration file"
        COPIED_ANYTHING=true
    fi

    if [[ -f "$SRC_CONFIG/data.json" ]]; then
        copy_item "$SRC_CONFIG/data.json" "$DEST_CONFIG/data.json" "Registry state"
        COPIED_ANYTHING=true
    fi

    if [[ -f "$SRC_CONFIG/history.jsonl" ]]; then
        copy_item "$SRC_CONFIG/history.jsonl" "$DEST_CONFIG/history.jsonl" "Search history"
        COPIED_ANYTHING=true
    fi
fi

# Source data
if [[ "$CONFIG_ONLY" == false ]] && [[ -n "$SRC_DATA" ]] && [[ -d "$SRC_DATA/sources" ]]; then
    copy_item "$SRC_DATA/sources" "$DEST_DATA/sources" "All source data and indices"
    COPIED_ANYTHING=true
fi

echo ""
if [[ "$COPIED_ANYTHING" == false ]]; then
    echo "${YELLOW}No data found to copy${RESET}"
    exit 0
fi

if [[ "$DRY_RUN" == true ]]; then
    echo "${YELLOW}Dry run complete. No files were copied.${RESET}"
    echo "Run without --dry-run to perform the actual copy."
else
    echo "${GREEN}${BOLD}✓ Hydration complete!${RESET}"
    echo ""
    echo "Your blz-dev setup now mirrors your production blz installation."
    echo "Run ${BOLD}blz-dev list${RESET} to verify sources were copied correctly."
fi
