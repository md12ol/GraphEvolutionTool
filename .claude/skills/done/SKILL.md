---
name: done
description: Close out the finished task — run a final save, then archive .claude/work/current/ to .claude/work/archive/<YYYY-MM>_<slug>/ and start a clean current/. Use when the user says a task is done, finished, wrapped up, or wants to start a new task.
model: sonnet
---

# Done

Close out the current task. `/save` checkpoints work *within* a task; `/done` ends one and clears
the desk for the next.

**This should fire regularly.** An empty `archive/` next to a long-running `work/current/` means tasks are
being merged into a program that never closes — the plan and history then grow past the point where
they fit in context, and every session pays to re-read them. If the objective has grown to the point
that `/done` can never pass its gate, the right move is to close the part that *is* finished and
`/start` the rest as a new task.

## The argument

The argument is normally the **archive slug** — the name that follows `<YYYY-MM>_` in the archive
directory. `/done api-migration` → `.claude/work/archive/2026-07_api-migration/`.

Normalize it: lowercase, spaces and underscores to hyphens, strip anything that isn't
`[a-z0-9-]`. `/done "API Migration"` → `api-migration`.

**Unless it clearly isn't a slug.** Read the argument for intent before treating it as a name:

- Contains its own year-month (`2026-07_api`, `july api work`) → the user is giving the full
  directory name or a date. Don't double-prefix.
- A path or an existing archive directory → they mean *that* directory; ask before writing into it.
- A sentence or an instruction (`just the teardown work`, `don't archive yet, only save`) → that's
  scope or a directive, not a name. Follow the instruction, and derive the slug from `plan.md`'s
  objective instead.
- Empty → derive the slug from the `# Plan —` objective line in `work/current/plan.md`, and **show it to
  the user for confirmation before creating the directory.**

When in doubt, say what slug you're about to use and why, then proceed.

## What moves and what stays

The test is: does the file describe **the work** or **the code**?

| Stays at `.claude/` | Why |
|---|---|
| `CLAUDE.md` | project rules |
| `decisions.md` | most entries constrain the codebase, not just this task |
| `hotfixes.md` | the band-aids are still **in the tree** after the task ends |
| `issues.md` | unfiled work doesn't stop existing |
| `traps.md` | the workspace still behaves that way |
| `collab.md` | coordination outlives any one task; **Agreed** items are never deleted |

| Archives with the task (`work/current/`) | Why |
|---|---|
| `plan.md` | tasks for one objective, dead once met |
| `plan_superseded.md` | the original wording of those tasks |
| `history.md` | the session log *of this task* |
| `handoff.md` | a prompt to resume a task that's over |

## Steps

**1. Run `/save` first.** Full save, no shortcuts — this is the last chance to capture rationale
from the live conversation. Everything below assumes the docs are current.

**2. Check the task is actually finished.** Read `work/current/plan.md`:

- Any `[ ]` pending or `[~]` unverified items? **Stop and list them.** Ask whether they are done,
  abandoned, or moving to the next task. Do not archive over unfinished work — `[~]` especially,
  since that's work that only *looks* done.
- Unanswered **Open questions**? Surface them the same way.

**3. Sweep the persistent files** before they carry forward:

- `hotfixes.md` — two passes per entry:
  1. **Is the code still there?** Read the file and confirm. Delete entries whose hotfix is gone.
  2. **Is the `Remove when:` condition met?** Check what you can actually check — has the upstream
     fix landed, has the owner's work shipped, does the symptom still reproduce. Set
     `**Last checked:** <YYYY-MM-DD>` on every surviving entry, and say what you based it on.
     If you cannot verify a condition (it depends on someone else's repo, or on a run you can't
     make), write `Last checked: <date> — could not verify, needs <who/what>` rather than implying
     you checked. **Never mark a condition met on inference.** Removing a load-bearing hotfix on a
     wrong guess breaks everything downstream.

  Then list to the user: entries whose condition now looks met (candidates for deletion), and
  entries not verifiable for more than ~2 task cycles (going stale).
- `issues.md` — anything still `Filed: not yet` gets listed to the user. This is the moment to file
  them; once the task is archived, nobody looks again. Include parked entries: ask whether each is
  still worth keeping or has been overtaken.
- `traps.md` — drop any entry that is no longer true (the tool was fixed, the path changed).
- `collab.md`, if it exists — move anything settled during this task into **Agreed** with its date,
  and flag any Open item this task's outcome has now overtaken. Never delete an item.
- `decisions.md` — append a `## Task complete: <slug> — <YYYY-MM-DD>` marker so later entries are
  attributable to the right task.

**4. GATE — do not archive until everything outstanding is dispositioned.**

This is a hard stop. `/done` is the last moment anyone looks at this task's loose ends; once
`work/current/` is archived, unfiled issues and undocumented hotfixes are effectively lost.

Collect everything outstanding and present it as a numbered list the user answers:

- Issues still `Filed: not yet` (both tiers).
- Hotfixes whose `Remove when:` condition now looks met, and any added during this task.
- `[ ]` and `[~]` items still in `plan.md`.
- Unanswered open questions.

**The gate is acknowledgment, not resolution.** Every item needs an explicit disposition, and
these are all valid answers:

