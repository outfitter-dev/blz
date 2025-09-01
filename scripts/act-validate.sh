#!/usr/bin/env bash
# Act local validation script for GitHub Actions workflows
# Provides fast/full validation modes for different development stages

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default settings
MODE="${1:-fast}"
WORKFLOW="${2:-rust-ci}"
VERBOSE="${VERBOSE:-0}"
ACT_PLATFORM="${ACT_PLATFORM:-ubuntu-latest=catthehacker/ubuntu:act-latest}"

# Help text
show_help() {
    cat << EOF
Usage: $0 [MODE] [WORKFLOW]

Modes:
  fast     Quick validation (<30s) - format, clippy basics (default)
  full     Complete workflow run - all jobs, all steps
  test     Run test suite only
  format   Run format checks only
  clippy   Run clippy checks only

Workflows:
  rust-ci  Main Rust CI workflow (default)
  miri     Miri unsafe validation (slow, not recommended locally)

Environment variables:
  VERBOSE=1          Show detailed act output
  ACT_PLATFORM       Override Docker image platform
  ACT_REUSE=1        Reuse containers between runs (faster)

Examples:
  $0                  # Fast validation with rust-ci
  $0 full             # Full rust-ci workflow
  $0 test rust-ci     # Test suite only
  VERBOSE=1 $0 fast   # Fast mode with debug output
EOF
}

# Check if act is installed
check_act() {
    if ! command -v act &> /dev/null; then
        echo -e "${RED}Error: act is not installed${NC}"
        echo "Install with: brew install act (macOS) or see https://github.com/nektos/act"
        exit 1
    fi
}

# Create act configuration if not exists
setup_actrc() {
    local actrc=".actrc"
    if [ ! -f "$actrc" ]; then
        echo -e "${YELLOW}Creating .actrc configuration...${NC}"
        cat > "$actrc" << 'EOF'
# Act configuration for local GitHub Actions testing
# Platform mappings for faster, smaller images
--platform ubuntu-latest=catthehacker/ubuntu:act-latest
--platform ubuntu-22.04=catthehacker/ubuntu:act-22.04
--platform ubuntu-20.04=catthehacker/ubuntu:act-20.04

# Use host Docker daemon
--container-daemon-socket -

# Reuse containers for speed
--reuse

# Pull images if needed
--pull=false

# Default event
--eventpath .github/workflows/act-event.json
EOF
    fi
}

# Create minimal event file for act
setup_event() {
    local event_file=".github/workflows/act-event.json"
    if [ ! -f "$event_file" ]; then
        mkdir -p "$(dirname "$event_file")"
        cat > "$event_file" << 'EOF'
{
  "pull_request": {
    "number": 999,
    "head": {
      "ref": "act-local-test",
      "sha": "0000000000000000000000000000000000000000"
    },
    "base": {
      "ref": "main"
    }
  },
  "repository": {
    "name": "blz",
    "owner": {
      "login": "outfitter-dev"
    }
  }
}
EOF
    fi
}

# Run act with appropriate settings
run_act() {
    local workflow_file=".github/workflows/${WORKFLOW}.yml"
    local job_filter=""
    local extra_args=""
    
    if [ ! -f "$workflow_file" ]; then
        echo -e "${RED}Error: Workflow file not found: $workflow_file${NC}"
        exit 1
    fi
    
    # Configure based on mode
    case "$MODE" in
        fast)
            # Run only format and basic clippy checks
            job_filter="-j rust"
            extra_args="--env FAST_MODE=1"
            echo -e "${BLUE}Running fast validation (format + clippy)...${NC}"
            ;;
        format)
            job_filter="-j rust"
            extra_args="--env FORMAT_ONLY=1"
            echo -e "${BLUE}Running format checks...${NC}"
            ;;
        clippy)
            job_filter="-j rust"
            extra_args="--env CLIPPY_ONLY=1"
            echo -e "${BLUE}Running clippy checks...${NC}"
            ;;
        test)
            job_filter="-j rust"
            extra_args="--env TEST_ONLY=1"
            echo -e "${BLUE}Running test suite...${NC}"
            ;;
        full)
            echo -e "${YELLOW}Running full workflow (this may take several minutes)...${NC}"
            ;;
        *)
            echo -e "${RED}Unknown mode: $MODE${NC}"
            show_help
            exit 1
            ;;
    esac
    
    # Build act command as array to handle arguments properly
    local -a act_args=()
    act_args+=("pull_request")
    act_args+=("-W" "$workflow_file")
    act_args+=("--platform" "$ACT_PLATFORM")
    
    if [ -n "$job_filter" ]; then
        act_args+=("$job_filter")
    fi
    
    if [ -n "$extra_args" ]; then
        # Split extra_args by spaces and add each part
        IFS=' ' read -ra EXTRA <<< "$extra_args"
        for arg in "${EXTRA[@]}"; do
            act_args+=("$arg")
        done
    fi
    
    if [ "$VERBOSE" = "1" ]; then
        act_args+=("--verbose")
    else
        act_args+=("--quiet")
    fi
    
    # Add reuse flag if set
    if [ "${ACT_REUSE:-1}" = "1" ]; then
        act_args+=("--reuse")
    fi
    
    # Execute act
    echo -e "${BLUE}Executing: act ${act_args[*]}${NC}"
    
    if act "${act_args[@]}"; then
        echo -e "${GREEN}✓ Validation passed!${NC}"
        return 0
    else
        echo -e "${RED}✗ Validation failed!${NC}"
        return 1
    fi
}

# Performance timer
time_execution() {
    local start_time=$(date +%s)
    "$@"
    local exit_code=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo -e "${BLUE}Execution time: ${duration}s${NC}"
    return $exit_code
}

# Main execution
main() {
    # Handle help flag
    if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
        show_help
        exit 0
    fi
    
    check_act
    setup_actrc
    setup_event
    
    # Run with timing
    time_execution run_act
}

# Execute if not sourced
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi