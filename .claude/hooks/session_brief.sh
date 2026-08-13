#!/usr/bin/env bash
# SessionStart — orient even when /load isn't typed.
#
# Prints the handoff's "Start here" plus the counts that go stale silently: unverified [~] items,
# open [ ] items, and unfiled issues. It does NOT replace /load — /load verifies the handoff
# against the repo, which this cannot do. It just makes a rotting item visible at zero cost.
#
# CHANGED 2026-08-13: live task directories are per-owner and tracked — work/<owner>/current/ and
# work/<owner>/parked/<slug>/ — so a task can be picked up on another machine and a blocked task can
# be parked instead of held. The owner is resolved from `git config user.email` against the same
# table documentation/mdube_edits.md uses; an unrecognised address prints one line and exits 0,
# because a hook that blocks a session is worse than a hook that says nothing.
#
# Test:  .claude/hooks/session_brief.sh

set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR" || exit 0

rule_top="─── .claude ───────────────────────────────────────────────"
rule_bot="───────────────────────────────────────────────────────────"

email="$(git config user.email 2>/dev/null || true)"
case "$email" in
    mdube04@uoguelph.ca|michael.dube@ovgu.de|35709889+md12ol@users.noreply.github.com)
        owner="mdube"; other="jsargant"; other_name="James" ;;
    shorinbonsai@gmail.com)
        owner="jsargant"; other="mdube"; other_name="Michael" ;;
    *)
        echo "$rule_top"
        echo "Unrecognised git user.email (${email:-unset}) — cannot tell whose work/ directory this is."
        echo "Add it to the table in this hook and in documentation/mdube_edits.md, then re-run."
        echo "$rule_bot"
        exit 0 ;;
esac

cur="work/$owner/current"
parked="work/$owner/parked"

# One line about the other owner, so their parked work is visible without being readable noise.
other_line=""
other_cur=0
[[ -f "work/$other/current/plan.md" ]] && other_cur=1
other_parked=$(find "work/$other/parked" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
if [[ "$other_cur" -gt 0 || "$other_parked" -gt 0 ]]; then
    other_line="$other_name: $other_cur current, $other_parked parked"
fi

# Parked tasks of your own, with what each is blocked on — the question you actually ask on return.
parked_report() {
    local d slug blocked
    for d in "$parked"/*/; do
        [[ -d "$d" ]] || continue
        slug="$(basename "$d")"
        blocked="$(grep -m1 -o '\*\*Blocked on:\*\*.*' "$d/handoff.md" 2>/dev/null | sed 's/\*\*Blocked on:\*\* *//')"
        printf '  parked: %-24s %s\n' "$slug" "${blocked:-no blocker recorded}"
    done
}

if [[ ! -f "$cur/plan.md" ]]; then
    echo "$rule_top"
    if [[ -n "$(parked_report)" ]]; then
        echo "No active task in .claude/$cur/ — these are parked:"
        parked_report
        echo "Resume one with /load <slug>, or start something new with /start."
    else
        echo "No active task in .claude/$cur/ — start one with /start."
    fi
    [[ -n "$other_line" ]] && echo "$other_line"
    echo "$rule_bot"
    exit 0
fi

# `grep -c` prints its count AND exits 1 when the count is zero, so the obvious `|| echo 0` appends
# a SECOND zero and the counts line breaks across two lines. Take the first line and default empty.
count() { local n; n=$(grep -c "$1" "$2" 2>/dev/null | head -1); echo "${n:-0}"; }

open=$(count '^- \[ \]' "$cur/plan.md")
unver=$(count '^- \[~\]' "$cur/plan.md")
unfiled=$(count 'Filed:.*not yet' work/issues.md)
traps=$(count '^### ' work/traps.md)

echo "$rule_top"
if [[ -f "$cur/handoff.md" ]]; then
    sed -n '1p' "$cur/handoff.md"
    # The machine stamp, if the handoff carries one — this is how a stale cross-machine handoff shows.
    grep -m1 '^\*\*Machine:' "$cur/handoff.md" 2>/dev/null
    # The "Start here" section, up to the next heading or the next bold label.
    #
    # FIXED 2026-08-13: this matched only a `## Start here` HEADING, and no handoff has ever had one
    # — /save's template writes `**Start here:**` inline, so this printed nothing from the day it was
    # written. Found while testing the parked-task swap. Both forms are accepted now.
    awk '/^(## .*Start here|\*\*Start here)/{f=1; print; next}
         f && /^(## |\*\*[A-Z⏰])/{exit}
         f' "$cur/handoff.md" | head -12
fi
printf '\nopen [ ]: %s   unverified [~]: %s   unfiled issues: %s   traps: %s\n' \
    "$open" "$unver" "$unfiled" "$traps"
parked_report
[[ -n "$other_line" ]] && echo "$other_line"
echo "Run /load to verify this against the repo before trusting it."
echo "$rule_bot"

exit 0
