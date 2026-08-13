---
name: makeAgenda
description: Turn .claude/work/collab.md into a meeting agenda at .claude/work/meetings/<YYYY-MM-DD>.md — one section per unsettled item, each with a title, a status, a proportionate brief, and click-to-answer questions. Rerunnable — a second call folds in items raised since the first without touching anything a human edited. Use when preparing for a joint meeting, or when new collab items land before one starts.
---

# Make agenda

Read `collab.md`, decide what actually needs the two owners in a room, and write it out as a file
that `/startMeeting` can walk. The agenda is a **derived document** — `collab.md` stays the source
of truth, and this skill never edits it.

**This is the judgement-heavy skill in the set.** The value is not in listing the items; it is in
classifying them correctly and writing questions that can be answered by clicking. An agenda that
statuses a settled item as `Decide` wastes the meeting's scarcest resource, and one that writes a
question nobody can answer without going back to the source has failed at its only job.

## 0. Work in the dedicated `main` worktree, not the branch checked out here

`collab.md` and `meetings/` are both under `.claude/work/`, so both live in the worktree pinned to
`main`. A feature branch's copy of `collab.md` goes stale the moment `main` moves — which is exactly
how a meeting gets prepared from a file missing the last three items raised.

```bash
MAIN_TREE="$(git rev-parse --show-toplevel)"
DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"
[[ -d "$DOCS_WT" ]] || { echo "Missing docs worktree — run: bash .claude/scripts/setup_docs_worktree.sh"; exit 1; }
cd "$DOCS_WT" && git pull
```

Everything below happens inside `$DOCS_WT`. The main tree's checked-out branch is never switched,
stashed or touched.

**Record the SHA you read `collab.md` at** — `git rev-parse --short HEAD`. It goes in the agenda
header and it is what a rerun diffs against.

## 1. Resolve the meeting date and the target file

`/makeAgenda [YYYY-MM-DD]`. With no argument, use today's date.

Target: `.claude/work/meetings/<YYYY-MM-DD>.md`. **No owner in the path** — a meeting belongs to
both owners, the same reasoning that keeps `work/archive/` shared.

Then branch on what is already there:

| State of the target file | What this run is |
|---|---|
| Does not exist | **First pass** — build the whole agenda. Go to §2. |
| Exists, header says `Status: prepared` | **Rerun** — fold in what is new. Go to §6. |
| Exists, header says `Status: in progress` | **Stop.** A meeting is being walked right now. Say so, name the item the cursor is on, and offer to append new items to the end of the agenda without touching anything before the cursor. |
| Exists, header says `Status: closed` | **Stop.** That meeting is over and `/endMeeting` may already have acted on it. Offer a new date instead. |

Never silently overwrite an agenda file. Everything about this skill's rerun behaviour exists
because the first pass is cheap and a lost decision is not.

## 2. Read every item in `collab.md` — all of it, in file order

```bash
grep -n '^### [0-9]' .claude/work/collab.md
```

**Do not trust the `## Open` / `## Settled` headings to tell you what is open.** They stopped being
accurate around item #48 (this is collab item #59), and items are physically interleaved out of
numeric order. Position in the file is not evidence of status; only the item's own text is.

Read each item **in full**, including every appended reply and addendum. The last stamped block in
an item is usually what decides its status, and it is frequently the opposite of what the opening
line asks for — an item raised as `FYI` may have been upgraded to `ACKNOWLEDGE` in an amendment, and
an item raised as a question may already have been answered and closed in a reply.

Watch for these, and report each one in the summary rather than fixing it:

- **Duplicate item numbers.** `uniq -d` will not catch one, because only the heading number
  collides and the bodies differ. Compare the numbers from the `grep` above against a plain count.
- **A reference to `#N` that means a GitHub issue, not a collab item.** These collide and the
  distinction is almost never spelled out. If it is ambiguous, quote both readings in the brief.
- **An item whose ask was never answered.** That is the failure the agenda exists to fix — status it
  by the ask, not by the silence.

