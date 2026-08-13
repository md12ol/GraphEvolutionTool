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

## 2026-08-13 16:43 — James — replicate runs shipped: de-badge six pages, and three code samples name the wrong parameter

- **Trigger:** GitHub #20, branch `jsargant_replicate_runs`. `run(seed, n_runs=1, max_cores=None)`
  now returns a **list** of `RunResult`, always — even at `n_runs=1` — with the master seed feeding
  a draw stream, native-Rust replicates running through a per-call rayon pool, and `python` fitness
  running them sequentially. Commits `6f8fc5c`, `50e4f7b`, `1abd10f`, `aa3e05c`.
- **Files:** `guide/python-api.html` (the "Replicates" section, ~L240, and its `.plan-note` at
  ~L288); `guide/reproducibility.html` ("Replicate seeding", ~L94); `guide/performance.html` (the
  `max_cores` heading, ~L112); `reference/lib.html` ("Replicate runs" ~L393 and the `api-item` at
  ~L403); `examples/index.html` (section 9, ~L320, and its `.plan-note` at ~L352); `status.html`
  (the "Replicate runs" row ~L83-90 and the `max_cores` row ~L92-97).
- **Now false:** every "Today:" note describing a single-run `run(seed)`. `python-api.html` says
  "`run(seed)` takes exactly one seed and performs one run, returning a single `RunResult` rather
  than a list" and advises a Python loop; `examples/index.html` says "`run(seed)` performs a single
  run and returns the edge list"; `status.html`'s two rows list both features as unbuilt.
  `reference/lib.html`'s api-item says "the `seed` parameter exists today; the other two are
  designed" — all three exist now.
- **Should say:** the prose describing the design is already accurate and needs no rewriting — this
  is badge and callout removal, not correction. The one substantive addition worth making is that
  the list is returned **unconditionally**, so `run(seed=1)` gives a one-element list rather than a
  bare result; that is the only part of the shipped behaviour the site does not already describe.
- **Naming correction — this one is a defect, not a badge.** Three pages show
  `evolver.run(seed=20260812, runs=30, max_cores=8)`: `guide/output.html` L140,
  `guide/python-api.html` L247, `examples/index.html` L329. The shipped parameter is **`n_runs`**,
  as `reference/lib.html` L403 already has it, so those three samples raise `TypeError` if copied.
  Rename `runs=` to `n_runs=` in all three.
- **Stale `src` reference:** `reference/lib.html` L405 points at `lib.rs:235` for `run`; the
  signature has moved with this change and the line is no longer right.
- **Badges:** drop `badge-planned` from `python-api.html` L240, `reproducibility.html` L94,
  `performance.html` L112, `lib.html` L393 and L406, `examples/index.html` L320; delete the
  `.plan-note` blocks at `python-api.html` ~L288 and `examples/index.html` ~L352; delete both
  `status.html` rows.
- **Adjacent, and not mine to file:** `reproducibility.html` L234 still badges "the seed and the run
  index on every log row" as planned. That shipped with GitHub #21 (PR #71), so it belongs in
  `mdube_edits.md` — flagged here because it sits four lines from a badge this entry does remove,
  and whoever sweeps this page will be looking straight at it.

*#replicate-runs-ship · filed 2026-08-13 16:43 — James.*
