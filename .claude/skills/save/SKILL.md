---
name: save
description: Save the current session state into the .claude/ working docs — sweep the conversation for loose threads, update the task's plan.md progress, append new decisions to decisions.md, log team issues to issues.md and temporary code to hotfixes.md, append a session entry to history.md, write the next-session prompt to handoff.md, then commit and push the owner's work directory. Use when the user asks to save, wrap up, checkpoint, or hand off the session.
model: sonnet
---

# Save

Persist this session into the `.claude/` working docs so the next session — or a teammate — can pick
up cleanly. The point of running this *in-conversation* is that you can see the **rationale**, which
a cold reader cannot reconstruct. Use the live conversation for the "what and why"; use git for the
mechanical state.

If arguments were passed, narrow the save to that focus (e.g. "just the teardown work"); otherwise
cover everything since the last save.

## 0. Resolve the owner — check, do not assume

Live task directories are per-owner and tracked. Decide by identity:

```bash
git config user.email
```

| Email | Directory |
|---|---|
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `.claude/work/mdube/` |
| `shorinbonsai@gmail.com` | `.claude/work/jsargant/` |

**Anything else: stop and ask.** The same table is in `.claude/hooks/session_brief.sh`,
`.claude/skills/load/SKILL.md`, `.claude/skills/park/SKILL.md` and `documentation/mdube_edits.md`.
Below, `<owner>` means whichever this resolved to; never write to the other owner's directory.

## 0.5. Work in the dedicated `main` worktree, not the branch checked out here

**Every path below is inside a separate worktree pinned to `main`, never the working tree this
session is coding in.** `CLAUDE.md`, "`.claude/work/` lives in a dedicated `main` worktree" has the
full reasoning; the short version is that `.claude/work/<owner>/` on a feature branch goes stale
the moment `main` moves, which defeats the entire point of tracking it.

```bash
MAIN_TREE="$(git rev-parse --show-toplevel)"
DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"
[[ -d "$DOCS_WT" ]] || { echo "Missing docs worktree — run: git worktree add \"$DOCS_WT\" main"; exit 1; }
cd "$DOCS_WT" && git pull
```

If the worktree doesn't exist on this machine, create it once (`git worktree add "$DOCS_WT" main`
from the main tree) and say so in the report — this is a one-time, per-machine setup, not something
to silently work around.

Everything from here — reading `plan.md`, editing `decisions.md`, the final commit and push — happens
**inside `$DOCS_WT`**. The main tree's checked-out branch is never switched, stashed, or touched.

## The files

All live under `.claude/`.

**Task-scoped**, in `.claude/work/<owner>/current/` — archived by `/done` when the task ends,
or moved to `work/<owner>/parked/<slug>/` by `/park` when it is blocked:

| File | Semantics | Holds |
|---|---|---|
| `current/plan.md` | **Edit in place** | Current objective + task list with status. Written by `/start`; `save` only updates status and appends newly-agreed work. |
| `current/plan_superseded.md` | **Append-only** | Original wording of tasks now done. Reference only. |
| `current/history.md` | **Append-only** | Session-by-session log for *this task*, newest first. |
| `current/handoff.md` | **Overwritten** | A prompt for the *next* session. Only the newest matters. |

**Persistent**, at `.claude/` — these outlive the task, because they describe the *code*, not the
work. Never archive them:

| File | Semantics | Holds |
|---|---|---|
| `decisions.md` | **Append-only** | Every choice made and why. Never edit or delete a past entry. |
| `issues.md` | **Churn list** | Work for other people, staged for the tracker. Entries leave only once filed. |
| `hotfixes.md` | **Churn list** | Temporary / band-aid code in the tree. Entries leave only once reverted. |
| `traps.md` | **Churn list** | Permanent workspace gotchas. Entries leave only when no longer true. |
| `collab.md` | **Append-only** | Cross-owner decisions, when the repo is shared. Agreed items are marked, never deleted. Skip if the file doesn't exist. |
| `collab_settled.md` | **Append-only** | The archive half, split out 2026-08-13. Closed items move here and are not edited afterwards. |

