#!/usr/bin/env bash
# PreToolUse(Bash) — enforce CLAUDE.md's "who runs the environment" rule.
#
# Prose rules get violated; exit code 2 does not. /setup builds the patterns below from the commands
# you name, or edit BLOCK/ALLOW by hand.
#
# EDIT THESE TWO PATTERNS BEFORE ENABLING — the defaults are examples, not your project.
#
# Reads the hook JSON on stdin, blocks on a match, and tells the agent what to do instead.
# Exit 0 = allow, exit 2 = block (the message on stderr goes back to the agent).
#
# Test:
#   echo '{"tool_input":{"command":"make deploy"}}' | .claude/hooks/block_env_commands.sh; echo "rc=$?"

set -uo pipefail

CMD="$(python3 -c 'import json,sys
try:
    print(json.load(sys.stdin).get("tool_input", {}).get("command", ""))
except Exception:
    print("")' 2>/dev/null)"

[[ -z "$CMD" ]] && exit 0

# Blocked: outward-facing, hard-to-undo publish/release actions. Everything local — cargo build,
# test, clippy, fmt — is allowed; see CLAUDE.md, the agent runs those itself.
# Word-boundaried so it does not fire on substrings (e.g. `git push --help` still matches, but
# `echo pushing` does not).
BLOCK='(^|[^[:alnum:]_./-])cargo[[:space:]]+publish([^[:alnum:]_-]|$)'
BLOCK+='|(^|[^[:alnum:]_./-])gh[[:space:]]+release[[:space:]]+(create|delete|upload|edit)'

# Warned, not blocked (2026-07-31): allowed to proceed, but never silently — the agent still may
# not push unless asked. Force-pushes stay a hard block; they destroy remote history.
WARN='(^|[^[:alnum:]_./-])git[[:space:]]+push([^[:alnum:]_-]|$)'
BLOCK+='|(^|[^[:alnum:]_./-])git[[:space:]]+push([^#]*)(--force([^[:alnum:]_-]|$)|--force-with-lease|[[:space:]]-f([[:space:]]|$))'

# Explicitly allowed even though they resemble the blocked ones — read-only inspection of the
# remote. Leave empty (ALLOW='^$') if there are no such exceptions.
ALLOW='(^|[^[:alnum:]_./-])git[[:space:]]+(push[[:space:]]+--dry-run|remote|log|status)'
ALLOW+='|(^|[^[:alnum:]_./-])gh[[:space:]]+release[[:space:]]+(list|view)'

if grep -qE "$ALLOW" <<<"$CMD"; then
    exit 0
fi

if grep -qE "$WARN" <<<"$CMD" && ! grep -qE "$BLOCK" <<<"$CMD"; then
    cat >&2 <<EOF
NOTICE from .claude/hooks/block_env_commands.sh

This command writes to the remote:
  $CMD

Allowed to proceed, but only because the user asked for it. CLAUDE.md: don't commit or push
unless asked. If you were not explicitly asked to push, stop and confirm first.
EOF
    exit 0
fi

if grep -qE "$BLOCK" <<<"$CMD"; then
    cat >&2 <<EOF
BLOCKED by .claude/hooks/block_env_commands.sh

CLAUDE.md reserves this command for the user to run.

Command refused:
  $CMD

Do this instead: make the code/config change, then hand over the EXACT command to run and
the log markers that indicate success or failure. Then stop and wait.

Still allowed: reading already-generated output (logs, reports, result directories) and any
git / grep / file inspection.
EOF
    exit 2
fi

exit 0