## 3. Classify every item into one of six statuses

The status drives everything downstream: the order, the length of the brief, and whether questions
are asked at all. Get it right and the meeting runs itself.

| Status | Test |
|---|---|
| **Decide** | The item asks something, and no answer has been appended. Nothing proceeds without a joint call |
| **Ratify** | Both owners have stated positions **and they agree** — it needs a formal yes to become binding. Sheet amendments almost always land here, since the sheet changes only at a meeting |
| **Acknowledge** | One owner changed something that binds the other's practice. No decision, but a confirmed read is the whole point of the item |
| **FYI** | Record only. No ask, or an ask already discharged |
| **Park** | Explicitly deferred by its author, or blocked on unbuilt work. The meeting confirms it stays parked rather than reopening it |
| **Close** | Already resolved inside the thread. The only remaining action is moving it to Settled |

Two rules that are easy to get wrong:

- **`Ratify` is not `Decide`.** An item where the other owner has written "agreed, but this is my
  position not a settled amendment" is `Ratify`. Statusing it `Decide` invites the meeting to
  re-argue something both parties already conceded, which is the most expensive thing an agenda can
  cause.
- **`Close` still needs a line in the agenda.** It is the confirmation that it may be moved to
  Settled, and that confirmation is a decision `/endMeeting` acts on. Never drop a Close item for
  brevity.

**Also detect blocked-on relationships.** An item that says "waits for #N", or whose answer is
determined by another item's answer, gets an explicit `Blocked on #N` marker and is ordered after
it. Build the map even where the item does not state it — if item A's options are worded as
"follows if B is (b) or (c)", that is a dependency.

## 4. Order the agenda

1. **Blockers first** — any item that gates two or more others, hardest thinking while everyone is
   fresh. An item that governs how the meeting's own outcomes get recorded belongs here too.
2. **Decide**
3. **Ratify**
4. **Park**
5. **Acknowledge**
6. **Close / FYI** — as a rapid-fire block at the end, two or three lines each.

## 5. Write the file

Header, then one section per item, then the consolidated checklist. Use this shape exactly —
`/startMeeting` and `/endMeeting` both parse it.

```markdown
# Joint meeting — <YYYY-MM-DD>

**Present:** Michael, James
**Source:** `.claude/work/collab.md` on `main` at `<short SHA>`, items **<lo>–<hi>**.
**Status:** prepared
**Agenda order:** blockers first, then Decide → Ratify → Park → Acknowledge → Close/FYI.
```

Then the "How this file is used" block, the status table, and the blocked-on map. Then per item:

```markdown
## #<N> — <one-line title>

**Status: <Decide|Ratify|Acknowledge|FYI|Park|Close>.** *<one line: why, or what it blocks>*

<the brief>

**Q1. <question>?**
- **(a) <label>** — <what it means and what it costs>
- **(b) <label>** — <…>

### Decisions & doc changes
_(to fill in during the meeting)_
```

**Brief length is set by status, and it is a budget not a target:**

- `FYI`, `Close` — **1–2 sentences.** What it was, and that it is resolved.
- `Acknowledge` — **2–4 sentences.** What changed, and specifically what the other owner has to do
  differently.
- `Decide`, `Ratify`, `Park` — **4–6 sentences.** State the question, both sides of it fairly, the
  evidence either side rests on, and what it costs to get wrong. **Keep the measured numbers** —
  "135 sheet references in 10,251 lines", "1.20 → 1.30 → 1.27 → 1.20" — because a number is what
  turns a preference into a decision. Drop the prose that argues; keep the fact that decides.

**Questions are the deliverable.** They are what makes the meeting clickable, so:

- **2–4 options each, mutually exclusive**, each a real position someone could hold. Never include a
  filler option to reach three.
- **Every option says what it costs**, not just what it is. "Keep it" is not an option;
  "Keep it — the badges carry the honesty, and nothing changes" is.
