#!/usr/bin/env bash
# One-time, per-machine setup: create the dedicated `main` worktree that /save, /park, /load,
# /done and /start now use for everything under .claude/work/, write a two-folder VS Code
# workspace file pointing at it, and open that workspace.
#
# Background: CLAUDE.md, "`.claude/work/` lives in a dedicated `main` worktree"; collab.md #58.
#
# The whole setup, start to finish:
#
#   git checkout main && git pull
#   bash .claude/scripts/setup_docs_worktree.sh
#
# That's it — the script creates the worktree and opens the workspace itself if `code` is on
# PATH. Safe to re-run any time: it's a no-op on the worktree if one already exists, and it
# rewrites + reopens the workspace file either way (handy if you just want the window back).
# Never touches your current branch or working tree changes — it only reads origin/main and
# creates a new sibling directory.

set -euo pipefail

say()  { printf '%s\n' "$*"; }
step() { printf -- '--- %s\n' "$*"; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

MAIN_TREE="$(git rev-parse --show-toplevel 2>/dev/null)" \
    || die "not inside a git repository — cd into your GraphEvolutionTool checkout first."
cd "$MAIN_TREE"

REPO_NAME="$(basename "$MAIN_TREE")"
DOCS_WT="$(dirname "$MAIN_TREE")/${REPO_NAME}-docs"
WORKSPACE_FILE="$MAIN_TREE/${REPO_NAME}.code-workspace"

say "Repo:            $MAIN_TREE"
say "Docs worktree:   $DOCS_WT"
say "Workspace file:  $WORKSPACE_FILE"
say ""

# A multi-root workspace: both folders in one window. Note there is nothing folder-specific
# excluded here — an earlier version tried to hide everything except .claude/work in the docs
# root via a folder-local .vscode/settings.json, and VS Code intermittently rendered that root as
# entirely empty (Explorer showed no children at all, sometimes not even after a reload). The
# sparse checkout already keeps that folder down to just .claude/work/ on disk, so the exclude
# rule was redundant as well as the likely cause — removed rather than fought with. If both
# folders still don't show after opening this, `code "<docs-worktree-path>"` on its own is the
# fallback (CLAUDE.md has the full story).
write_workspace_file() {
    cat > "$WORKSPACE_FILE" <<EOF
{
    "folders": [
        {
            "name": "$REPO_NAME",
            "path": "."
        },
        {
            "name": "$REPO_NAME (docs, always main)",
            "path": "$DOCS_WT"
        }
    ]
}
EOF
    say "Wrote $WORKSPACE_FILE"
}

open_workspace() {
    if command -v code >/dev/null 2>&1; then
        say "Opening it now: code \"$WORKSPACE_FILE\""
        code "$WORKSPACE_FILE" >/dev/null 2>&1 &
        disown
    else
        say "'code' isn't on PATH — open it yourself:"
        say "  code \"$WORKSPACE_FILE\""
    fi
}

# Optional, confirmed: hides .claude/work/ in the CODE folder's Explorer, since the live copy
# lives in the docs worktree now and this branch's copy is stale (its own current/parked was
# removed; decisions.md/traps.md/etc. here are only whatever was last merged in). Purely cosmetic
# — nothing about how /save, /park, /load, /done or /start behave. Never applied without asking,
# and only ever touches the single "files.exclude" key in your settings.json.
offer_hide_stale_work() {
    local settings="$MAIN_TREE/.vscode/settings.json"

    say "One more, optional: hide .claude/work/ in this folder's Explorer view."
    say "  Why: the copy of .claude/work/ tracked on this branch is stale now — the live one is"
    say "  in the docs worktree you just opened. Hiding it here just avoids reading the wrong"
    say "  copy by accident; nothing about how the skills behave changes either way."
    say "  What it touches: adds/merges one key, \"files.exclude\": {\".claude/work\": true}, into"
    say "  $settings — nothing else in that file is changed."

    if [[ ! -t 0 ]]; then
        say "  (not an interactive terminal — skipping; re-run interactively if you want this.)"
        return 0
    fi

    read -r -p "  Apply it? [y/N] " reply
    case "$reply" in
        [yY]|[yY][eE][sS]) ;;
        *) say "  Skipped — nothing written."; return 0 ;;
    esac

    mkdir -p "$MAIN_TREE/.vscode"
    if [[ ! -f "$settings" ]]; then
        printf '{\n    "files.exclude": {\n        ".claude/work": true\n    }\n}\n' > "$settings"
        say "  Created $settings."
    elif command -v jq >/dev/null 2>&1; then
        tmp="$(mktemp)"
        jq '.["files.exclude"][".claude/work"] = true' "$settings" > "$tmp" \
            && mv "$tmp" "$settings" \
            && say "  Merged into existing $settings." \
            || { rm -f "$tmp"; say "  jq failed to merge — left your settings.json untouched. Add by hand:"; \
                 say "    \"files.exclude\": { \".claude/work\": true }"; }
    else
        say "  $settings already exists and 'jq' isn't on PATH, so this won't try to merge it"
        say "  automatically — a blind text edit risks corrupting your existing settings. Add"
        say "  this key yourself, inside the existing \"files.exclude\" object if you have one,"
        say "  or as a new top-level key if you don't:"
        say "    \"files.exclude\": { \".claude/work\": true }"
    fi
}

if [[ -d "$DOCS_WT" ]]; then
    say "Already set up — $DOCS_WT exists."
    say "If it looks wrong: git worktree remove \"$DOCS_WT\"   (then re-run this script)"
    say ""
    write_workspace_file
    open_workspace
    say ""
    offer_hide_stale_work
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

step "Creating the worktree (sparse — only .claude/work/, not a second full checkout)"
git worktree add --no-checkout "$DOCS_WT" main
(
    cd "$DOCS_WT"
    git sparse-checkout init --no-cone
    git sparse-checkout set '/.claude/work/*'
    git checkout main --quiet
)
say ""
say "Done. $DOCS_WT is a permanent checkout of 'main' now, containing only .claude/work/ — not a"
say "second copy of the whole repo. /save, /park, /load, /done and /start read and write"
say "everything under .claude/work/ there instead of wherever your primary tree happens to be"
say "checked out — that's the whole fix. You don't do anything differently; the skills find it"
say "automatically from here on. The hook (session_brief.sh) still runs from your primary tree as"
say "usual — it reads main's content via 'git show', not from this worktree."
say ""

step "Setting up the editor workspace"
write_workspace_file
open_workspace
say ""
offer_hide_stale_work
