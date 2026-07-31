#!/usr/bin/env bash
# SessionStart — orient even when /load isn't typed.
#
# Prints the handoff's "Start here" plus the counts that go stale silently: unverified [~] items,
# open [ ] items, and unfiled issues. It does NOT replace /load — /load verifies the handoff
# against the repo, which this cannot do. It just makes a rotting item visible at zero cost.
#
# Test:  .claude/hooks/session_brief.sh

set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR" || exit 0

[[ -f work/current/plan.md ]] || { echo "No active task in .claude/work/current/ — start one with /start."; exit 0; }

open=$(grep -c '^- \[ \]' work/current/plan.md 2>/dev/null || echo 0)
unver=$(grep -c '^- \[~\]' work/current/plan.md 2>/dev/null || echo 0)
unfiled=$(grep -c 'Filed:.*not yet' work/issues.md 2>/dev/null || echo 0)
traps=$(grep -c '^### ' work/traps.md 2>/dev/null || echo 0)

echo "─── .claude ───────────────────────────────────────────────"
if [[ -f work/current/handoff.md ]]; then
    sed -n '1p' work/current/handoff.md
    # The "Start here" section, up to the next heading.
    awk '/^## .*Start here/{f=1} f&&/^## /&&!/Start here/{exit} f' work/current/handoff.md | head -12
fi
printf '\nopen [ ]: %s   unverified [~]: %s   unfiled issues: %s   traps: %s\n' \
    "$open" "$unver" "$unfiled" "$traps"
echo "Run /load to verify this against the repo before trusting it."
echo "───────────────────────────────────────────────────────────"

exit 0
