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

*Nothing pending. James had no work in flight when this file was created on 2026-08-13 — no open PR
and no remote branch — and the site was current as of that date.*