- *File it now* — do it, record the URL in `Filed:`.
- *Carry forward* — it stays in the persistent file for the next task. Say so in the archive README.
- *Drop it* — remove the entry, and note why in `decisions.md`.
- *Already handled* — verify, then remove.

A hotfix blocked on someone else's work is **carried forward**, not a blocker — otherwise no task
could ever close while an upstream fix is pending.

**Do not proceed to step 5 (archive) until every item has an answer.** If the user doesn't respond to
the list, stop there — a partial `/done` that saved but didn't archive is fine and recoverable; an
archive that swallowed unresolved work is not. Never disposition an item on the user's behalf.

**5. Archive.** Create `.claude/work/archive/<YYYY-MM>_<slug>/` using **today's** year-month. If the
directory exists, do not overwrite — append `-2`, or ask. Then plain `mv` of `work/current/plan.md`,
`work/current/plan_superseded.md`, `work/current/history.md`, `work/current/handoff.md` into it.
⚠ **`plan_superseded.md` is easy to miss** — it is created lazily by `/save`, so it is absent from
some tasks and present in others. Leaving it behind blocks the next `/start`, which refuses to run
while `work/current/` is non-empty.

Write a short `README.md` in the archive directory: the objective, dates spanned (first and last
session in `history.md`), the outcome in 2–3 sentences, and any hotfixes or issues left behind that
outlived the task.

**6. Leave `work/current/` empty.** Do **not** write a stub `plan.md` or seed `history.md` — `/start`
scaffolds the next task, and an empty directory makes it obvious there isn't one.

**7. Land the close-out on `main` — and check where you are standing first.** Added 2026-08-12,
Michael. The Constraints section already said the close-out belongs on `main`; it never said how to
get there, and the sweep runs at the end of a task, which is exactly when the task's own code branch
is the checked-out one. Committing there puts the archive on the feature branch, where it waits for
someone else's review — the stall the two-independent-tracks rule exists to prevent.

Ask for the OK first (Constraints below — every time, no exceptions), then:

```bash
git checkout main && git pull          # even if you think you are already on it
git add .claude && git commit          # archive, decisions marker, hotfix stamps, traps
git push origin main
```

The docs are staged from `.claude/` only. **Never** add the task's source changes here — they
belong to their own branch and PR, and the two tracks stay separate.

**Then delete the task's branch, but only once it is merged — and expect the remote copy to be
gone already.** Nobody merges their own PR here, so by the time you close your own task the other
owner has usually merged it and deleted the remote branch in the same breath. **The copy that
reliably survives is the local one**, and it survives on *your* machine only, which is why this
belongs in `/done` rather than in the merge snippet. A `remote ref does not exist` error here is
the normal case, not a fault:

```bash
git fetch --prune                                 # drop remote-tracking refs already deleted
git branch --merged main | grep -qx "  <branch>" || echo "NOT merged — stop"
git branch -d <branch>                            # the copy that is usually still here
git push origin --delete <branch>                 # no-op if they already deleted it; ignore the error
```

Two ways to end up not deleting, and both are reported rather than fixed:

- **Not merged.** `/done` legitimately runs while the PR is still open (`collab.md` #28), and
  deleting an open PR's branch closes it unmerged — destructive, and nobody asked for it. Leave
  both copies, name the branch in the report, say the deletion is waiting on its PR.
- **Already fully gone.** Say that too, in one clause. It means the merge path did its job.

GitHub's `delete_branch_on_merge` will not clean up later either, because this repo merges locally;
see `traps.md`, `auto-delete-does-not-fire-on-a-locally-merged-pr`.

**8. Report.** Say what was archived and where, the disposition of every item from the gate,
which hotfixes and issues carried forward into the next task, whether the task's branch was deleted
or is still waiting on its PR, and that the next step is `/start`.

## Constraints

- **`mv`, never delete.** If the archive directory can't be created, stop and say so rather than
  proceeding.
- ~~Do not commit or push.~~ **Corrected 2026-08-10 — Michael.** The old wording contradicted
  `CLAUDE.md`, which says the `/done` sweep "commits and pushes straight to `main` right then", and
  left the archive sitting unpushed in one person's working tree. **The close-out belongs on
  `main`** — the task-complete marker in `decisions.md`, `hotfixes.md`'s `Last checked` stamps,
  `traps.md` updates, and the archive directory itself. `work/archive/` is tracked precisely so a
  finished task's record reaches the other owner; left unpushed it reaches nobody.

  **But ask first — every time.** At step 7, say what you are about to commit and push
  and wait for an explicit OK. `CLAUDE.md` is unambiguous that "every commit, push and PR needs its
  own explicit instruction, each time, no matter what the plan says", and a skill step reading
  "push" is exactly what makes that rule look satisfied when it is not. Reaching the end of `/done`
  is not the instruction; the user saying so is. Do not treat a previous task's approval as
  standing.

  Two things that are *not* in question once they say go: this is docs only and needs **no PR** (see
  `CLAUDE.md`'s routing table), and **the task's own code still goes through its branch and PR** —
  two independent tracks, so the close-out is never bundled into the code branch or held for
  someone's review.
- If the user's argument said to save but not archive, do step 1 only and stop.