- **Mark the raiser's stated preference** where they expressed one, as `*(James's preference.)*`.
  It is information, not a thumb on the scale, and it speeds the common case where the other owner
  simply agrees.
- **Never write a question whose answer is already in the item.** If both owners agreed in the
  thread, that is `Ratify` with a single "ratify as written?" question, not four fresh ones.
- **Two or three questions per item is the ceiling.** More than that means the item should have been
  split, and splitting it in the agenda is fine — say so in the brief.
- `Close` and `FYI` items get one line — *"**Ask:** confirm move to Settled."* — not a question
  block.

Finish with the consolidated checklist, grouped by target file, every group present even if empty:

```markdown
# Consolidated action checklist

_Filled in at the end of the meeting, then executed by `/endMeeting` in one pass._

### `official_spec_sheet.md`
- [ ] _(pending)_

### `.claude/CLAUDE.md`
### `.claude/work/decisions.md`
### `.claude/work/collab.md`
### `.claude/work/deferred.md` / `issues.md`
### GitHub tracker
### `documentation/`
### code — `get/src/`, `Cargo.toml`, `config.example.toml`, `examples/`
### skills and hooks
```

## 6. Rerun — fold in what is new, preserve what is not

A rerun happens because items were raised after the agenda was built. It is an **update, not a
rebuild**, and the difference is the whole reason this section exists.

```bash
git log --oneline <SHA-in-agenda-header>..HEAD -- .claude/work/collab.md
git diff <SHA-in-agenda-header>..HEAD -- .claude/work/collab.md
```

Then:

1. **Classify only what changed.** New items get a section. An existing item whose *last* stamped
   block is new gets re-read and possibly re-statused — an answer appended since the first pass can
   move `Decide` to `Ratify` or straight to `Close`, which is the most valuable thing a rerun does.
2. **Never touch a section whose `Decisions & doc changes` block is filled in.** That is a human
   decision and this skill does not own it. If such an item's source text changed, leave the section
   alone and add a note under the block: *"⚠ `collab.md` #N gained a reply after this was decided —
   re-read before `/endMeeting` acts on it."*
3. **Never delete a section.** If an item was settled in `collab.md` between passes, re-status it to
   `Close` and say why; do not remove it. Someone may have been counting on discussing it.
4. **Preserve hand edits.** If a brief or an option was reworded by a person — it will not match what
   you would have written — keep their wording. Re-status and re-order around it. When in doubt,
   keep the human's text and note the divergence.
5. **Re-order** the whole agenda after folding in, since a new blocker changes the order. Moving a
   section is not editing it.
6. **Update the header:** bump the SHA, bump the item range, and append a line —
   `**Reruns:** <YYYY-MM-DD HH:MM> — folded in #60–#62; #51 moved Decide → Ratify.`

## 7. Report

Five lines, no more:

- Where the file is, and how many items at each status.
- The blocked-on chain, if any — which item must go first and what it gates.
- Anything found in `collab.md` that needs a human: duplicate numbers, an unanswered ask that has
  been open a long time, an `#N` that is ambiguous between collab and GitHub.
- On a rerun: what was folded in, what was re-statused, and what was left alone because it was
  already decided.
- That nothing outside `meetings/` was touched.

## Constraints

- **Never edit `collab.md`.** Not to fix a duplicate number, not to move an item between `Open` and
  `Settled`, not to append an answer. Reorganising that file means moving lines that already exist,
  which `CLAUDE.md`'s announce-first rule reserves for a joint decision — so it is an *agenda item*,
  not something this skill does on the way past.
- **Never edit any other document.** No `decisions.md`, no spec sheet, no code. This skill reads and
  writes exactly one file.
- **Never invent an item.** If something needs discussing and has no `collab.md` item, say so in the
  report and let a person raise it properly. An agenda entry with no source is unciteable afterwards.
- **Do not commit or push.** The agenda is a working document; committing it is a separate,
  explicitly-requested step. Say in the report that it is uncommitted.
- **Do not run `/startMeeting`.** Preparing the agenda and walking it are different sittings, and
  usually different days.
