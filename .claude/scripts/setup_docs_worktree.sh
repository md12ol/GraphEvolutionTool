#!/usr/bin/env bash
# One-time, per-machine setup: create the dedicated `main` worktree that /save, /park, /load,
# /done and /start now use for everything under .claude/work/.
#
# Background: CLAUDE.md, "`.claude/work/` lives in a dedicated `main` worktree"; collab.md #58.
#
# Run from anywhere inside your GraphEvolutionTool checkout:
#
#   bash .claude/scripts/setup_docs_worktree.sh
#
# Safe to re-run — it's a no-op if the worktree already exists. Does not touch your current
# branch or working tree changes; it only reads origin/main and creates a new sibling directory.

set -euo pipefail

say()  { printf '%s\n' "$*"; }
step() { printf -- '--- %s\n' "$*"; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

MAIN_TREE="$(git rev-parse --show-toplevel 2>/dev/null)" \
    || die "not inside a git repository — cd into your GraphEvolutionTool checkout first."
cd "$MAIN_TREE"

DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"

say "Repo:          $MAIN_TREE"
say "Docs worktree: $DOCS_WT"
say ""

if [[ -d "$DOCS_WT" ]]; then
    say "Already set up — $DOCS_WT exists. Nothing to do."
    say "If it looks wrong: git worktree remove \"$DOCS_WT\"   (then re-run this script)"
    exit 0
fi

step "Checking your primary tree is in a state this can safely run from"

branch="$(git branch --show-current)"
[[ -n "$branch" ]] || die "primary tree is in a detached HEAD state — check out a branch first."

if [[ "$branch" == "main" ]]; then
    die "primary tree currently has 'main' checked out. Git will not let the same branch be
checked out in two worktrees at once, so creating a 'main' worktree from here would fail anyway.
Switch to any other branch first — e.g. 'git checkout <your-feature-branch>', or
'git checkout -b scratch' if you have nothing else in flight — then re-run this script."
fi
say "OK: on branch '$branch', not 'main'."

if [[ -n "$(git status --porcelain)" ]]; then
    die "primary tree has uncommitted changes. This script never touches '$branch' or your
working tree — it only reads origin/main and creates a new sibling directory — but a dirty tree
here usually means mid-session work. Commit, stash, or finish up first, then re-run."
fi
say "OK: working tree is clean."

step "Syncing local 'main' with origin/main"
git fetch origin main --quiet || die "could not reach origin — check your network/credentials and retry."

if git show-ref --verify --quiet refs/heads/main; then
    local_sha="$(git rev-parse main)"
    remote_sha="$(git rev-parse origin/main)"
    if [[ "$local_sha" == "$remote_sha" ]]; then
        say "OK: local 'main' already matches origin/main ($remote_sha)."
    elif git merge-base --is-ancestor main origin/main; then
        say "Local 'main' is behind — fast-forwarding the ref (not checking it out here)."
        git fetch origin main:main --quiet \
            || die "could not fast-forward local main — resolve by hand, then re-run:
  git branch -f main origin/main   (only if you're sure you have nothing local on main)"
        say "OK: local 'main' now at $(git rev-parse main)."
    else
        die "local 'main' has commits origin/main doesn't. This script won't guess how to
reconcile that — resolve it by hand (ask Michael if unsure), then re-run."
    fi
else
    say "No local 'main' branch yet — creating one that tracks origin/main."
    git branch main origin/main
    say "OK: local 'main' created at $(git rev-parse main)."
fi

step "Creating the worktree"
git worktree add "$DOCS_WT" main
say ""
say "Done. $DOCS_WT is a permanent checkout of 'main' now. /save, /park, /load, /done and /start"
say "read and write everything under .claude/work/ there instead of wherever your primary tree"
say "happens to be checked out — that's the whole fix. You don't do anything differently; the"
say "skills find it automatically from here on."
say ""
say "Verify it end to end:"
say "  bash \"$DOCS_WT/.claude/hooks/session_brief.sh\""
