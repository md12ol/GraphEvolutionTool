# `.claude/` — how work is tracked in this project

Claude Code sessions are stateless. This directory is the memory: what we're building, what was
decided, what's temporarily hacked, and where the last session stopped. Four slash commands
maintain it.

You don't have to read the rest of this file to use it. The short version:

```
/setup   once per project, right after install — fills in CLAUDE.md
/start   at the beginning of a piece of work
/save    last thing before you stop, every session
/load    first thing when you come back
/done    when the work is finished
```

## The loop

```
  /setup  ── once per project ──┐
                                ▼
  new task ──▶ /start ──▶ work ──▶ /save ──┐
                            ▲              │
                            └── /load ◀────┘   (once per session)
                                  │
                            finished? ──▶ /done <slug> ──▶ archive/
```

- **`/setup`** runs once, ever. It inspects the repo, asks what it can't infer — above all *which
  commands you run yourself and the agent must not* — and turns the template's `FILL IN` blocks into
  this project's rules. If `CLAUDE.md` has no `FILL IN` blocks left, it's already done.

- **`/start`** agrees the objective and writes `work/current/plan.md` **before any code**. It refuses to
  run if there's an unfinished task in `work/current/`.
- **`/save`** is the important one. It re-reads the session for things that were *discussed but
  never landed* — agreed then diverted, noticed in passing, asked and unanswered — and asks you
  about the ones it can't settle. Then it updates every doc and writes the next-session prompt.
- **`/load`** reads that prompt and **checks it against the repo** before trusting it. Docs go
  stale; where they disagree with the code, the code wins.
- **`/done`** settles every loose end — unfiled issues, hotfixes whose removal condition is now
  met, unverified items — then archives the task.

## The files

**Task-scoped** — `work/current/`, archived by `/done`:

| | |
|---|---|
| `work/current/plan.md` | objective + task list. **A task list, not a record** — kept under ~600 lines |
| `work/current/plan_superseded.md` | original wording of finished tasks. Reference only |
| `work/current/history.md` | append-only session log for this task |
| `work/current/handoff.md` | the next-session prompt. Overwritten every save |

**Persistent** — these describe the *code*, so they outlive any one task:

| | |
|---|---|
| `decisions.md` | append-only: what was chosen and why, including reversals |
| `issues.md` | work belonging to other people, staged for the tracker |
| `hotfixes.md` | temporary code in the tree, each with a `Remove when:` |
| `traps.md` | permanent workspace gotchas |

`CLAUDE.md` holds the rules themselves and is loaded into every session automatically.

## The three task states

```
[ ]  pending
[~]  done but NOT verified      ← the one that matters
[x]  done AND verified
```

`[~]` exists because "it compiles" and "it works" are different claims, and conflating them is the
most expensive mistake this system is designed to prevent. Nothing is promoted to `[x]` on
inference — only on evidence, or on you saying you ran it. Every task carries a `Verify by:` line
naming what would prove it.

## Conventions worth knowing

- **Absolute dates only.** "Last session" means nothing to a cold reader three weeks later.
- **Supersede, don't overwrite.** When a rule or decision changes, the old one is struck through and
  the new one dated beside it. The reversal trail is usually worth more than the tidy version.
- **One home per fact.** What happened → `history.md` · why → `decisions.md` · temporary code →
  `hotfixes.md` · someone else's problem → `issues.md` · workspace gotcha → `traps.md`. Duplication
  across files is how half of them go quietly wrong.

## Backups

`backup_docs.sh` snapshots this directory to `~/.claude-backups/<project>/<date>/`, fired by hooks
in `settings.json`. **If `.claude/` is tracked by this project's git, you don't need it** — delete
the two hooks. It exists for the case where it isn't, and a same-disk daily copy is a weak
substitute for version control.

---

*Installed from the `.claude` template. `install.sh --update` refreshes the skills without touching
anything project-specific.*
