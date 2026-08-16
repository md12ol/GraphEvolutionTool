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

## 2026-08-13 12:45 — James — `set_base_graph` exists now, so every "not yet" claim about it is false

- **Trigger:** GitHub #28, branch `jsargant_set_base_graph`, commits `3af041c` (the setter and its
  three checks) and `c99fa11` (threading it through `dispatch::evolve` into `edge_edit_start`).
  Shipped as `GraphEvolver.set_base_graph(num_nodes, edges)`, matching the signature the site
  already documents.
- **Files:** `guide/python-api.html` (the `#base-graph` section, ~L317-340); `examples/index.html`
  (the stacking example's `.plan-note`, ~L314-317); `status.html` (the "Supplying a base graph"
  row, ~L115); `HANDOFF.md` (the mirror row at ~L82).
- **Now false:** `examples/index.html` says "**Today:** there is no `set_base_graph`, so an
  edge-edit run always starts from an empty graph and stacking is not yet possible." Stacking is
  possible now, and the example above that note runs as written. `status.html` and `HANDOFF.md`
  both still list the feature as planned.
- **Should say:** the setter exists and takes `(num_nodes, edges)` with `edges` as
  `(u, v, multiplicity)` triples — the same shape `run` returns as `best_edges`, so an SDA run's
  output feeds an edge-edit run with no reshaping. Unset means an empty base graph, which is the
  default, and five of the nine opcodes (`Swap`, `Hop`, the three `Local*`) are inert on one until
  `Add`/`Toggle` build structure — self-correcting, not a defect.
- **Badges:** `guide/python-api.html` — drop `badge-planned` from the `#base-graph` heading (L317).
  `examples/index.html` — delete the `.plan-note` at ~L314-317 entirely. `status.html` — delete the
  "Supplying a base graph" row (~L115). `HANDOFF.md`'s row is the duplicate `collab.md` #57 raised;
  leave it to whatever #50 settles rather than patching the same table twice.

*#set-base-graph-ships · filed 2026-08-13 12:45 — James.*

## 2026-08-13 12:45 — James — cap narrowing raises now; the site still says it silently collapses

- **Trigger:** GitHub #28, commit `3af041c`. The decision is `decisions.md` 2026-08-12 — the
  cap-narrowing check **rejects** with `ValueError` rather than warning or clamping.
- **Files:** `guide/python-api.html` (the three-checks list under `#base-graph`);
  `examples/index.html` (the `.warn` block above the stacking plan-note, ~L307-313).
- **Now false:** `python-api.html` says cap narrowing "must be rejected **or warned**", which was
  the open question and is now settled. `examples/index.html`'s warning says setting edges
  "**clamps** rather than rejecting, so piping a cap-3 result into a cap-1 run silently collapses
  every weight to 1 and you get a plausible-looking network" — that is still true of
  `Graph::set_edge` itself, but no longer of the path a user can reach: `set_base_graph` checks
  every multiplicity before building anything and raises.
- **Should say:** `set_base_graph` raises `ValueError` naming the offending edge, its multiplicity
  and the configured cap, so the silent collapse is not reachable through the Python API. The
  advice to keep `max_edge_multiplicity` identical or raise it stands — it just fails loudly now
  instead of quietly. Worth keeping a sentence that `Graph::set_edge` still clamps, since that is
  why the setter has to check at all, and it is what a Rust-side embedder still faces.
- **Badges:** none — neither location carries a `badge-planned` span or a `.plan-note`. This is a
  correctness fix to prose, not a de-badging, and it is separable from `#set-base-graph-ships`.

*#set-base-graph-cap-rejects · filed 2026-08-13 12:45 — James.*

## 2026-08-13 14:41 — James — the setter owes five checks now, not three, and two of them are new

- **Trigger:** the joint meeting of 2026-08-13 settled `collab.md` #61 — `decisions.md` 20:16,
  "Caller-supplied graph data is rejected, not silently dropped". `set_base_graph` now rejects an
  out-of-range endpoint and rejects a self-loop, each raising `ValueError` naming the offending
  edge. Implemented on `jsargant_set_base_graph` for GitHub #28. `Graph::set_edge` is unchanged.
- **Files:** `guide/python-api.html` — the "The setter owes three checks" list under `#base-graph`.
- **Now false:** the list opens "the node count must match `network_size`, **or out-of-range edges
  are silently dropped**". That was the rationale for check 1 and is no longer what happens: the
  node count is still checked, and separately every edge is checked, so an out-of-range endpoint
  raises rather than disappearing. The framing "three checks" is also now wrong.
- **Should say:** the setter validates the declared node count against `network_size`, and then
  each edge for an out-of-range endpoint, a self-loop, and a multiplicity above
  `max_edge_multiplicity` — raising `ValueError` on the first failure and building nothing. Worth
  keeping the reason, because it is the non-obvious part: a node count equal to `network_size` does
  **not** make the edges in range, and a caller who takes `num_nodes` from their config rather than
  their data hits exactly that. The unset-base bullet is unaffected and stays.
- **Badges:** none — this list carries no `badge-planned` span or `.plan-note`. Prose correctness
  only, and it stacks with `#set-base-graph-ships`, which de-badges the section this list sits in.

*#set-base-graph-five-checks · filed 2026-08-13 14:41 — James.*