**Before appending to `collab.md`, and before finishing the save** — both added 2026-08-13 at the
joint meeting:

- **Take the next item number from the highest across *both* collab files**, never from the live one
  alone. `### 48` was used twice because two sessions each read the end of one file.
- **Report every open item with no appended reply, oldest first, at any age.** No threshold — an
  item counts from the moment it is raised, because the failure being closed is an item going quiet
  rather than an item going stale.

## 1. Gather state

- `git status --short` and `git diff --stat` for **every repo this project spans** — see `CLAUDE.md`'s
  repo layout, and don't assume the root repo is the only one.
- Report each repo's current branch (`git branch --show-current`). Read the actual branch, never
  assume.
- Read `work/<owner>/current/plan.md`, and the tops of `decisions.md`, `issues.md`, `hotfixes.md`, `traps.md`, so
  you match their format and don't duplicate existing entries.

## 2. Sweep the session for loose threads

**Do this before writing anything.** Re-read the conversation since the last save and list everything
that was **discussed but never landed**. Only an in-conversation save can do this: the docs know what
was *written*, and the failure mode is a thing that was agreed out loud and then dropped when the
conversation moved on. It is the single highest-value step here.

Look for:

- **Agreed, then diverted** — you proposed something, the user approved it, and neither of you came
  back to it. The signal is an approval ("yes", "go ahead") with no later result.
- **Found in passing** — a bug or gap you noticed while doing something else and mentioned in prose,
  but never turned into a plan task, issue, or hotfix entry.
- **Asked and unanswered** — a question you put to the user that the next message overtook.
- **Recommended, no verdict** — you advised something and the user neither took it nor refused it.
- **The user took it on** — anything they said *they* would do. Record it, or explicitly drop it;
  never silently assume it happened.
- **Corrections with reach** — a mid-session correction that invalidates something already written
  in a doc, a plan item, or an earlier claim. Fix the doc; don't just note the correction.
- **Numbers that moved** — a measurement or estimate that changed. Anything quoting the old value
  has to be updated too.
- **Work started but not finished** — a file half-edited, a check run but not acted on.
- **New traps** — anything that cost you time this session and will cost it again: a tool flag that
  must always be passed, a command that silently does the wrong thing, a path that isn't what it
  looks like. → `traps.md`.

Then dispose of every item, strictly:

- **Can be captured now** → write it into the right file in the steps below (plan task, decision,
  issue, hotfix, trap). Actually do it — don't just report it.
- **Needs the user** → **ask.** See below. **Never guess a disposition and record it as though it
  were agreed.**

Don't pad the list. If a thread genuinely resolved, leave it off. Its value is that every line on it
is real.

### Ask the open ones as a series of questions

Put the threads that need the user through `AskUserQuestion` — **one question per thread**, batched
up to 4 per call, repeating until they are all answered. Do not bury them in a prose list; a bullet
in a closing summary is easy to skim past, and these are exactly the items that vanish when the
context is cleared.

For each question:

- State the thread in the question text, with enough context to answer cold — the user may have
  discussed it an hour and several topics ago.
- Give real options, each with what it actually costs or implies. Lead with a recommendation where
  you have one, marked `(Recommended)`.
- Where the choice is "do it now / write it down / drop it", say so plainly — *drop it* is a
  legitimate answer and should be offered, not smuggled in as an afterthought.

Then act on every answer **before** finishing the save: write the resulting task, decision, issue or
hotfix entry, so the answers land in the files rather than only in the transcript.

Ask only about genuine forks. Anything you can settle by reading the repo, or that has an obvious
default, you settle yourself and mention in the closing brief.

## 3. `work/<owner>/current/plan.md` — update status

- Mark completed items. Use three states, and keep them honest:
  `[ ]` pending · `[x]` done **and verified** · `[~]` done but **NOT verified**.
- `[~]` is the important one: code that only compiles, or that ran only somewhere that doesn't
  count, is `[~]` — not `[x]`. Say what verification is still owed, and **stamp it**:
  `(unverified since <YYYY-MM-DD>)`. Age is what makes a stale `[~]` visible as stale.
