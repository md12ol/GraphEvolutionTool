---
name: load
description: Start a session on the current task — read .claude/work/current/handoff.md, plan.md, decisions.md and hotfixes.md, verify them against the actual repo state, and report where things stand before doing any work. Use at the start of a session, when resuming a task, or when the user asks where things are.
---

# Load

Pick up the current task. This is step 5 of the loop:

1. New task
2. `/start`
3. Work
4. `/save`
5. **`/load`** ← you are here
6. Work
7. Finished? → step 8. Not finished? → step 4.
8. `/done <slug>`

`/save` wrote `work/current/handoff.md` for you. Your job is to consume it, **check it is still true**,
and report — then stop and wait. Do not start work as part of `/load`.

## 1. Read, in this order

1. `.claude/work/current/handoff.md` — the instruction from the last session. This is the primary input.
2. `.claude/work/current/plan.md` — the objective and task status.
3. `.claude/work/decisions.md` — read at least the most recent entries. **Do not re-litigate anything
   recorded here.** If you think a past decision is wrong, say so explicitly rather than quietly
   doing something else.
4. `.claude/work/hotfixes.md` — temporary code you might otherwise mistake for a bug, or delete.
5. `.claude/work/traps.md` — the workspace gotchas. Cheap to read, and each one is there because it
   already cost someone a session.
6. `.claude/work/issues.md` — only to notice what's already logged, so you don't re-report it.

`work/current/plan_superseded.md` is reference only. Don't read it on load, and never action anything in
it — it holds the original wording of tasks that are already done.

If `work/current/` is empty or has no `plan.md`, there is **no active task**. Say so and point at
`/start`. Do not invent one.

## 2. Verify the handoff against reality

The handoff may be days old. Treat it as a claim to check, not a fact. Confirm before relying on it:

- **Branches.** `git branch --show-current` for every repo the work spans — see `CLAUDE.md`'s repo
  layout. The handoff's manifest may name a branch you are no longer on.
- **Working tree.** `git status --short` per repo. Files may have been committed, reverted, or
  further edited since. Conflicts recorded as unresolved may now be resolved — or vice versa.
- **Specific claims.** If the handoff says a file is in a particular state ("unresolved conflict",
  "stub", "not yet written"), open it and confirm. Cheap, and it's the class of thing that silently
  goes stale.
- **`[~]` items in the plan.** These are done-but-unverified. Check whether the verification named
  in `Verify by:` has since happened. Never promote `[~]` to `[x]` yourself on inference — only on
  evidence, or on the user telling you they ran it.

Where reality and the docs disagree, **the repo wins**. Report the discrepancy; don't silently
patch the docs to match, and don't silently follow the stale version.

## 3. Report and stop

Give the user a short brief:

- **Where things stand** — 2–4 sentences, from the handoff, corrected by what you verified.
- **Anything that changed since the handoff was written** — explicitly, or "nothing changed".
- **Start here** — the next concrete action the handoff names, or the first `[ ]` item in the plan.
- **Live traps** — unverified `[~]` items (**oldest first, with their age**), load-bearing hotfixes
  in the code you're about to touch, known-broken state.
- **Blockers** — unanswered open questions in the plan that gate the next action.

Then **wait for the user**. `/load` orients; it does not begin work. If the next action is obvious
and small, still confirm before starting — the user may have switched priorities since the handoff
was written, which is exactly the information the docs can't have.

## Constraints

- Read-only. `/load` changes no files, including the docs — if they're wrong, report it and let
  `/save` or the user fix it.
- Don't commit, push, or start edits.
- Respect `CLAUDE.md`'s rule on who runs the environment. If verifying something requires a run you
  are not allowed to make, say what you need and ask the user to run it.
