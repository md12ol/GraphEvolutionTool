# Hooks

`settings.json` enables only the two **backup** hooks (`Stop`, `SessionEnd`). Three more ship here as
ready scripts, disabled by default. They turn `CLAUDE.md` rules from prose into enforcement.

`/setup` offers the ones that apply and wires them up. To do it by hand: edit the script's pattern,
add its JSON to `settings.json`, then check the file still parses —

```bash
python3 -m json.tool .claude/settings.json > /dev/null && echo OK
```

A malformed settings file disables **every** hook in it, silently.

| Script | Event | Blocks? |
|---|---|---|
| `backup_docs.sh` | `Stop`, `SessionEnd` | no — **enabled by default** |
| `block_env_commands.sh` | `PreToolUse(Bash)` | **yes**, exit 2 |
| `show_hotfixes.sh` | `PreToolUse(Edit\|Write)` | no |
| `session_brief.sh` | `SessionStart` | no |

Each is testable without a session:

```bash
echo '{"tool_input":{"command":"make deploy"}}' | .claude/hooks/block_env_commands.sh; echo "rc=$?"
echo '{"tool_input":{"file_path":"vendor/x.py"}}' | .claude/hooks/show_hotfixes.sh
.claude/hooks/session_brief.sh
```

---

## 1. `block_env_commands.sh` — the one that matters

Enforces `CLAUDE.md`'s "who runs the environment" rule. Without it that rule is a sentence an agent
may or may not honour; with it the tool call fails and the agent is told what to do instead.

⚠ **Edit `BLOCK` and `ALLOW` in the script before enabling.** The defaults (`make deploy`,
`docker compose up`, `./deploy.sh`) are examples. `ALLOW` exists for read-only commands that resemble
blocked ones — analysis scripts that only read already-generated output.

Patterns are word-boundaried, so `echo deploying` does not match `deploy`. Test both directions
before trusting it.

```json
"PreToolUse": [
  { "matcher": "Bash",
    "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/block_env_commands.sh\"" } ] }
]
```

## 2. `show_hotfixes.sh` — before editing someone else's file

Prints matching `work/hotfixes.md` entries when an edit targets a path carrying deliberate
working-tree changes. Never blocks — those edits are legitimate; the risk is making them *unaware*,
because the rules are usually not uniform across files.

⚠ **Edit the path pattern** (default: `vendor|third_party`).

```json
"PreToolUse": [
  { "matcher": "Edit|Write",
    "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/show_hotfixes.sh\"" } ] }
]
```

## 3. `session_brief.sh` — orientation without `/load`

Prints the handoff's *Start here* plus counts of open `[ ]`, unverified `[~]`, unfiled issues and
traps. No editing needed.

It does **not** replace `/load`, which verifies the handoff against the repo — a hook can't. It makes
a rotting item visible at zero cost.

```json
"SessionStart": [
  { "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/session_brief.sh\"" } ] }
]
```

---

## Merging more than one

`PreToolUse` is shared by hooks 1 and 2 — combine them into one array rather than repeating the key:

```json
"hooks": {
  "PreToolUse": [
    { "matcher": "Bash",       "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/block_env_commands.sh\"" } ] },
    { "matcher": "Edit|Write", "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/show_hotfixes.sh\"" } ] }
  ],
  "SessionStart": [ { "hooks": [ { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/session_brief.sh\"" } ] } ],
  "SessionEnd":   [ … keep the backup hook … ],
  "Stop":         [ … keep the backup hook … ]
}
```

Project-wide hooks go in `settings.json` (tracked). Personal ones go in `settings.local.json`.