- **Compress each item as you tick it — to ≤ 3 lines.** What was done, the one piece of evidence
  that verifies it, where the detail lives. The evidence itself goes to `history.md`, the reasoning
  to `decisions.md`. If the original wording is worth keeping, move it to
  `work/<owner>/current/plan_superseded.md` under a `## <item id> — superseded <YYYY-MM-DD>` heading. **Never
  leave a superseded item in `plan.md` wearing a `[ ]` checkbox** — an item that can never be ticked
  teaches everyone to skim past `[ ]`, and then a real pending item gets lost.
- Append any work that was agreed *during* this session and isn't yet on the plan — including
  anything the step-2 sweep turned up.
- Strike items that were abandoned, with a one-line reason (and a matching `decisions.md` entry).
- Do not restructure or rewrite the plan's existing items.

### Keep the plan a TASK LIST, not a record — enforce this every save

Left unenforced, `plan.md` grows without bound. In the project this template came from it reached
**1432 lines** and had to be cut in half by hand. It grew because evidence, rationale and superseded
text all accumulated in it. Every one of those belongs somewhere else, and each has a file that owns
it:

| What | Where it goes | NOT in the plan |
|---|---|---|
| What happened, measurements, tables | `work/<owner>/current/history.md` | ✗ |
| Why we chose it, what was rejected | `decisions.md` | ✗ |
| Original wording of a task now done | `work/<owner>/current/plan_superseded.md` | ✗ |
| Temporary code | `hotfixes.md` | ✗ |
| Someone else's work | `issues.md` | ✗ |

**Budgets — check them at every save, and fix on the spot:**

- **A completed item is ≤ 3 lines.** Compress it **when you tick it**, not later: what was done, the
  single piece of evidence that verifies it, and where the detail lives. Do not paste the evidence.
- **An open item is ≤ 20 lines.** What to do, the verify-by, and any constraint that would cause harm
  if forgotten. If it needs more, the reasoning goes in `decisions.md` and the plan links to it.
- **Never keep "(original text, kept for the reasoning)" blocks in the plan.** Move them to
  `work/<owner>/current/plan_superseded.md` the moment the task is done, and **never leave one wearing a `[ ]`** —
  an item that can never be ticked teaches everyone to skim past `[ ]`, and then a real pending item
  gets lost.
- **Soft cap ~600 lines.** If `plan.md` is over it, compress the biggest completed items *before*
  appending new ones. `wc -l current/plan.md` — do this check as part of the save.

**Amalgamate.** If two items describe the same work (a task and its "verification" twin, an item and
its rewrite), merge them into one and keep the number that is referenced elsewhere.

## 4. `decisions.md` — append what was chosen and why

One entry per real decision, appended at the **bottom**. Never edit a past entry: if this session
reversed an earlier decision, write a NEW entry that names and supersedes it. The reversal trail is
the value.

```markdown
## <YYYY-MM-DD> — <author> — <short title>
**Chose:** what we're doing.
**Why:** the reasoning, in the terms it was actually argued.
**Rejected:** the alternatives considered, and what ruled them out.
**Affects:** `path:line`, or the area it constrains.
**Supersedes:** <date + title of the earlier decision>   (only if applicable)
```

Only log decisions a cold reader couldn't re-derive from the code. Skip the obvious.

Drop the `— <author>` field if you work alone. Keep it if anyone else uses this `.claude/`:
the persistent docs merge with `merge=union`, which never reports a conflict, so the stamp is
the only thing that makes an accidentally duplicated entry visible.

## 5. `issues.md` — stage work for other people

Anything found this session that belongs to someone else, or is out of scope for the current work.
The file has **two tiers**, and the distinction is whether it has been root-caused.

**Parked** — noticed, not investigated. Cheap to write, so nothing is lost just because chasing it
would derail the current task. Four lines, no more:

```markdown
### <what was noticed>
- **Where:** `path` or component, as far as it's known.
- **Impact:** why it matters — who or what it breaks.
- **Noticed:** <YYYY-MM-DD>, in <what you were doing when you hit it>
```

**Ready to file** — root-caused and evidenced, written so the body pastes into the tracker without
rewriting. How issues get filed (tool, confirmation rule, project mapping) lives in `.claude/CLAUDE.md`.

