---
name: park
description: Park a task that is blocked — run /save, then move .claude/work/<owner>/current/ into .claude/work/<owner>/parked/<slug>/ so another task can start without losing this one's plan, history and handoff. Use when a task cannot proceed (waiting on the other owner, an unmerged PR, an unanswered question) and you want to work on something else.
model: sonnet
---

# Park

Set a blocked task down without losing it. `/park <slug>` saves the session, then moves
`.claude/work/<owner>/current/` to `.claude/work/<owner>/parked/<slug>/`, leaving `current/` empty
for the next `/start`.

This is not `/done`. `/done` is for finished work and archives to the shared, tracked
`work/archive/`. `/park` is for **unfinished** work that cannot proceed right now, and everything it
moves is expected to come back via `/load <slug>`.

## 0. Resolve the owner — check, do not assume

Live task directories are per-owner. Decide by identity:

```bash
git config user.email
```

| Email | Directory |
|---|---|
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `.claude/work/mdube/` |
| `shorinbonsai@gmail.com` | `.claude/work/jsargant/` |

**Anything else: stop and ask.** Do not pick the likelier one. Parking into the wrong owner's
directory is silent — the task is neither lost nor found, and it surfaces only when someone opens a
directory they did not expect to have work in.

The same table is in `.claude/hooks/session_brief.sh` and `documentation/mdube_edits.md`. If you add
an address, add it in all three.

## 1. Refuse the cases that would lose work

Check all of these **before** running `/save`, and stop with a plain report if any fails:

- **`work/<owner>/current/` has no `plan.md`** → there is no task to park. Say so; don't create one.
- **`work/<owner>/parked/<slug>/` already exists** → the slug is taken. Report what is in it (its
  objective line and `Blocked on:`) and ask for a different slug. Never merge two tasks into one
  directory, and never overwrite.
- **No slug was passed** → derive one from `plan.md`'s objective, propose it, and ask. Slugs are
  kebab-case, no date prefix — `work/archive/` uses `<YYYY-MM>_<slug>` because it is a chronological
  record, but a parked task is live and gets its date from the plan.

## 2. Run `/save` first — the whole of it

Invoke the `save` skill and let it finish, including its step-2 sweep and its questions. A park is
exactly the moment a loose thread is lost: the session ends here for this task, and the next person
to open it may be you in three weeks on a different machine.

Do not skip `/save` because "nothing changed since the last one". If that is true it costs nothing;
if it is false it is the whole point.

## 3. Stamp the handoff before moving it

`/save` has just written `handoff.md`. Add two things to it, directly under the `# Next session`
heading:

```markdown
**Machine:** `<hostname>` · parked <YYYY-MM-DD HH:MM> · <short commit SHA>
**Blocked on:** <the concrete thing that must happen first — a PR merge, a collab.md item, an answer>.
Resume with `/load <slug>` once that lands.
```

**Both lines are parsed, so the format is not free.** `session_brief.sh` greps `^**Machine:` and
`**Blocked on:**` to list parked tasks with their blockers, and `/load` reads the same `Machine:`
line for its divergence check. Keep the leading `**Machine:**` exactly as `/save` writes it — the
only difference is the word *parked* in place of *saved*. A stamp in any other shape is not an
error anyone will see; the task just lists as `no blocker recorded`.

**`Blocked on:` is the line that earns this skill.** It is what `/load` and the session brief show
when listing parked tasks, and it is the question you will actually be asking when you come back:
not "what was this" but "can I work on it yet". Write the specific unblocking event — `PR #65 and
#66 merged; collab.md #53 answered` — not "waiting on James".

If the handoff's own **Start here** was completed during this session's save, say so on the same
line rather than leaving an instruction that has already been carried out.

## 4. Move, don't copy

```bash
mkdir -p .claude/work/<owner>/parked/<slug>
git mv .claude/work/<owner>/current/* .claude/work/<owner>/parked/<slug>/
```

Move **every** file, including `plan_superseded.md` if it exists. `current/` is left empty, not
deleted — `/start` expects to find it.

`git mv` rather than `mv` because these directories are tracked as of 2026-08-13. If a file is
untracked (a plan written before its first save), plain `mv` it and let the next commit pick it up.

## 5. Commit and push

Same rule as `/save`: `work/<owner>/` is committed and pushed to `main` as the last step, because a
parked task that exists on one laptop defeats the reason these directories are tracked. Commit
message: `Park <slug> — blocked on <the short form>`.

The routing table in `CLAUDE.md` puts `.claude/work/` on the direct-push path, so this does not need
a branch or a PR.

## 6. Report

Three lines, no more:

- What was parked, and where it now lives.
- What it is blocked on, and the command that resumes it (`/load <slug>`).
- That `current/` is empty and `/start` is the next move.

## Constraints

- Never park into an existing slug, and never merge two tasks into one directory.
- Never park a **finished** task — that is `/done`, and it archives rather than parks.
- Never edit the parked task's `plan.md` after moving it. It resumes exactly as it was left.
- Do not touch the other owner's `work/<owner>/` directory for any reason.
