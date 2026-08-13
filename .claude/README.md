# `.claude/` — how work is tracked in this project

Claude Code sessions are stateless. This directory is the memory: what we're building, what was
decided, what's temporarily hacked, and where the last session stopped. Five slash commands
maintain it.

You don't have to read the rest of this file to use it. The short version:

```
/setup   once per project, right after install — fills in CLAUDE.md
/start   at the beginning of a piece of work
/save    last thing before you stop, every session
/load    first thing when you come back — /load <slug> resumes a parked task
/park    set a blocked task down without losing it
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
                                  │
                            blocked?  ──▶ /park <slug> ──▶ parked/<slug>/
                                                              │
                                            /load <slug> ◀────┘
```

- **`/setup`** runs once, ever. It inspects the repo, asks what it can't infer — above all *which
  commands you run yourself and the agent must not* — and turns the template's `FILL IN` blocks into
  this project's rules. If `CLAUDE.md` has no `FILL IN` blocks left, it's already done.

- **`/start`** agrees the objective and writes `current/plan.md` **before any code**. It refuses to
  run if there's an unfinished task in `current/`.
- **`/save`** is the important one. It re-reads the session for things that were *discussed but
  never landed* — agreed then diverted, noticed in passing, asked and unanswered — and asks you
  about the ones it can't settle. Then it updates every doc, writes the next-session prompt, and
  commits and pushes your work directory so the next session can be on another machine.
- **`/load`** reads that prompt and **checks it against the repo** before trusting it. Docs go
  stale; where they disagree with the code, the code wins. `/load <slug>` resumes a parked task.
- **`/park <slug>`** saves, then sets a **blocked** task down in `parked/<slug>/` with a
  `Blocked on:` line, so you can start something else without losing it.
- **`/done`** settles every loose end — unfiled issues, hotfixes whose removal condition is now
  met, unverified items — then archives the task. It refuses a parked task; resume it first.

## The files

**Task-scoped** — `work/<owner>/current/`, archived by `/done`, or parked by `/park` into
`work/<owner>/parked/<slug>/`. `<owner>` is `mdube` or `jsargant`, resolved from
`git config user.email` — never assumed:

| | |
|---|---|
| `current/plan.md` | objective + task list. **A task list, not a record** — kept under ~600 lines |
| `current/plan_superseded.md` | original wording of finished tasks. Reference only |
| `current/history.md` | append-only session log for this task |
| `current/handoff.md` | the next-session prompt, with a `Machine:` stamp. Overwritten every save |

**Persistent** — these describe the *code*, so they outlive any one task:

| | |
|---|---|
| `decisions.md` | append-only: what was chosen and why, including reversals |
| `issues.md` | work belonging to other people, staged for the tracker |
| `hotfixes.md` | temporary code in the tree, each with a `Remove when:` and an `Owner:` |
| `traps.md` | permanent workspace gotchas |
| `collab.md` | cross-owner decisions between Michael and James |

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

## Two of us use this

`.claude/` is tracked in the repo and used by both owners on their own machines. Full rules in
`CLAUDE.md`, "Two people, one `.claude/`". The short version:

- **Don't run `/setup`.** It rewrites `CLAUDE.md` from the template and would destroy it. On a
  fresh clone, start with `/load`. Personal settings → `settings.local.json` (gitignored).
- **Stamp every entry with an author.** The persistent docs merge with `merge=union`
  (`/.gitattributes`), so our appends never conflict — but union merge never conflicts about
  *anything*, including two edits to the same entry. The stamp is what makes a silent duplicate
  visible. Read the tail of those files after a merge.
- **Check `Owner:` in `hotfixes.md`.** An uncommitted hotfix of theirs is not in your tree.
- **Hook and `settings.json` changes go through a PR, both ways.** They execute on the other
  person's machine at session start, on their next pull, without them reading the diff.
- **`work/<owner>/` is yours alone but tracked; `work/archive/` is shared**, so finished tasks reach
  both. Tracked live tasks are what let you pick one up on another machine — which makes the
  remaining conflict *you against yourself*, from two laptops. `handoff.md` carries a `Machine:`
  stamp for exactly that, and `/load` stops and reports rather than merging a plan file.
- **`[x]` is per-machine.** Never promote someone else's `[~]` because their notes read as done.
- **One-time setup: a dedicated `main` worktree.** `work/<owner>/` and the persistent docs are read
  and written from `../<repo-name>-docs`, a linked worktree pinned to `main` — `git worktree add
  ../<repo-name>-docs main` from the repo root, once, on each machine. Fixes a real bug (a feature
  branch's copy of `current/`/`parked/` going stale the moment `main` moves); full reasoning in
  `CLAUDE.md`, "`.claude/work/` lives in a dedicated `main` worktree", and `collab.md` #58.

## Backups

`backup_docs.sh` snapshots this directory to `~/.claude-backups/<project>/<date>/`, fired by hooks
in `settings.json`. **If `.claude/` is tracked by this project's git, you don't need it** — delete
the two hooks. It exists for the case where it isn't, and a same-disk daily copy is a weak
substitute for version control.

---

*Installed from the `.claude` template. `install.sh --update` refreshes the skills without touching
anything project-specific.*
