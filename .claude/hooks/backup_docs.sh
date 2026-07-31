#!/usr/bin/env bash
# Back up this project's .claude/ working docs.
#
# Insurance for the case where .claude/ is NOT tracked by the project's own git. If you do track it
# (recommended — see .claude/README.md), this is belt-and-braces and you can drop the hooks that
# call it.
#
# Snapshots land in ~/.claude-backups/<project>/<YYYY-MM-DD>/ — one directory per day, overwritten
# within the day, pruned beyond RETAIN_DAYS. The project name is derived from the directory that
# contains .claude/, so nothing needs configuring per project.
#
#   .claude/hooks/backup_docs.sh            # throttled; safe from a Stop hook after every turn
#   .claude/hooks/backup_docs.sh --force    # copy regardless of throttle (use for SessionEnd)

set -euo pipefail

# This script lives in .claude/hooks/ — CLAUDE_DIR is one level up.
CLAUDE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="$(basename "$(dirname "$CLAUDE_DIR")")"
DEST_ROOT="${CLAUDE_DOCS_BACKUP_DIR:-$HOME/.claude-backups/$PROJECT}"
RETAIN_DAYS="${CLAUDE_DOCS_BACKUP_RETAIN:-14}"

# Loose files at the .claude/ root. settings*.json are included deliberately — settings.json holds
# the hooks that run this script, so without it a restore cannot restore its own trigger.
DOCS=(
    CLAUDE.md
    README.md
    settings.json
    settings.local.json
)

# Directories copied whole. work/ is all the accumulated project state — the actual point of the
# backup. skills/ and hooks/ are machinery: cheap to include, annoying to rebuild by hand.
DIRS=(work skills hooks)

DEST="$DEST_ROOT/$(date +%F)"

# Throttle: a Stop hook fires after every assistant turn, so skip if the snapshot is already fresh.
THROTTLE_SECS="${CLAUDE_DOCS_BACKUP_THROTTLE:-900}"
if [[ "${1:-}" != "--force" && -d "$DEST" && "$THROTTLE_SECS" -gt 0 ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$DEST") ))
    if (( age < THROTTLE_SECS )); then
        echo "backup skipped — snapshot is ${age}s old (throttle ${THROTTLE_SECS}s)"
        exit 0
    fi
fi

mkdir -p "$DEST"

copied=0
for f in "${DOCS[@]}"; do
    if [[ -s "$CLAUDE_DIR/$f" ]]; then
        cp -p "$CLAUDE_DIR/$f" "$DEST/$f"
        copied=$((copied + 1))
    fi
done

for d in "${DIRS[@]}"; do
    if [[ -d "$CLAUDE_DIR/$d" ]]; then
        rm -rf "${DEST:?}/$d"
        cp -rp "$CLAUDE_DIR/$d" "$DEST/$d"
    fi
done

# Prune old daily snapshots.
if [[ -d "$DEST_ROOT" ]]; then
    find "$DEST_ROOT" -mindepth 1 -maxdepth 1 -type d -mtime "+$RETAIN_DAYS" -exec rm -rf {} + 2>/dev/null || true
fi

echo "backed up $copied files + $(printf '%s ' "${DIRS[@]}")to $DEST"
