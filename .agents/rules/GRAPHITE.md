# GRAPHITE.md

This repository uses Graphite for source control. In many cases, `gt` commands and workflows replace typical git commands and operations. It's critical to use the `gt` commands to keep things properly maintained.

## Requirements

- Graphite CLI (`gt`) must be installed
- Repository must be initialized with `gt init`
- User must be authenticated with `gt auth`

## Mental Model & Core Concepts

### Stack Mental Model

A **stack** is a linear chain of branches where each branch:

- Depends on exactly one parent branch (or trunk)
- Contains typically one commit for surgical code review
  - IMPORTANT: If using a single commit message, it should be about *all* changes made within the branch
- Can have zero or more child branches
- Is tracked in Graphite's dependency graph

Graphite provides safe, stack-aware operations that maintain these relationships automatically.

### Core Commands Reference

#### Navigation & Visualization

```bash
gt log              # Visualize stacks with full branch info
gt log short        # Compact stack visualization (alias: gt ls)
gt log long         # Full commit ancestry graph (alias: gt ll)
gt log --stack      # Show only current stack (ancestors + descendants)
gt checkout         # Interactive branch selector
gt up [n]           # Move up the stack n steps (default: 1)
gt down [n]         # Move down the stack n steps (default: 1)
gt top              # Jump to the tip of current stack
gt bottom           # Jump to the bottom of current stack
```

#### Creating & Modifying

```bash
gt create [name] -am "message"    # Create branch with staged changes
gt modify -am "message"            # Amend current branch's commit
gt modify -cam "message"           # Create new commit on current branch
gt modify --interactive-rebase     # Interactive rebase for current branch
gt absorb -a                       # Auto-distribute hunks to correct commits
```

#### Publishing & Syncing

```bash
gt submit --stack           # Push and create/update PRs for entire stack
gt sync                     # Pull trunk, prune merged, auto-restack
gt merge                    # Merge PRs in order from trunk to current
```

#### Stack Operations

```bash
gt restack                  # Rebase current branch onto its parent
gt restack --upstack        # Restack current + all descendants
gt restack --downstack      # Restack current + all ancestors
gt move --onto <branch>     # Change parent branch + restack descendants
gt reorder                  # Interactively reorder stack branches
gt split                    # Split branch into multiple single-commit branches
gt squash                   # Squash commits in current branch, restack descendants
gt fold                     # Fold current branch into parent and restack
```

#### Branch Management

```bash
gt track --parent <branch>  # Start tracking untracked branch
gt untrack                  # Remove branch from Graphite tracking
gt rename <new-name>        # Rename branch with metadata update
gt delete <branch>          # Delete branch + restack children onto parent
gt get <branch|pr#>         # Pull teammate's stack locally (defaults to restack; skip via --no-restack / scope with --downstack)
gt pr --stack               # Open the stack page in your browser
gt undo                     # Undo the most recent Graphite mutation
```

#### Conflict Resolution

```bash
gt continue         # Continue after resolving conflicts
gt abort            # Abort current Graphite operation
```

## Standard Workflow Patterns

### Core Workflow

```bash
# Start of session
gt sync                                 # Update trunk, prune merged branches
gt log short                            # Check stack state

# Create new work
gt create feat/new-feature -am "Initial implementation"

# Iterate on current branch
gt modify -am "Address review feedback"  # Amend existing commit (be sure the message considers all changes, not just latest)
# OR
gt modify -cam "Add additional changes"  # New commit

# Fix the right commit in the stack
gt absorb -a                            # Auto-distribute changes to correct commits

# Publish/update PRs
gt submit --stack                       # Create/update all PRs in stack

# Navigate the stack
gt down                                 # Go to parent branch
gt up 2                                 # Go up two branches
gt top                                  # Jump to stack tip
```

### Conflict Resolution Pattern

```bash
# When Graphite operation encounters conflicts:
# 1. Resolve conflicts in files
# 2. Stage resolved files
git add <resolved-files>

# 3. Continue Graphite operation
gt continue

# Or abort if needed
gt abort
```

### Stack Repair Pattern

```bash
# When stack needs comprehensive fix
gt sync
gt restack --upstack
gt submit --stack
```

## Critical Rules: NEVER → ALWAYS → WHY

### Performing Publishing & Syncing

**NEVER:** `git push` or `git push --force` on tracked branches
**ALWAYS:** `gt submit --stack` (add `--always` if web view desynced)
**WHY:** Ensures stack coherence, prevents unsafe overwrites, maintains PR relationships

**NEVER:** `git pull` on tracked branches
**ALWAYS:** `gt sync`
**WHY:** Prevents merge commits, auto-prunes merged branches, maintains linear history

### Performing Branch Operations

**NEVER:** `git checkout -b` for stack work
**ALWAYS:** `gt create` or `gt track --parent`
**WHY:** Ensures proper parent tracking from creation

**NEVER:** `git branch -m` on branches with PRs
**ALWAYS:** `gt rename`
**WHY:** Updates Graphite metadata. Note: GitHub PR branch names are immutable—renaming removes the PR association
**WARNING:** If you have an open PR, avoid renaming the branch entirely—it will break the PR link and you'll need to recreate the PR

