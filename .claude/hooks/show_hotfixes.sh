#!/usr/bin/env bash
# PreToolUse(Edit|Write) — warn at the moment of the edit, not an hour after CLAUDE.md was read.
#
# Two cases, both from CLAUDE.md's "Two people, one .claude/" section:
#
#   1. Co-owned source. Files where Michael's and James's work meet. Editing one can collide with
#      the other person's branch, and the disposition is per-file — only hotfixes.md and collab.md
#      know which. Prints the matching hotfixes.md entries and the open collab.md items.
#
#   2. The .claude/ machinery itself. settings.json and hooks/*.sh execute on the OTHER person's
#      machine, at session start, on their next pull, without them reading the diff. Those changes
#      go through a PR — this is the reminder.
#
# Never blocks — exit 0 always. Both kinds of edit are legitimate; they just need to be deliberate.
#
# Per-machine override: set GET_COOWNED_PATHS (an ERE) in .claude/settings.local.json to narrow
# this to the files the OTHER owner is actually working on right now.
#
# Test:
#   echo '{"tool_input":{"file_path":"get/src/evolver/generational.rs"}}' | .claude/hooks/show_hotfixes.sh
#   echo '{"tool_input":{"file_path":".claude/hooks/session_brief.sh"}}'  | .claude/hooks/show_hotfixes.sh

set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

FILE="$(python3 -c 'import json,sys
try:
    t = json.load(sys.stdin).get("tool_input", {})
    print(t.get("file_path") or t.get("notebook_path") or "")
except Exception:
    print("")' 2>/dev/null)"

[[ -z "$FILE" ]] && exit 0

# ── 1. Co-owned source ────────────────────────────────────────────────────────────────────────
# The evolver module and the traits both strategies implement against. generational.rs is James's
# live work; steady_state.rs is Michael's; common.rs, mod.rs, fitness.rs and genome.rs are the
# shared surface between them. Update this list as ownership moves — an out-of-date pattern that
# never fires is the same as no hook.
COOWNED="${GET_COOWNED_PATHS:-get/src/evolver/(generational|common|mod)\.rs|get/src/(fitness|genomes/genome)\.rs}"

if grep -qE "$COOWNED" <<<"$FILE"; then
    cat <<EOF

⚠  $FILE is co-owned — the other owner may have live work in it.

CLAUDE.md: read hotfixes.md BEFORE editing, staging or reverting here, and check the Owner: line.
A hotfix owned by someone else is NOT in your working tree. The rules are not uniform — one file
may have to be committed and another must never be, and only the entry knows which.

hotfixes.md entries (heading · owner · where · remove-when):
EOF
    grep -nE '^### |^- \*\*(Owner|Machine|Where|Remove when):' "$DIR/work/hotfixes.md" 2>/dev/null \
        | sed 's/^/  /' | head -60

    echo
    echo "Open collab.md items — settle a conflict there rather than overwriting their work:"
    awk '/^## Open/{f=1;next} /^## /{f=0} f&&/^### /' "$DIR/work/collab.md" 2>/dev/null \
        | sed 's/^/  /' | head -20
    echo
fi

# ── 2. The .claude/ machinery ─────────────────────────────────────────────────────────────────
if grep -qE '(^|/)\.claude/(settings\.json|hooks/)' <<<"$FILE"; then
    cat <<EOF

⚠  $FILE runs on the other owner's machine.

settings.json and hooks/*.sh are executable code that fires at THEIR session start on their next
pull, without them reading it. CLAUDE.md: these changes go through a PR — never straight to main.
Say in the PR what the hook now does.

(settings.local.json is the per-machine escape hatch and is gitignored — use it for anything
personal, and nothing here applies.)

EOF
fi

exit 0
