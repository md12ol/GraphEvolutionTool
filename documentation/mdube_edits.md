# `documentation/` — the pending-edits queue

**Do not edit the site during an ordinary task. File the edit here instead.**

Added 2026-08-13 — Michael. This file exists because keeping `documentation/` in step with the code
*inside every task's PR* costs more per task than the task itself: shipping GitHub #27 alone touched
ten HTML files, none of which were the point of the work. Batching is cheaper and produces one
coherent sweep instead of ten partial ones.

## The rule

When a task changes something the site describes — a signature, a returned type, a name, a claim
about what does or does not exist yet — the task **does not** open the HTML. It appends an entry
below saying what is now wrong and what it should say. The site is then corrected in **one sweep**,
as its own task, when Michael says so.

**This is an explicit standing instruction to whoever is working, agent or owner: keeping this file
up to date is part of finishing a task, not optional tidying.** A task that changes the code and
files nothing here has left the site quietly lying, which is the exact failure the
`badge-planned` convention was built to prevent. The obligation moved; it did not go away.

~~**It supersedes `CLAUDE.md`'s "de-badge its documentation in the same PR" rule** for the *timing*
only. The de-badging still has to happen — badge, `.plan-note` callout, and the `status.html` row —
just in the sweep rather than in the shipping PR.~~ **Struck 2026-08-21 — Michael:** there is no
de-badging left to time. The badges, `.plan-note` and `status.html` are all deleted, and no page
describes an unbuilt feature; see `README.md`, "There is no convention for unbuilt work". That rule binds both owners, so the wording in
`CLAUDE.md` needs amending to match; until it does, this file and that rule disagree and this file
is the newer decision.

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
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `documentation/mdube_edits.md` |
| `shorinbonsai@gmail.com` | `documentation/jsargant_edits.md` — created for him, still pending his agreement in `collab.md` #53 |

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
## <short title, stated as the change>

- **Pages:** the pages and rough locations.
- **Now false:** the claim the site currently makes.
- **Should say:** what replaces it.

*#<slug> · filed <YYYY-MM-DD HH:MM> — <author>.*
```

**Corrected 2026-08-20 — Michael.** This block used to prescribe a
`## <YYYY-MM-DD HH:MM> — <author> — <short title>` heading with a `**Trigger:**` line, which no
entry filed since 2026-08-18 has followed: the date and author migrated to a closing stamp and the
heading became the change itself, which is what a sweeper scanning for "does this page appear
anywhere" actually reads. The instructions were describing a practice nobody used, so they are
corrected to the practice rather than the entries being rewritten to the instructions.

Delete an entry when the sweep has applied it — this is a queue, not a log. What was changed and why
belongs in `decisions.md`; this file only carries work that has not been done yet.

## Pending

Swept 2026-08-21 against the 16-page site. Ten entries left — eight applied, two moot when
`reference/`, `status.html` and `HANDOFF.md` were deleted. What survives is the work the trim did
not do, restated against pages that exist. The evidence for each disposition is in
`.claude/work/archive/`'s `close-67` history.

## Edge files: the `# nodes = N` header is stated on one page out of four

- **Pages:** `guide/route-python-toml.html`, `guide/route-rust-library.html`,
  `guide/route-rust-cli.html`. `guide/route-python-objects.html:162` already has it.
- **Not false, thin:** no page describes the format wrongly any more, and
  `guide/fitness.html:475` correctly says `save_results` writes a loadable edge file. But three of
  the four route pages never mention that an edge file a reader supplies must carry the header, so a
  reader on those routes meets the rejection message before the rule.
- **Should say:** the header is required rather than optional, and **why** — a node with no edges is
  invisible to any count taken from the edges, so an inferred count is short by exactly the nodes
  hardest to notice. A base-graph file whose header disagrees with `network_size` is rejected; a
  reference file's count is its own and is expected to differ across the set.
- **Narrowed 2026-08-21** from `#edge-files-state-their-node-count`, whose "now false" list is spent:
  every page it named is deleted, and no surviving page repeats the claim.

*#edge-file-header-missing-from-three-route-pages · filed 2026-08-20 09:40, narrowed 2026-08-21 — Michael.*

## Per-route environment setup — still nothing on the site

- **Pages:** the four route pages, one section each.
- **Now wrong, and unchanged by the trim:** there are no environment setup instructions anywhere.
  `grep -rl "maturin develop\|ensurepip" documentation/` is empty. A reader has to infer that the
  Python routes need `ensurepip`, a venv, `maturin` and `maturin develop`, and that the Rust CLI
  route needs a real Python on `PATH` because `cli` pulls `pyo3/auto-initialize` — without which the
  run dies as exit 127 before printing a word. `guide/route-rust-cli.html` names
  `auto-initialize` but does not turn it into a setup step.
- **Should say:** for each of the four routes, the exact terminal commands in order. **Assume a bash
  shell** — WSL, Ubuntu or macOS — and call out explicitly anywhere a platform differs rather than
  leaving the reader to discover it. Also raised as a comment on GitHub #67.
- **The extension-marker half of this entry is done** — PR #155 branch-qualified all eight markers
  and the guide pages follow them. Only the setup instructions remain.

*#per-route-setup-instructions · filed 2026-08-20 21:42, narrowed 2026-08-21 — Michael.*