**NEVER:** `git branch -D` on tracked branches
**ALWAYS:** `gt delete` or `gt untrack`
**WHY:** Properly restacks children, cleans metadata

### Performing History Rewrites

**NEVER:** `git rebase -i` on tracked branches
**ALWAYS:** `gt modify --interactive-rebase`
**WHY:** Maintains descendant alignment, provides clear continuation path

**NEVER:** `git rebase --onto` to change parents
**ALWAYS:** `gt move --onto`
**WHY:** Restacks all descendants automatically

**NEVER:** Manual `git cherry-pick` across stack branches
**ALWAYS:** `gt absorb`
**WHY:** Automatically amends correct commits and restacks

### Collaboration

**NEVER:** `git fetch && git checkout <teammate-branch>`
**ALWAYS:** `gt get <branch|pr#>`
**WHY:** Fetches entire stack coherently with dependencies

**NEVER:** Manually merge PRs out of order in the GitHub UI
**ALWAYS:** `gt merge` (preview with `gt merge --dry-run`)
**WHY:** Ensures correct trunk→tip ordering and prevents accidental mid-stack merges

## Recovery Scenarios

### Untracked Branch Created with Git

**Symptom:** Branch missing from `gt log`

```bash
# Fix
gt track --parent <correct-parent>
gt restack --only
gt submit --stack
```

### Pushed with Raw Git

**Symptom:** PRs out of sync with stack view

```bash
# Fix
gt submit --stack        # Normal update
# If severely desynced:
gt submit --always --stack
```

### Merge Commit Mid-Stack

**Symptom:** Non-linear history from `git pull`

```bash
# Fix
gt sync
gt restack --only
```

### Manual Interactive Rebase

**Symptom:** Descendants misaligned after `git rebase -i`

```bash
# Fix
gt restack --upstack
gt submit --stack
```

### Branch Renamed with Git

**Symptom:** PR association broken

```bash
# Fix
gt rename <new-name>  # Even if already renamed
gt submit --stack
```

### Parent Changed Manually

**Symptom:** Children still on old base after `git rebase --onto`

```bash
# Fix
gt move --onto <new-base>
gt restack --upstack
gt submit --stack
```

### Deleted with Git

**Symptom:** Metadata/children dangling

```bash
# Fix
gt delete <branch>    # Metadata-aware cleanup
# OR
gt untrack <branch>   # Just remove from tracking
gt sync
```

## Agent Guardrails

### Pre-push Hook

Save as `.git/hooks/pre-push` (make executable). Blocks raw pushes only on tracked branches (and allows trunk). Set `GT_BYPASS_GUARD=1` to bypass in emergencies.

```bash
#!/usr/bin/env bash
set -euo pipefail

# allow escape hatch
if [[ "${GT_BYPASS_GUARD:-}" == "1" ]]; then exit 0; fi

# skip if Graphite not installed or repo not initialized
command -v gt >/dev/null 2>&1 || exit 0
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0

current="$(git rev-parse --abbrev-ref HEAD)"
trunk="$(gt trunk 2>/dev/null | head -n1 | awk '{print $1}')"

# if we're on trunk, allow normal pushes
if [[ -n "${trunk}" && "${current}" == "${trunk}" ]]; then exit 0; fi

# if current branch is tracked (has a parent), block raw push
if gt parent >/dev/null 2>&1; then
  echo "[Guardrail] Tracked Graphite branch detected: '${current}'. Use 'gt submit --stack' instead of 'git push'." >&2
  echo "           (Set GT_BYPASS_GUARD=1 to bypass once.)" >&2
  exit 1
fi

exit 0
```

### Quick Reference

```bash
# Fix everything
gt sync && gt restack --upstack && gt submit --stack

# Jump to tip and create
gt top && gt create -am "message"

# Show current stack only
gt log --stack

# Interactive branch switch
gt checkout
```

## Command Aliases & Shortcuts

Common aliases configured by default:

- `gt ss` → `gt submit --stack`
- `gt ls` → `gt log short`
- `gt ll` → `gt log long`

## Important Implementation Notes

### For AI Agents

1. **Always verify Graphite is initialized** before using `gt` commands
2. **Never mix raw Git branch operations** with Graphite-tracked branches
3. **Use `gt log short` frequently** to verify stack state
4. **Prefer `gt modify` over `gt modify -c`** for cleaner history
5. **Run `gt sync` at the start** of any work session
6. **Use `--force` flags sparingly** and only when certain

### Error Handling

- If a command fails with "not tracked", use `gt track --parent`
- If "needs restack" appears, run `gt restack` before continuing
- If conflicts occur, always resolve then `gt continue`
- If unsure about state, run diagnostic: `gt log short && git status`

### Performance Considerations

- `gt log long` can be slow on large repos
- `gt sync` with many branches may take time on first run
- `gt submit --stack` validates entire stack - be patient

## Additional Resources

- Official Docs: https://graphite.dev/docs
- Command Reference: https://graphite.dev/docs/command-reference
