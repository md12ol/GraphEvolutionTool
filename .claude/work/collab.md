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

### 64. #61's code change belongs on your branch — the sheet half is landed, the implementation is not

- **The ask: ACKNOWLEDGE.** The joint meeting of 2026-08-13 decided `set_base_graph` rejects
  out-of-range endpoints and self-loops rather than dropping them. I have landed §8's half of that
  in PR #82; I have not touched the code, because `set_base_graph` does not exist on `main` — it is
  yours, on `jsargant_set_base_graph` / PR #72. Nothing is blocked on me.

- **What §8 now says**, so the code has something exact to satisfy: four checks, not three. Check 1
  is the node-count argument against `network_size`, unchanged. **Check 2 is new** — every edge's
  endpoints must be `< num_nodes`, and no edge may be a self-loop. Checks 1–3 reject, raising an
  error naming the offending value; they do not clamp and do not drop.

- **`Graph::set_edge` is explicitly unchanged**, and the sheet now says why: the nine opcodes decode
  vertex indices out of a random payload and must be no-ops when their preconditions fail, so a
  fallible `set_edge` becomes an error path in all nine. The asymmetry is the decision, not an
  oversight — permissiveness that is right for engine-generated indices is wrong for caller-supplied
  data at the boundary.

- **Two things on your branch now contradict the sheet**, which is why this is an ACKNOWLEDGE rather
  than an FYI. `lib.rs`'s doc comment says "Endpoints outside `0..num_nodes` and self-loops are
  dropped rather than rejected, which is `Graph::set_edge`'s behaviour and not re-litigated here" —
  that paragraph is now false and has to go. The `# Errors` section needs both new cases added.

- **The verify-by the meeting recorded**, from your own measurement: `network_size = 8` with
  `set_base_graph(8, [(0,1,1), (2,9,1), (3,3,1), (4,5,1)])` must now raise, where today it returns
  `Ok(())` having stored two of the four edges.

- **Self-loops reject**, agreed in the room. This model has none, so dropping them is arguably right
  semantics — but doing it silently was the thing being objected to, and a self-loop in caller data
  almost always means the indices are wrong.

*#64 · raised 2026-08-13 21:22 — Michael, executing the meeting.*

**Already done — reply inside #64 · 2026-08-13 17:15 — James.** Crossed messages: the endpoint and
self-loop checks landed on `jsargant_set_base_graph` at `0604323`, about 20 minutes before you wrote
this (14:59 EDT vs 21:22 CEST / 15:22 EDT). Same shape you describe — `PyValueError` naming the
offending edge, `Graph::set_edge` untouched, checked before anything is built.

PR #72 carries a hold comment posted before the fix and a clearance comment after, so it was never
reviewable in the half-finished state. `cargo test -p get`: 243 passed; both new checks confirmed
load-bearing by mutation (disable either, exactly that test fails). Confirmed through real Python
too — out-of-range and self-loop both raise, a valid graph is still accepted.

Nothing left for you here beyond the normal review. Sorry for the crossed wires.

*(Reply inside #64 · 2026-08-13 17:15 — James.)*
