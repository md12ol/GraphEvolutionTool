---
name: load
description: Start a session on a task — resolve which one (the active task, or a parked one named by slug), read its handoff.md, plan.md, decisions.md and hotfixes.md, verify them against the actual repo state, and report where things stand before doing any work. Use at the start of a session, when resuming or unparking a task, or when the user asks where things are.
model: sonnet
---

# Load

Pick up a task. This is step 5 of the loop:

1. New task
2. `/start`
3. Work
4. `/save`
5. **`/load [slug]`** ← you are here
6. Work
7. Finished? → step 8. Blocked? → `/park <slug>`. Not finished? → step 4.
8. `/done <slug>`

`/save` wrote `handoff.md` for you. Your job is to consume it, **check it is still true**, and
report — then stop and wait. Do not start work as part of `/load`.

## 0. Resolve the owner, then resolve the task

Live task directories are per-owner and tracked. Decide the owner by identity, never by memory:

```bash
git config user.email
```

| Email | Directory |
|---|---|
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `.claude/work/mdube/` |
| `shorinbonsai@gmail.com` | `.claude/work/jsargant/` |

**Anything else: stop and ask.** The same table is in `.claude/hooks/session_brief.sh`,
`.claude/skills/park/SKILL.md` and `documentation/mdube_edits.md`; if you add an address, add it in
all four. Below, `<owner>` means whichever this resolved to. **Never read or write the other owner's
directory** — theirs is tracked so it survives their laptop, not so you can work in it.

Then resolve *which task*, in this order:

- **A slug was passed** (`/load result-object`) → that parked task. If
  `work/<owner>/parked/<slug>/` does not exist, list what does exist and stop; do not guess at a
  near match.
- **No slug, and `work/<owner>/current/plan.md` exists** → the active task. Parked tasks are not
  touched, but **name them and their `Blocked on:` lines** in the report — a parked task that has
  become unblocked is exactly what a session needs told.
- **No slug, `current/` empty, exactly one parked task** → that one. Say plainly that you unparked
  it rather than doing it silently.
- **No slug, `current/` empty, several parked** → **ask.** List each slug with its `Blocked on:`
  line, so the choice is answerable without opening anything.
- **No slug, `current/` empty, nothing parked** → there is no active task. Say so and point at
  `/start`. Do not invent one.

### Unparking is a swap, and it is done in one go

Loading a parked task while another is active means both move, because everything downstream —
`/save`, `/done`, the session brief — reads `work/<owner>/current/` and nothing else:

1. If `current/` holds a task, park it first. Run the `park` skill properly rather than moving the
   files by hand: its `/save` and its `Blocked on:` stamp are the reason the task will be resumable.
2. `git mv .claude/work/<owner>/parked/<slug>/* .claude/work/<owner>/current/`, then remove the now
   empty `parked/<slug>/`.
3. Note in the report that a swap happened and which task went the other way. A session that thinks
   it parked nothing will `/save` over the wrong plan.

Never read a parked task's plan "in place" as a shortcut. Two directories containing a live plan is
the state this whole layout exists to prevent.

## 0.4. Work in the dedicated `main` worktree, not the branch checked out here

**Every `.claude/work/` path in this skill is inside a separate worktree pinned to `main`**, never
the working tree this session is coding in — `CLAUDE.md`, "`.claude/work/` lives in a dedicated
`main` worktree" has the full reasoning.

```bash
MAIN_TREE="$(git rev-parse --show-toplevel)"
DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"
[[ -d "$DOCS_WT" ]] || { echo "Missing docs worktree — run: git worktree add \"$DOCS_WT\" main"; exit 1; }
cd "$DOCS_WT"
```

The unpark move (step 0), the divergence check (0.5) and every read in step 1 happen inside
`$DOCS_WT`. The main tree's checked-out branch is never switched or touched by `/load`.

## 0.5. Check for cross-machine divergence — before reading anything as true

