---
name: startMeeting
description: Walk the prepared agenda at .claude/work/meetings/<YYYY-MM-DD>.md one item at a time, asking each item's questions as click-to-answer choices and recording the decision and the doc changes it implies into that item's block. Changes no other file. Use when the joint meeting is starting, or resuming after a break.
---

# Start meeting

Take the agenda `/makeAgenda` prepared and turn it into minutes. Walk the items in order, ask each
one's questions as buttons, and write the answer and its consequences into that item's
**Decisions & doc changes** block.

**The one rule that matters: nothing outside the meeting file is edited.** Not the spec sheet, not
`CLAUDE.md`, not `collab.md`, not a line of code. The meeting decides; `/endMeeting` executes. This
is deliberate — decisions taken in the first half routinely change what the second half decides, and
a document edited at item 3 and contradicted at item 14 is worse than one edited once at the end.

## 0. Set up

Work in the `main` worktree, same as `/makeAgenda`:

```bash
MAIN_TREE="$(git rev-parse --show-toplevel)"
DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"
cd "$DOCS_WT" && git pull
```

`/startMeeting [YYYY-MM-DD]`, defaulting to today. Then check the header's `Status:` line:

| `Status:` | Do this |
|---|---|
| `prepared` | Normal start. Stamp it `in progress`, set the cursor to the first item, go to §1 |
| `in progress` | **Resume.** Report which items are already decided, name the item the cursor is on, and confirm before continuing from there. Never restart from the top |
| `closed` | **Stop.** That meeting ended. Offer `/makeAgenda` for a new date |
| file missing | **Stop.** Run `/makeAgenda` first. Do not improvise an agenda from `collab.md` in the room |

Stamp the header before asking anything:

```markdown
**Status:** in progress
**Started:** <YYYY-MM-DD HH:MM>
**Cursor:** #<N>
```

The cursor is what makes this survive an interruption — a meeting that breaks for lunch resumes at
the right item instead of re-asking eleven answered questions.

## 1. Before the first item, read the map aloud

Two lines, once:

- How many items, at what statuses, and roughly how long the Decide block is.
- The blocked-on chain — which item gates which — so nobody is surprised when item 3 turns out to
  have been pre-agreed by item 1.

## 2. Walk one item at a time

For each item, in agenda order:

**a. Present the brief.** Not a paraphrase and not a re-read of the whole section — the four to six
sentences the agenda already holds, compressed to what the room needs to answer the question. If
someone asks for the underlying detail, quote `collab.md` rather than recalling it.

**b. Ask the questions as buttons.** Use `AskUserQuestion` with the options exactly as the agenda
wrote them, so the labels and their costs are the ones both owners already read. Ask an item's
questions together in one call where they are independent; ask them in sequence where the second
depends on the first.

**Never ask a question the agenda did not prepare** unless the discussion actually opened one. If it
did, ask it — a meeting that surfaces a real question is working correctly — but say plainly that it
is new, and record it in the item's block as a question raised in the room, so a reader can tell it
apart from the prepared set.

**c. Take the discussion.** The buttons are the fast path, not the only path. If either owner says
something that changes the shape of the question, follow it. Do not force a prepared option onto a
conversation that has moved past it — record what was actually decided, including "neither, we are
doing X instead".

**d. Write the block, immediately, before moving on.**

```markdown
### Decisions & doc changes

**Decided:** <one or two lines. The decision itself, in the language a reader six months
from now can act on — never "option (b)".>

- `<target file>` — <the specific edit: what changes, from what to what>
- `<target file>` — <…>
- **Knock-on #<M>:** <what this pre-decides about a later item>
```

Three things about that block, all load-bearing:

- **Name the target file for every consequence.** A decision with no file attached is a decision
  `/endMeeting` cannot execute, and it will be silently dropped.
- **"Option (b)" is not a decision.** The letters are scaffolding for the meeting and meaningless
  once the agenda is archived. Write the substance.
- **Record knock-ons the moment you notice them.** When an answer settles part of a later item, say
  so in *both* items — the earlier one gets a `Knock-on` line, the later one gets a note above its
  questions. Then, when you reach the later item, ask only what is genuinely still open.

**e. Confirm and move the cursor.** Read the block back in one sentence, update `**Cursor:**` in the
header, and go to the next item. If either owner corrects the readback, fix it before moving —
correcting it later means correcting it from memory.

## 3. Rapid-fire the Close and FYI block

These get one confirmation each, not a discussion: *"#40, `/done`'s skill body — acknowledged and
resolved in the thread. Move to Settled?"* Batch them into a single multi-select where the answers
are all the same shape, and record each in its own block regardless.

**If one turns out not to be settled, pull it out of the batch and treat it as `Decide`.** That is
the point of reading them aloud rather than assuming.

## 4. Close the meeting

**a. Fill the consolidated checklist.** Walk every item's block and roll every consequence up into
the checklist, grouped by target file. This is a transcription, not a fresh judgement — if a
consequence is in an item's block it belongs in the checklist, and if it is in the checklist it must
trace back to an item.

**Flag conflicts rather than resolving them.** Two decisions that edit the same file in
incompatible ways is exactly what the end of the meeting is for, and both owners are still in the
room. Ask.

**b. Sort each group into route.** `/endMeeting` needs this and the room is the cheapest place to
get it:

- **Direct to `main`** — `.claude/work/*.md`, `.claude/CLAUDE.md`, skill bodies.
- **Branch + PR** — `official_spec_sheet.md`, anything under `get/src/`, `Cargo.toml`,
  `config.example.toml`, `documentation/`, skill frontmatter, `hooks/`.
- **Tracker** — new issues, with the dependency level `(N)` derived from their blockers, and an
  assignee where one was agreed.

**c. Read back the whole checklist** and get a single confirmation that it is complete and correct.
This is the last moment both owners see it together.

**d. Stamp the header and stop.**

```markdown
**Status:** closed
**Ended:** <YYYY-MM-DD HH:MM>
**Decisions:** <count> · **Actions:** <count> · **Next:** `/endMeeting <YYYY-MM-DD>`
```

Then say what `/endMeeting` will do, and that **nothing has been changed yet**. Do not run it. It
is a separate, explicitly-requested step, usually with fresh context, and it is the one that touches
real files.

## 5. If the meeting stops early

Leave `Status: in progress` and the cursor where it is. Report which items are decided, which are
not, and that `/startMeeting` resumes at the cursor. **Do not fill the consolidated checklist from a
partial meeting** — a half-filled checklist is indistinguishable from a complete one to
`/endMeeting`, which is how half a meeting's decisions get executed as though they were all of them.

## Constraints

- **Edit exactly one file: the meeting file.** Every other document waits for `/endMeeting`.
- **Never commit or push.** Not the meeting file either. Minutes are committed as part of
  `/endMeeting`, or on explicit instruction.
- **Never skip an item because the answer seems obvious.** The Close items exist precisely because
  "obviously settled" has been wrong before, and confirming one costs ten seconds.
- **Never record a decision the room did not make.** If discussion ran out of time on an item, mark
  the block `**Deferred to next meeting.**` and say so. An inferred decision is worse than an
  undecided one, because it carries the same authority and nobody remembers agreeing to it.
- **Never edit `collab.md`, even to append an answer inside an item.** That append is real and it is
  wanted — it is `/endMeeting`'s job, so the file is touched once.