```markdown
### <title — imperative, issue-ready>
- **For:** teammate / team / unassigned
- **Project:** the tracker project it belongs to
- **Filed:** not yet          ← becomes the issue URL once filed
- **Component:** `path:line` (the specific module/function)
- **Body:**
  What's wrong, the mechanism with `path:line`, evidence (rates, run IDs, measurements),
  how to reproduce, and the candidate fixes.
```

Promote parked → ready only when the investigation actually happened. **Never fabricate the
evidence fields to make something look file-ready** — an unroot-caused issue dressed as a
root-caused one wastes the assignee's time. If it's still a guess, it stays parked.

Once an entry is filed, **the tracker is the source of truth.** If its content changes afterwards,
push the change to the tracker in the same session and note that you did. `issues.md` must not
become a private fork of the tracker.

Drop entries whose **Filed** is a URL and whose issue is closed. Leave everything else.

## 6. `hotfixes.md` — track temporary code

Every band-aid, stub, sleep, hardcoded value, or workaround still in the tree. Each entry needs an
exit condition, or it lives forever:

```markdown
### <what was hacked>
- **Owner:** who put it there and who removes it. Omit if you work alone.
- **Machine:** `owner's working tree, uncommitted` · `committed — in every tree` · `branch <name>`.
  Omit if you work alone; otherwise this is what tells a reader whether it is in *their* tree.
- **Where:** `path` or symbol name — prefer function names over line numbers, they survive edits.
- **What it does:** the mechanism, if not obvious from the title. Optional.
- **Why it's a hotfix:** the problem it papers over, and why the proper fix wasn't done here.
- **Real fix:** what would make this unnecessary, and **who owns it** if it's someone else.
- **Remove when:** the concrete condition that makes it unnecessary.
- **Added:** <YYYY-MM-DD>
- **Last checked:** <YYYY-MM-DD> — set by `/done`, not by `/save`. Shows when the `Remove when:`
  condition was last assessed, so a stale entry is visible as stale.
```

Group entries under `## <theme>` headings by what unblocks them (e.g. shared infra, upstream,
someone else's work) — that's the axis on which they actually get removed, in batches.

Mark any hotfix that is currently **load-bearing** (something would break today without it) with
⚠️ in its `Remove when:`, so nobody deletes it on a tidying pass.

If a hotfix in someone else's file must never be committed, say so **in the entry**, not only in the
plan — that entry is what a future session reads before touching the file.

Remove entries whose hotfix is genuinely gone from the tree — verify by reading the file, don't
assume.

## 7. `traps.md` — permanent workspace gotchas

Distinct from hotfixes: a hotfix is *code you added and want to remove*; a trap is *how this
workspace behaves and always will*. Tool flags that must always be passed, commands that silently do
the wrong thing, paths that aren't what they look like, files that a routine command will delete.

```markdown
### <the trap, stated as the mistake it prevents>
- **Bites when:** the action that triggers it.
- **Do this instead:** the correct form.
- **Why:** the mechanism, one line.
- **Added:** <YYYY-MM-DD>
```

These belong here rather than in `handoff.md`. `handoff.md` is overwritten every save, so anything
durable parked there is deleted the moment it stops being top-of-mind.

**Verify a trap before recording it.** Traps are stated as fact and get trusted for months. Run the
reproducer and put it in the entry — in the project this came from, a `grep` trap was carried in
`handoff.md` for days with the wrong mechanism before anyone re-tested it.

## 8. `work/<owner>/current/history.md` — append a session entry

Insert `## Session <YYYY-MM-DD>: <one-line headline>` at the **top** of the session log —
after the header/goals block at the top of the file and before the previous most-recent session
section. Do not rewrite old sections; if a past section's status changed, add a
one-line `UPDATE <YYYY-MM-DD>:` note to it.

Contents: what changed (with `path:line`), what was validated vs. not, and the **git manifest** —
exact uncommitted/unpushed state per repo and branch, so nothing is lost. Keep the reasoning short
here; let `decisions.md` carry it.

