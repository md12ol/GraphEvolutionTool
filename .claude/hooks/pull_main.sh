#!/usr/bin/env bash
# SessionStart — fast-forward `main` so nothing pushed directly to it (.claude docs, decisions.md
# and collab.md entries, traps, hotfixes) sits unseen for a whole session.
#
# Only ever fast-forwards, and only on `main`. If the branch is anything else — any feature
# branch counts as mid-task — this does nothing. If `main` can't fast-forward (local commits not
# on origin, or a working-tree change origin's version would overwrite), it prints one line and
# leaves everything untouched. It never merges, rebases, or discards anything.
#
# Test:  .claude/hooks/pull_main.sh

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
cd "$ROOT" || exit 0

branch="$(git branch --show-current 2>/dev/null)"
[[ "$branch" == "main" ]] || exit 0

git fetch origin main --quiet 2>/dev/null || { echo "pull_main: couldn't reach origin, skipping"; exit 0; }

local_sha="$(git rev-parse HEAD 2>/dev/null)"
remote_sha="$(git rev-parse origin/main 2>/dev/null)"
[[ -n "$local_sha" && -n "$remote_sha" ]] || exit 0
[[ "$local_sha" == "$remote_sha" ]] && exit 0

if git merge-base --is-ancestor HEAD origin/main 2>/dev/null; then
    if git merge --ff-only origin/main --quiet 2>/dev/null; then
        echo "pull_main: fast-forwarded main to $(git rev-parse --short HEAD) (was behind origin)"
    else
        echo "pull_main: origin/main moved but the fast-forward failed (local changes in the way) — pull by hand"
    fi
else
    echo "pull_main: local main has commits origin doesn't — left untouched, push or resolve by hand"
fi

exit 0
