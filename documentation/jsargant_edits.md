# `documentation/` — James's pending-edits queue

**Do not edit the site during an ordinary task. File the edit here instead.**

Created 2026-08-13 by Michael, on James's behalf, ahead of the ask in `collab.md` #53 — so the
convention was usable the moment James said yes, and so a session working under his identity had
somewhere to file instead of guessing.

**Agreed by James 2026-08-13**, at the joint meeting; the item is in `collab_settled.md`. The
rejection route described here originally — deleting the file — is spent, and the queue is now the
standing convention for both owners.

The companion file is `documentation/mdube_edits.md`, which carries the same rules and the same
format. Neither is the master copy; if the two ever disagree about *process*, `collab.md` #53 and
whatever it settles into is the authority.

## The rule

When a task changes something the site describes — a signature, a returned type, a name, a claim
about what does or does not exist yet — the task **does not** open the HTML. It appends an entry
below saying what is now wrong and what it should say. The site is then corrected in **one sweep**,
as its own task, when the owner says so.

**This is an explicit standing instruction to whoever is working, agent or owner: keeping this file
up to date is part of finishing a task, not optional tidying.** A task that changes the code and
files nothing here has left the site quietly lying, which is the exact failure the `badge-planned`
convention was built to prevent. The obligation moved; it did not go away.

**It replaces `CLAUDE.md`'s "de-badge its documentation in the same PR" rule** on *timing* only. The
de-badging still has to happen — badge, `.plan-note` callout, and the `status.html` row — just in the
sweep rather than in the shipping PR. That bullet was struck through in `CLAUDE.md` on 2026-08-13
(`a73af39`) once both owners agreed, so the two documents no longer disagree.

## Which file — check, do not assume

There is one queue **per owner**, because this is a churn list: an entry is *deleted* once the sweep
applies it, and `CLAUDE.md` already establishes that deletions are exactly what a union merge cannot
express. Separate files mean neither owner ever touches the other's, so no merge can silently
resurrect an applied entry.

Decide by identity, not by memory:

```bash
git config user.email
```

| Email | File |
|---|---|
| `shorinbonsai@gmail.com` | `documentation/jsargant_edits.md` — this file |
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `documentation/mdube_edits.md` |

**Anything else: stop and ask.** Do not pick the likelier one. Filing into the wrong owner's queue is
silent — the entry is neither lost nor found, and it surfaces only when someone sweeps a file they
did not expect to have work in it.

**A sweep reads every queue file, not just its own.** One page can be owed edits by both owners, and
applying half of them leaves the page wrong in a way that looks deliberate.

## Filing an entry

One `##` heading per edit, so two sessions appending concurrently cannot collapse into each other.
Say **where**, **what is now false**, and **what it should say** — enough that the sweep does not
have to re-derive it from the code.

```markdown
## <YYYY-MM-DD HH:MM> — <author> — <short title>

- **Trigger:** what shipped, and the issue number.
- **Files:** the pages and rough locations.
- **Now false:** the claim the site currently makes.
- **Should say:** what replaces it.
- **Badges:** any `badge-planned` span, `.plan-note` callout or `status.html` row to remove.
```

Delete an entry when the sweep has applied it — this is a queue, not a log. What was changed and why
belongs in `decisions.md`; this file only carries work that has not been done yet.

## Pending

**Swept 2026-08-21 by Michael, at his instruction, on branch `mdube_close-67`.** This is a departure
from "only the queue's owner deletes" — flagged in `collab.md` #113 before it was done, and the trail
is here rather than in a commit message. Twelve entries in, one out. Every disposition below was
checked against the 16 pages by grepping for what the entry claimed, not inferred from the page list.

**Applied by the trim, or moot with the page (9), deleted:**
`#set-base-graph-ships`, `#set-base-graph-cap-rejects`, `#set-base-graph-five-checks`,
`#set-base-graph-takes-min-node-index` — the setter, its checks, the `ValueError` on a narrowed cap
and `min_node_index` are all on `guide/route-python-objects.html`, and no "not yet" claim survives
anywhere. `#replicate-runs-ship` — `n_runs` is correct on every page and the `runs=` samples that
would have raised `TypeError` are gone. `#base-graph-file-loaders` — applied except its warning
half, see below. `#genome-table-now-rejects-unknown-keys` — the pages naming it are deleted and
`guide/variation.html`'s only "one table" is about SDA transitions. `#crossover-now-has-a-shared-helper`
— was already tagged `edited — please verify`; `reference/evolver-common.html` is deleted.
`#operator-config-enums-added` (all three) — `status.html` and `reference/sda.html` are gone, and
`guide/variation.html`'s operator note was rewritten in PR #157.

**Applied in this PR (3), deleted:**
`#fitness-chain-documented-in-crate` and the chain half of `#struct-match-objective-added` —
`guide/new-fitness.html`'s Route 2 was a second, divergent copy of the crate's chain: a different
order, no dispatch-test step, and none of the three constraints. Rewritten to the crate's six steps
with constructor validation, the no-filesystem rule on config validation and the once-per-replicate
`Arc` note. Its objective-list half was already done — `guide/fitness.html` says "The Four Built-in
Objectives". `#selection-extension-point-documented` — the eight-site chain it described became six
across four files in PR #147 and `guide/new-selection.html` already had that; the three contracts and
the enum-is-not-a-trait framing it asked for were still missing and are now on the page. The warning
half of `#base-graph-file-loaders` is on `guide/route-python-objects.html`.

## `SdaGenome::from_parts` and its four accessors are documented nowhere

- **Page:** `guide/route-rust-library.html` — the only surviving page whose reader assembles genomes
  by hand. `reference/sda.html`, which this entry originally named, is deleted.
- **Now missing:** `SdaGenome::from_parts(init_char, transitions, responses, max_resp_len)` returns
  `Result` and is the only supported way to supply a chosen automaton, or to feed a previous run's
  winner into a later one. The four accessors that return exactly its arguments —
  `init_char()`, `transitions()`, `responses()`, `max_resp_len()` — go with it. Added in PR #146
  (GitHub #121); `grep -rn from_parts documentation/` is empty.
- **Should say:** what each check converts — every one turns a failure that would otherwise land
  mid-run into one at construction. State plainly that an **empty response does not panic, it
  hangs**: running the automaton makes progress only by appending a response's characters, so it
  loops until the process is killed. And that it deliberately does *not* check the alphabet against
  a context's `max_edge_multiplicity`, because nothing at construction knows which context the
  genome will meet — `express` asserts that pairing.
- **Left filed rather than written 2026-08-21:** the surviving page is about driving a run, not
  about genome internals, so where this belongs is a judgment call about the shape of the route
  page rather than a correction to something false.

*#sda-chosen-automaton-constructor · filed 2026-08-20 15:05, repointed 2026-08-21 — James's entry, swept by Michael.*