Because `$DOCS_WT` is always `main`, this is no longer a "which branch" question — it's whether
*this machine's* copy of `main` in `$DOCS_WT` agrees with `origin/main`. The one failure the old
per-person layout could not have still applies: **you, against yourself, from two machines**, each
running a session and pushing to `main` without the other having pulled first.

```bash
cd "$DOCS_WT"
git status --short                  # uncommitted local work from an interrupted session
git fetch origin main --quiet
git log --oneline HEAD..origin/main # commits on origin you don't have yet
git log --oneline origin/main..HEAD # commits you have that origin doesn't — the real divergence
```

- **Behind, not ahead** → the normal case. `git pull --ff-only` and continue; this is not a
  divergence, just a worktree that hadn't synced yet.
- **Both ahead and behind** → a genuine divergence: this machine and another each committed to
  `main` without syncing. **Stop and report** — do not merge, rebase, or reset. Name both sets of
  commits and let the user decide. `handoff.md`'s `Machine:` stamp tells you which session wrote
  which side.
- **Local uncommitted changes** → this machine ended a previous session without `/save` reaching
  its push step. Report them; they are probably the real state.

`plan.md` and `history.md` are rewritten in place, so a merge of two versions is a plausible-looking
file that is nobody's plan. Stopping on real divergence is always correct here.

## 1. Read, in this order

1. `work/<owner>/current/handoff.md` — the instruction from the last session. This is the primary input.
2. `work/<owner>/current/plan.md` — the objective and task status.
3. `.claude/work/decisions.md` — read at least the most recent entries. **Do not re-litigate anything
   recorded here.** If you think a past decision is wrong, say so explicitly rather than quietly
   doing something else.
4. `.claude/work/hotfixes.md` — temporary code you might otherwise mistake for a bug, or delete.
5. `.claude/work/traps.md` — the workspace gotchas. Cheap to read, and each one is there because it
   already cost someone a session.
6. `.claude/work/issues.md` — only to notice what's already logged, so you don't re-report it.
7. `.claude/work/collab.md`, if it exists — the repo is shared. Read the **Open** items: each one
   is a decision on one side that overrides work on the other, and acting against an open item is
   how someone's work gets silently overwritten.

`work/<owner>/current/plan_superseded.md` is reference only. Don't read it on load, and never action
anything in it — it holds the original wording of tasks that are already done.

The empty-`current/` cases are all handled in step 0; by the time you are here a task is resolved.

**If the repo is shared and this session follows a merge**, read the *tail* of the persistent docs
before trusting them: they merge with `merge=union`, which never reports a conflict, so two people
editing the same entry yields both versions interleaved. A doubled or self-contradicting entry is
a merge artefact to fix, not a decision to follow.

## 2. Verify the handoff against reality

**This step runs in the main tree** (`$MAIN_TREE` from step 0.4), not `$DOCS_WT` — it's checking
the state of the code the handoff describes, not the docs. The handoff may be days old. Treat it as
a claim to check, not a fact. Confirm before relying on it:

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
- **Parked tasks** — one line each: slug and `Blocked on:`. Then say whether any of them now looks
  **unblocked** by what you verified above — a merged PR, an answered `collab.md` item. That is the
  single thing a parked task needs from a session it is not part of, and nothing else will notice.
- **A swap, if one happened** — which task you unparked and which you parked to make room.

Then **wait for the user**. `/load` orients; it does not begin work. If the next action is obvious
and small, still confirm before starting — the user may have switched priorities since the handoff
was written, which is exactly the information the docs can't have.

## Constraints

- Read-only **except for the unpark move in step 0**, which is the one thing `/load` is allowed to
  change — and only by moving whole files between `parked/<slug>/` and `current/`, never by editing
  their contents. If the docs are wrong, report it and let `/save` or the user fix it.
- Don't commit, push, or start edits. The unpark move is committed by the next `/save`, not here —
  it happens in `$DOCS_WT` and is left staged/uncommitted there until then.
- Never touch the other owner's `work/<owner>/` directory.
- Respect `CLAUDE.md`'s rule on who runs the environment. If verifying something requires a run you
  are not allowed to make, say what you need and ask the user to run it.
