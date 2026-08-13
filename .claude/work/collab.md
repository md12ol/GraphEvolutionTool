# Collaboration log — questions, answers, and overrides

Shared by everyone who works in this repo (Michael / md12ol and James / shorinbonsai). Everyone
reads and writes it. It is not addressed at any one person.

## What goes here

- **A question** you want the other owner to answer before you build on it.
- **A decision on your side that conflicts with or overrides theirs** — the kind where proceeding
  silently wastes someone's work.

Not every disagreement. If it needs no answer from anyone, it is a `decisions.md` entry instead.

## How to use it

**Raising.** Append a new item at the end of **Open**, numbered one higher than the last. Say what
you want — **Confirm**, **Decide**, or **FYI** — in the first line. An item with no ask sits open
forever.

**Answering.** Append your reply *inside* that item, beneath the existing text, as its own stamped
line. Do not edit what the other person wrote; do not delete their words to make room for yours.

**Settling.** When an item is resolved, move the whole item to **Agreed** with the date, keeping
every stamp. Agreed items are never deleted — the trail is what stops the same argument recurring.

## Formatting — one rule that bites

`/.gitattributes` sets `merge=union` on this file and `decisions.md` (narrowed 2026-08-04 — the
other three working docs no longer use it), so concurrent appends merge without
conflict markers — and **never conflict**, which means byte-identical lines on both sides fold
together and interleave two entries into one. So **close every item with its own number and a
time**: `*#7 · raised 2026-07-31 15:42 — Michael.*` — never a bare `*Raised <date> — <name>.*`.

Audit before pushing and after any merge; anything it prints could collapse:

```bash
grep -vE '^\s*$' .claude/work/collab.md | sort | uniq -d
```

**Run this second check too — the first one cannot see a splice** (added 2026-08-09, item #23).
Union merge can graft one entry into the middle of a line of another, which duplicates no line, so
the audit above returns clean on a corrupted file. It happened here on 2026-08-04:

      grep -n '^### [0-9]' .claude/work/collab.md | wc -l   # then eyeball the list itself

A heading that shows up mid-line, or one you know exists but which this does not list, is the
splice. Two formatting notes, both learned by tripping over them while adding this paragraph:
indented rather than fenced, because a second ```bash fence makes the `uniq -d` audit print the
fence lines forever; and worded differently from the identical command inside item #23, because two
byte-identical lines are the very thing the audit is looking for.

Full rules: `CLAUDE.md`, "Formatting for union merge".

Persistent: survives `/done`, because coordination outlives any one task.

---

## Open

**Nothing is open.** Items 1–19 were settled at the joint meeting of 2026-08-04 and items 20–38 at
the meeting of 2026-08-09; all of them sit under **Settled** below, bodies and stamps intact, behind
their disposition tables. **Append the next item as 40**, numbering continuing across both sections.

*(Item 39 — the trace of that meeting's self-merge, two direct spec pushes and the force-push — was
raised and settled the same day, and sits at the very end of this file. It needed no decision once
both owners had been through all three in the room.)*

Every item raised up to and including #62 is settled and now lives in
`collab_settled.md`. This file holds items that are still open; there are none at present.

**Before appending:** take the next free number from **both** this file and `collab_settled.md` —
the highest so far is **#63**. Reusing a number is what `#59` existed to fix.
