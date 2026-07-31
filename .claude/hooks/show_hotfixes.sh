#!/usr/bin/env bash
# PreToolUse(Edit|Write) — surface hotfixes.md when touching another team's component.
#
# CLAUDE.md's "files outside your scope" section names paths that carry deliberate working-tree
# edits. This shows hotfixes.md at the moment of the edit rather than relying on it having been read
# an hour earlier.
#
# EDIT THE PATH PATTERN BELOW BEFORE ENABLING — the default is an example.
#
# Never blocks — exit 0 always. It only prints, because the edits themselves are legitimate.
#
# Test:
#   echo '{"tool_input":{"file_path":"vendor/x.py"}}' | .claude/hooks/show_hotfixes.sh

set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

FILE="$(python3 -c 'import json,sys
try:
    t = json.load(sys.stdin).get("tool_input", {})
    print(t.get("file_path") or t.get("notebook_path") or "")
except Exception:
    print("")' 2>/dev/null)"

[[ -z "$FILE" ]] && exit 0

# Components owned by other teams that carry deliberate working-tree edits.
if grep -qE '(^|/)(vendor|third_party)/' <<<"$FILE"; then
    cat <<EOF

⚠  $FILE is in another team's component.

CLAUDE.md: read hotfixes.md BEFORE editing, staging or reverting anything here. The rules are
usually NOT uniform — one file may have to be committed and another must never be, and only
the hotfix entry knows which.

Matching hotfixes.md entries:
EOF
    # Print the entry headings plus their Remove-when lines, not the whole file.
    grep -nE '^### |^- \*\*(Where|Remove when):' "$DIR/work/hotfixes.md" 2>/dev/null \
        | sed 's/^/  /' | head -60
    echo
fi

exit 0