## 9. `work/<owner>/current/handoff.md` — write the next-session prompt

Overwrite the whole file. This is not a summary — it is an **instruction to the next session**,
written so that pasting it is enough to resume. Address it to the agent, second person, imperative:

```markdown
# Next session — <YYYY-MM-DD>

**Machine:** `<hostname>` · saved <YYYY-MM-DD HH:MM> · <short commit SHA>

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** 2–4 sentences.

**Start here:** the single next concrete action, with the file and the command to run.

**Watch out for:** live traps — unverified `[~]` items, hotfixes that will bite, known-broken state.

**⏰ Time-sensitive:** anything dated, with absolute dates.
```

**The `Machine:` line is not decoration.** These directories are tracked so a task can be picked up
on another machine, which creates a failure the old per-person layout could not have: you, against
yourself, from two laptops. `/load` reads this line and the SHA beside it to decide whether the
handoff in front of it is the newest one, and stops rather than merging a plan file if they
disagree. Write the real `hostname` and `git rev-parse --short HEAD` **as they are while you write
it** — that names the repo state this handoff describes. It is deliberately not the SHA of the save
commit itself, which does not exist yet and would need an amend to record.

**Keep it to what is true this week.** If you are about to write something permanent here — a tool
flag that always applies, a rule about how to write issues, a path that a routine command destroys —
it belongs in `traps.md` or `CLAUDE.md` instead. This file is overwritten every save; anything
durable left in it is on a timer.

**One `Start here`.** If there is genuinely a queue, name the single next action, then list the rest
under a separate "then, in priority order" heading — so the next session cannot mistake the queue
for the instruction.

## Union-merge safety — check before you finish

If the repo is shared and `.gitattributes` sets `merge=union` on `.claude/work/*.md`, a byte-identical
line in two entries is folded together on merge and the entries interleave — silently. Every entry
you appended this save must have a **unique first and last line**: a heading carrying the author,
and a closing stamp carrying a time, e.g. `*#7 · raised <YYYY-MM-DD> <HH:MM> — <name>.*`

Run this on every file you touched, and fix anything it prints:

```bash
grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d
```

Bare labels are the usual culprit — `- **Body:**`, `- **Added:** <date>`, a trailing `---`.

## Conventions

- **Absolute dates only** — convert "today" / "tomorrow" / "last session" to real dates.
- Reference code as `path:line`. Keep it skimmable: headers and bullets, not walls of prose.
- If tooling or output behavior changed, check the run instructions in `work/<owner>/current/plan.md` and
  `work/<owner>/current/history.md` still match, and fix them if not.
- Never truncate or rewrite `work/<owner>/current/history.md` or `decisions.md`.
- **If anyone else uses this `.claude/`:** stamp every new entry in the persistent docs with an
  author, and never silently rewrite someone else's entry — raise it in `collab.md` instead.
  If this session merged, read the tail of the persistent docs before appending: `merge=union`
  can interleave two entries without reporting a conflict.

## Memory — do not use it for this project

**Do not write auto-memory files.** See `.claude/CLAUDE.md`, "Do not use the auto-memory store".

Durable project facts go in the file that owns that lifetime — `hotfixes.md`, `issues.md`,
`traps.md`, `decisions.md`, `work/<owner>/current/history.md`, `work/<owner>/current/plan.md`. A rule about *how to work* that
must be known before reading any of them goes in `CLAUDE.md`.

## 10. Commit and push `work/<owner>/` — this is part of the save

A handoff that was never pushed is not a handoff. The whole reason these directories are tracked is
so the next session can be on a different machine, and that only works if the save reaches `origin`.
**This runs inside `$DOCS_WT` (step 0.5), always against `main` — never the code branch.**

```bash
cd "$DOCS_WT"
git add .claude/work/<owner>/
git status --short .claude/work/<owner>/     # read it — nothing outside this path may be staged
git commit -m "Save <task slug> — <one line>"
git push origin main
```

**Scope it to `work/<owner>/` and nothing else.** The persistent docs (`decisions.md`, `traps.md`,
`issues.md`, `hotfixes.md`, `collab.md`) are a separate concern with their own routing, and code is
absolutely not part of a save — and code can't leak in by accident here anyway, since `$DOCS_WT` has
no uncommitted code to stage: it's a separate worktree that was never used to edit source. If the
main tree (where the code work happened) has uncommitted code, leave it uncommitted and say so in
the brief — a save must never quietly commit a half-finished source file there either.

**This is a deliberate, narrow exception to `CLAUDE.md`'s "don't commit or push unless asked."** It
covers this path, at this step, only. Agreed 2026-08-13: an unpushed handoff fails silently and is
discovered on the other machine, usually a day late, which is the exact class of failure the working
docs exist to prevent. Everything else still needs its own explicit instruction, every time.

If the push fails — no network, or `main` has moved — **say so in the brief and stop.** Do not
rebase, do not force. The files are written; the next session's `/load` will see the divergence and
report it, which is the designed behaviour rather than a failure.

## 11. Close with a brief the user can answer

End with a short summary — **not** a file-by-file changelog. Its job is to surface what is
outstanding, so nothing quietly rots between sessions. Keep it under ~20 lines.

By this point the step-2 sweep's open threads have already been **asked and answered** via
`AskUserQuestion`, and the answers written into the files. The brief reports what came of them; it
does not re-litigate them.

```markdown
**Saved:** <one line — what this session actually did>

**Settled this save:** <one line per loose thread the questions resolved, and where it landed>

**Outstanding**
- ⚠️ <N> issues unfiled: <titles>            ← only if any are `Filed: not yet`
- ⚠️ <N> hotfixes still in the tree: <the ones touched or relied on this session>
- <N> `[~]` unverified, oldest first: <what needs running, by whom, and since when>
- <blockers / open questions from plan.md>

**Next session starts at:** <the one action in handoff.md>
```

Rules for this brief:

- **Only list what's genuinely outstanding.** If nothing is unfiled and nothing is unverified, say
  so in a line. A brief that always looks the same gets skipped.
- Unfiled issues and unverified `[~]` items go **first** — the two that go stale silently.
- Don't list every hotfix every time; list the ones this session added, touched, or leaned on.
- It's a prompt for the user, so make each line answerable: name the thing, say what it's waiting
  on.

The user may reply with dispositions ("file that one", "that one can wait"). Act on them and update
the docs before finishing.

## 12. Offer to clear the context

End by asking — **never do it yourself, and never assume the answer:**

> Everything is captured in the docs. Want to `/clear` and start fresh? Next session picks up with
> `/load`.

`/clear` is a CLI command only the user can type; you cannot run it and must not try. The offer
exists because a save is the one moment when clearing is safe: the session's state lives in the
files, and `work/<owner>/current/handoff.md` is written to make the next session resumable from cold.

If the user is mid-task and plans to keep working in this session, they'll decline — that's the
expected answer as often as not. Ask once, take the answer, don't press.

### If work continues after the save, the save is stale

Declining `/clear` is normal, and a save is a snapshot, not a seal. **Any work done after the brief
is not in the docs** — and this bites in a specific way: the closing brief and `handoff.md` are
written as though the session ended there, so they can actively *mislead* the next session by
pointing at a state that has since moved on.

So, when the session continues past a save:

- **Keep updating the docs as you go.** Don't bank changes for a second save — a decision made after
  the brief still belongs in `decisions.md` when it is made.
- **Re-run `/save` before `/clear`**, or before the session actually ends. It is cheap: step 2 sweeps
  only what has happened since, and most files need no change.
- **Re-check `handoff.md` in particular.** It is the one file written entirely in the past tense
  about a "next session" that may now start from somewhere else. Verify its *Start here* is still the
  real next action and that it names no file or section the later work deleted.

The step-2 sweep catches threads *within* a save. Nothing catches work done *after* one except
saving again.

## Constraints

- **Commit and push `.claude/work/<owner>/` — and nothing else** (step 10). Every other path,
  including code, the persistent docs and the spec sheet, still needs the user's explicit
  instruction, every time.
- Never write to the other owner's `work/<owner>/` directory.
- After the brief, report per file what you added, or that it was unchanged.
