# GraphEvolutionTool (GET) — working rules

## The design lives in `/official_spec_sheet.md` — read it first

`official_spec_sheet.md` at the repo root is **the authority on how this system is designed**:
the graph, both genomes, the mutation contract, fitness and orientation, both evolvers, config
and validation, the Python interface, and the non-goals. Agreed by both owners on 2026-07-31.

- **Read it before changing anything in `get/src/`.** It answers most "why is it like this"
  questions without archaeology, and it records decisions the code does not yet reflect.
- **It is design only.** No build order, no task list — sequencing lives in its own document.
- **Where the sheet and the code disagree, the sheet is the intent.** This inverts the usual
  "the repo wins" rule below, and it is deliberate: parts of the sheet were agreed before being
  implemented. Fix the code, or write a dated entry superseding the sheet — never silently
  follow the code.
- **Changing it is a `decisions.md` entry too.** The sheet says *what*; `decisions.md` says
  *why* and keeps the reversal trail.

**The sheet is only changed at a joint meeting.** Not by one owner mid-task, not by an agent, not
because the code turned out to be easier a different way. The route is fixed:

1. Something needs to change → raise it in `collab.md` under **Open**, with the ask.
2. Both owners discuss it. Until then, **build to the sheet as written**, or stop and ask.
3. Agreed at a meeting → amend the sheet, append a dated `decisions.md` entry stamped with both
   names, and move the `collab.md` item to **Agreed**.

This is what keeps the sheet worth trusting: if any session could edit it, it would drift into
being a description of whatever was most recently implemented, which is the exact failure the
sheet replaced (`IMPLEMENTATION.md` mixed design with build order and rotted every time the order
changed). An agent that finds the sheet wrong writes a `collab.md` item — it does not fix the
sheet.

## Working docs

Session state lives in `.claude/`:

**Task-scoped** — `.claude/work/<owner>/current/`, archived by `/done` when the task ends, or moved
to `.claude/work/<owner>/parked/<slug>/` by `/park` when the task is blocked:

| File | |
|---|---|
| `current/plan.md` | objective + tasks. `[ ]` pending · `[x]` done **and verified** · `[~]` done, NOT verified. **A task list, not a record** — see the size rules below |
| `current/plan_superseded.md` | original wording of tasks now done. Reference only, never actionable |
| `current/history.md` | append-only session log for this task |
| `current/handoff.md` | prompt for the next session — **read this first**. Carries a `Machine:` stamp and the SHA the save was written against |

**`<owner>` is `mdube` or `jsargant`, resolved from `git config user.email` — checked, never
assumed** (added 2026-08-13). The mapping table lives in `.claude/hooks/session_brief.sh` and in the
`load`, `save`, `park` and `start` skills; an unrecognised address stops and asks. Never read or
write the other owner's directory.

**These are tracked, and that is the point** — changed 2026-08-13, when `work/current/` stopped
being gitignored. The old reason for ignoring it was that two people must not fight over one live
plan; the per-owner *path* now serves that reason, so nothing is gained by hiding the files and one
thing is lost: a task that lives on one laptop cannot be picked up on another. `/save` commits and
pushes `work/<owner>/` as its last step, which is a deliberate, narrow exception to "don't commit or
push unless asked" — see Conventions.

**The hazard this introduces is you-versus-you, not you-versus-James.** Two machines editing one
`plan.md` is a real conflict on a file that is rewritten in place, which no merge strategy can help
with. So `handoff.md` carries `Machine: <hostname> · saved <ts> · <SHA>`, and `/load` **stops and
reports** on divergence rather than merging or resetting. Note `pull_main.sh` fast-forwards `main`
at session start but **refuses on a dirty tree** — a divergence usually means it declined.

**Parking a blocked task.** `/park <slug>` runs `/save`, stamps `handoff.md` with `Blocked on:` —
the concrete unblocking event, not "waiting on James" — and moves `current/` to `parked/<slug>/`,
leaving the desk clear for `/start`. `/load <slug>` brings it back, parking whatever is live to make
room. Parked slugs carry **no date prefix**: `work/archive/` uses `<YYYY-MM>_<slug>` because it is a
chronological record, while a parked task is live and takes its date from the plan. `/done` refuses
a parked task — resume it first, because `/done`'s final save has to run against the session that
actually finished the work.

**Persistent** — these describe the *code*, not the work, so they outlive the task:

| File | |
|---|---|
| `decisions.md` | append-only: what was chosen and why |
| `issues.md` | staged for the tracker, for other people |
| `deferred.md` | **not yet** — wanted, out of scope for the first release. Sits between §10 Non-goals (*never*) and the tracker (*now*). An entry names the change and what admitting it requires; **no dates, no ordering, no priority**, or it becomes the build order the spec sheet refuses to carry. Filed → it leaves. Added 2026-08-11 |
| `hotfixes.md` | temporary code in the tree, each with a `Remove when:` and an **`Owner:`** — a hotfix lives in one person's working tree, so it needs a name on it |
| `traps.md` | permanent gotchas about this workspace — the things that bite every session |
| `collab.md` | **questions and overrides between the owners.** Post a question for the other to answer, or flag a decision on your side that conflicts with theirs. Answers are appended *inside* the item, stamped. Settled items move to **Settled** and compress to a one-line disposition once their reasoning lives in `decisions.md` or the spec — never edit someone else's words, and never drop the only copy of a reason |

Finished tasks land in `.claude/work/archive/<YYYY-MM>_<slug>/` — **tracked and shared**, with no
owner in the path. A finished task is the project's history; only *live* tasks are per-owner.

**Reference notes** — `.claude/reference/`, added 2026-08-07. Longer-form notes about how a
*dependency or toolchain* behaves, where a `traps.md` entry would be too long and a `decisions.md`
entry would be the wrong shape because nothing was decided. Deliberately **outside `work/`**, so it
is never mistaken for a churn list and never picks up a merge driver.

| File | |
|---|---|
| `reference/pyo3-maturin.md` | the Python boundary: why `extension-module` breaks `cargo test`, why calling Python from a rayon closure deadlocks, and what GET still lacks (a `pyproject.toml`) |

Each note says whether a claim was **measured here** or came from elsewhere. Keep that split — a
borrowed configuration is evidence that something works somewhere, not that it is right here.

### Keep `plan.md` small — it is a task list, not a record

Left alone it grows without bound. In the project this template came from it reached **1432 lines**
and had to be halved by hand. Evidence, rationale and superseded wording had all piled up in it, and
each of those already has a file that owns it: what happened → `work/<owner>/current/history.md` · why →
`decisions.md` · original wording of a finished task → `work/<owner>/current/plan_superseded.md` · temporary code
→ `hotfixes.md` · someone else's work → `issues.md`.

- **Completed item: ≤ 3 lines**, compressed **when you tick it** — what was done, the one piece of
  evidence that verifies it, and where the detail lives. Never paste the evidence in.
- **Open item: ≤ 20 lines** — what to do, the verify-by, and any constraint that causes harm if
  forgotten. Longer reasoning goes in `decisions.md`, and the plan links to it.
- **Soft cap ~600 lines.** Over it, compress the biggest completed items before appending new ones.
- **Amalgamate** duplicate items rather than keeping both.

### Keep one task per task

`/done` exists and should actually fire. A task whose objective needs six lettered sections is a
*program*, not a task — split it, and let each section close on its own gate. The symptom of getting
this wrong is an empty `archive/` next to a plan and history that no longer fit in context, so every
session pays to re-read them before doing any work.

## Two people, one `.claude/`

This `.claude/` is checked into the repo and used by **both owners on their own machines** —
Michael (md12ol) and James (shorinbonsai). Four rules follow from that, and all four are
non-obvious.

**1. The two append-only docs merge by union — so stamp every entry with an author.**
`decisions.md` and `collab.md` are append-only, which means both of us write to the tail of the
same file and every concurrent session would otherwise end in a merge conflict. `/.gitattributes`
sets `merge=union` on those two: both sides' lines survive, no conflict markers.

**`traps.md`, `issues.md` and `hotfixes.md` are NOT union-merged** — narrowed 2026-08-04. They are
**churn lists**, where deleting an entry is a normal operation, and union merge cannot express a
deletion: a delete that races with any edit to the same region is silently discarded and the entry
comes back. Those three take git's normal 3-way merge, so a concurrent append **conflicts** and is
resolved by hand. That is deliberate — loud and occasional beats silent and wrong. Full reasoning
in `decisions.md` 2026-08-04 18:25 and `/.gitattributes`.

The catch is that union merge **never conflicts**. Measured 2026-07-31: two entries with
distinct text merge correctly and only lose the blank line between them, but lines that are
**byte-identical** on both sides are deduplicated and the two entries interleave into one block
that reads as coherent and is not. So:

### Formatting for union merge

An entry's **first and last lines are the ones a merge treats as shared context**, so those are
what must be unique. Four rules, all load-bearing:

1. **The heading is unique** and carries the author and a time:
   `## 2026-07-31 15:42 — Michael — <title>`, or `### 7. <the item>` in `collab.md`.
2. **The closing stamp repeats that identity and carries a time** —
   `*#7 · raised 2026-07-31 15:42 — Michael.*`. Two independent guards: the item number, and the
   `HH:MM`, which makes a byte-identical stamp essentially impossible even for two entries by the
   same author on the same day. Never a bare `*Raised 2026-07-31 — Michael.*` — that collides the
   moment one person raises two items in a day, and it was live in `collab.md` until 2026-07-31,
   **nine times over**. Entries written before this rule keep their date-only stamps; times were
   not recorded and are not to be invented.
3. **Never close an entry with a bare `---`.** Headings delimit entries; a repeated horizontal rule
   is exactly the identical boundary line rule 1 warns about.
4. **No bare structural labels.** Write `- **Body:** <first sentence>`, not `- **Body:**` alone —
   a label with nothing after it is byte-identical in every entry that uses it. Same for
   `- **Added:** <date>`: append the entry's slug.
5. **Append; do not edit an existing entry in place — and if you must, say so in `collab.md`
   first. This is a rule, not a courtesy** (agreed 2026-08-04). The announcement is the *only*
   mechanism that prevents the concurrent case, because git will not warn you: two people editing
   the same existing line makes union keep **both** versions silently. Rules 1–4 protect against
   byte-identical
   lines being *deduplicated*. The opposite failure also exists: if two people edit the **same
   existing line** on separate branches, union keeps **both** versions one after the other and
   reports `1 file changed, 1 insertion(+)`. Measured 2026-08-04 on a 250-line file, so it is not a
   small-file artifact. **Authorship is irrelevant** — union does not know who wrote a line, so
   editing "your own" entry is fine socially and buys nothing mechanically. What matters is whether
   both sides touched the region. If an entry genuinely must be amended, raise it in `collab.md`
   *first*: the announcement is the only thing that prevents the concurrent case, because git will
   not warn you.

Audit any of these files with:

```bash
grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d
```

Anything it prints is a line two entries could collapse onto. All five files were clean on
2026-07-31.

**`uniq -d` is not sufficient on its own — run a structure check beside it** (added 2026-08-09,
`collab.md` #23). Union merge has a third failure it cannot see: it can splice one entry into the
*middle of a line* of another, which repeats no line, so the audit above comes back clean on a
genuinely corrupted file. Measured on `main` 2026-08-04, when one item was spliced into another and
stopped being a top-level heading at all:

```bash
grep -n '^### [0-9]' .claude/work/collab.md   # every heading at column 0; count as expected
```

An item heading that appears mid-line, or one you know exists but which this does not list, is the
splice. Full mechanism in `traps.md`, `union-merge-splices-entries-without-duplicating`.

- After a merge that touched these files, **read the tail** — `git diff HEAD~1 -- .claude/work/`.
  Fix interleaves by hand; the merge won't have told you.
- Editing or deleting *someone else's* entry is a `collab.md` item, not a silent rewrite.

**2. Hook and settings changes go through a PR.** `settings.json` and everything in `hooks/` is
executable code that runs on the other person's machine at session start, on their next pull,
without them reading it. Never push a change to either straight to `main` — open a PR and say
what it does. This is the one part of `.claude/` where "it's just docs" is false.

**3. `/setup` runs once, ever — never on a clone.** It rewrites `CLAUDE.md` from the template's
FILL IN blocks and would destroy this file. If you have just cloned the repo, `.claude/` is
already set up; start with `/load`. Personal settings go in `settings.local.json`, which is
gitignored and exists exactly for that.

**4. Verification is per-machine.** `[x]` means *you* saw it verified, on your machine. Never
promote someone else's `[~]` to `[x]` because their notes read as finished — re-run the
`Verify by:` or leave it alone.

## Pull requests — the other owner merges yours

**Added 2026-08-04 — nobody merges their own PR.** James merges Michael's; Michael merges James's.
Opening it, pushing to it and asking for review are yours; clicking merge is not. ~~An agent never
merges a PR at all — it opens one and stops.~~ **Amended 2026-08-04 18:25 — an agent never merges
*unprompted*.** It opens a PR and stops; told to merge, it merges. The original absolute wording was
overridden twice within hours of being written, both times correctly, which is the definition of a
rule that is stated wrong rather than one being broken.

### What must go through a branch and a PR, and what may not

**Added 2026-08-04 15:55 — Michael.** The line is **code versus working docs**, and it is drawn
where review actually buys something:

| Change | Route |
|---|---|
| Anything under `get/src/`, `Cargo.toml`, `config.example.toml` — code that solves an issue | **Feature branch + PR.** Never a direct push to `main`, no matter how small |
| `settings.json`, `hooks/` | **Feature branch + PR** — these execute on the other person's machine at session start (see rule 2 above) |
| `/official_spec_sheet.md` | **PR, and only after a joint meeting** — see the top of this file |
| `.claude/work/*.md` — `decisions.md`, `traps.md`, `issues.md`, `hotfixes.md`, `collab.md` | Direct push to `main` is fine. They carry no behaviour, and a trap that is not on `main` protects nobody. Note only `decisions.md` and `collab.md` are union-merged (rule 1 above) |
| `.claude/work/<owner>/` — live and parked task directories | Direct push to `main`, **by `/save` and `/park` automatically** (added 2026-08-13). Nobody reviews your own plan, and an unpushed handoff is the failure these directories were tracked to prevent. Union merge does not reach them — verified with `git check-attr merge` |
| `.claude/CLAUDE.md` | Direct push is permitted, but **prefer a PR when the change binds the other owner's practice** rather than recording a fact |
| `.claude/skills/*/SKILL.md` — **frontmatter** (`model:`, `allowed-tools:`, any hook-adjacent key) | **Feature branch + PR.** Changing it changes what executes on the other person's machine on their next pull, without them reading it |
| `.claude/skills/*/SKILL.md` — **body** | Direct push to `main` is fine. It is prose we both read anyway, and a PR round-trip in front of a typo fix is how a rule starts being skipped |

**The test is "does this change what runs", not "which directory is it in"** — added 2026-08-09,
agreed in `collab.md` #34. That is the whole reason rule 2 exists for `settings.json` and `hooks/`,
and it is why the skills row splits rather than naming the directory: the next person to add a
fourth directory should be able to route it from the principle instead of waiting for the table to
catch up. Frontmatter is configuration the harness executes; a skill's body is prose a reader
evaluates. Michael pinned the five working-docs skills to `model: sonnet` in `011480d` before this
row existed, and logged it precisely because the rule did not yet cover it.

**This applies even while the task's own code PR is still open.** `/done`'s sweep — the task-complete
marker in `decisions.md`, `hotfixes.md`'s `Last checked` stamps, `traps.md` updates, the archive
itself — commits and pushes straight to `main` right then, not bundled into the code branch and not
held until the other owner merges it. They are two independent tracks: the PR carries the code, the
docs carry the record that the task is closed. Waiting would hold the task-closing record hostage to
someone else's review schedule, which is exactly the kind of stall `/done` exists to avoid. Settled
2026-08-06 doing exactly this for issue #22 while PR #43 was still open — `collab.md` #28.

The reason code is absolute: a defect in `get/src/` is invisible until something downstream reads a
wrong number, and the current issue set has several files claimed by two workstreams at once. The
reason docs are not: they carry no behaviour, and the one thing review would catch in them — a
union-merge interleave — has its own audit command, which is cheaper to run than a review is to
request.

Branch naming: `<owner>_<short-description>`, e.g. `mdube_sir_sim`, `jsargant_mutation_contract`.

This is not ceremony, because three things in this repo fail *silently* and a second reader is the
only thing that catches them:

- **`merge=union` never conflicts.** Byte-identical lines in `decisions.md` and `collab.md` — the
  two union-merged files, narrowed from all five on 2026-08-04 — dedupe and interleave two entries
  into one block that reads as coherent and is not. Git will not tell you; the reviewer might.
- **Source files genuinely overlap.** `collab.md` #14 has three files claimed by #10 *and* #14/#15
  at once. Review is where a conflicting edit gets noticed while it is still cheap.
- **Rule 2 above is the strict case, not the exception.** `settings.json` and `hooks/` execute on
  the other person's machine at session start, without them reading the diff. Those were already
  PR-only; this generalizes the habit to everything so the rule has no edge to fall off.

~~Self-merging is allowed in exactly one case: the other owner is unavailable and the change is
blocking.~~ **Widened 2026-08-09 at the joint meeting to two cases** — `collab.md` #29:

1. **The other owner is unavailable and the change is blocking.** Unchanged.
2. **A strict deletion, or a one-line correction, to a doc — where the change removes something
   that is already false.** New. The test is that the change *subtracts* a falsehood rather than
   adding a claim: dropping a caveat that cites a closed issue, correcting a status row for
   something that has shipped, fixing a glob that names files it no longer covers. A sentence that
   asserts something new is not this case, however short it is.

Either way: say so in the PR, and say it in `collab.md` too — an unreviewed merge should leave a
trace, not a gap.

**Why case 2 exists.** PR #37 was self-merged under case 1 when case 1 did not apply — James was
demonstrably available, having merged two PRs six minutes earlier — and Michael logged it honestly
as a self-merge of convenience rather than dressing it as the documented one (`collab.md` #29). A
rule that gets correctly broken is stated wrong, which is the same reasoning that reworded the
agent-merge rule above. The cost of the old wording was visible on 2026-08-09: the spec sheet's
status table had been stale on **four of nine rows** for days, each row naming a component as
unbuilt that had shipped, because correcting a fact needed the full branch-and-review cycle.
Reviewing a deletion of something false is a check nobody was ever going to fail.

**Merge locally whenever the PR touches `.claude/work/*.md` — never with the GitHub button.**
Measured 2026-08-04: `.gitattributes` merge drivers are applied by *your* git, not by GitHub's
servers, so `merge=union` does not run on the website. The same PR merges clean locally and reports
`mergeable=false, dirty` on GitHub, which then offers to resolve an append-only log in a textarea.

```bash
git checkout main && git pull && git merge --no-ff origin/<branch>
git diff HEAD~1 -- .claude/work/    # union never reports a problem; read the tail yourself
git push origin main
git push origin --delete <branch> && git branch -d <branch>
```

This is not a footnote to the rule above — the rule sends people to the merge button, and that is
exactly where the driver is absent. Full mechanism in `traps.md`.

**Delete the branch yourself after a local merge — added 2026-08-12, Michael.** The repo's
`delete_branch_on_merge` was turned on 2026-08-12 and does **not** cover this path: it fires only
when GitHub itself performs the merge, and a locally-merged PR is marked merged after the fact
without the cleanup running. So the last line above is part of the merge, not an optional tidy-up.
An agent that merges when told to merge deletes the branch in the same breath, remote then local.
Mechanism in `traps.md`, `auto-delete-does-not-fire-on-a-locally-merged-pr`.

## Workflow

**Start the task**

1. New task
2. `/start` — agree the objective, write `work/<owner>/current/plan.md` **before any code**
3. Work

**Then loop, once per session** ⟳

4. `/save` — update every doc, write the next-session prompt, push · *last thing before you stop*
5. `/load [slug]` — read the handoff, check it against the repo, report · *first thing when you return*
6. Work
7. Not finished? → back to **4**

**Blocked, not finished**

- `/park <slug>` — save, then set the task down in `work/<owner>/parked/<slug>/` and `/start`
  something else. `/load <slug>` picks it up again, parking whatever is live to make room.

**Finish the task**

8. `/done <slug>` — settle every loose end, then archive `work/<owner>/current/` → `archive/<YYYY-MM>_<slug>/`

Docs can go stale between sessions. Where the docs and the repo disagree, **the repo wins** —
report the discrepancy rather than following the stale version.


## Filing issues

Tracker: **GitHub**, via the `gh` CLI. Target project is **`md12ol/GraphEvolutionTool`** — the
only one. There is no per-component mapping and there never should be. The credential is the
user's `gh` auth; **never read, print, or echo the token** — only invoke `gh`.

**Confirm before every single file action.** Print the exact title, body and target repo, then
wait for an OK. One confirmation per issue — never a batch, never opportunistically mid-task.
Problems noticed during work go to `issues.md` first; filing is a separate, deliberate step.

**Pass no labels.** GitHub silently creates an unknown label as a side effect of using it, so the
safe default is none — let the owner triage.

**Issue titles carry a dependency level, `(N)`, and it is not a priority.** Every title in the
tracker opens with one — `(1) Rename common::evaluate ...`, `(7) Sweep generational and
steady_state ...`. The scale is topological, not editorial:

> **`(1)` depends on nothing. `(2)` depends on one or more `(1)`s. `(3)` depends on one or more
> `(2)`s**, and so on.

So the level is a fact about the issue's blockers, derived from the issue set — not a judgement
about how much anyone wants it. A `(1)` can be a cosmetic doc fix and a `(6)` can be the most
wanted feature in the project; `(6)` only says five layers of work stand in front of it.

**Give every issue a level when you file it, and derive it — never guess.** Name the issues it
depends on, take the highest level among them, add one. If it depends on nothing open, it is `(1)`.

**Written down 2026-08-11 — Michael.** This lived only in the owners' heads for 28 issues. A cold
session read the same 28 titles and inferred "priority, 1 = do first", which fits the visible
evidence almost perfectly — every `(1)` is closed, the big features sit at 4–6 — and is wrong.
The correlation is real and the causation is backwards: `(1)`s are all closed *because* nothing
blocked them, not because anyone ranked them first. An undocumented convention that a careful
reader can confidently misread is worse than one they would have to ask about, which is why it is
here now.

**Verify after filing; don't trust the exit code.** Re-read the issue with `gh issue view <n>` and
confirm the body survived — collapsed blocks, code fences and tables are what get mangled. A filed
issue with a broken body is worse than an unfiled one.

**The sync obligation.** A staged issue can be rewritten freely; a filed one cannot. Once filed,
the tracker is the source of truth — changes go to the tracker in the same session, and
`issues.md` must not become a private fork of it.

Record any `gh` quirk here the moment you hit it. These cost an hour each, every time, and they
are exactly what a cold session cannot rediscover.

**`gh issue view <n>` is broken on this repo — use `--json` (hit 2026-08-04, Michael).** The plain
command exits 1 with `GraphQL: Projects (classic) is being deprecated in favor of the new Projects
experience ... (repository.issue.projectCards)` and prints no issue at all. It is the default view's
`projectCards` field, not anything about the issue, so it fails for *every* issue number. Read
issues with:

```bash
gh issue view <n> --json title,body -q '.title, .body'
```

`gh issue list` is unaffected. This matters beyond convenience: the verify-after-filing rule above
requires re-reading a filed issue to confirm its body survived, and the bare command cannot do it.

**Amended 2026-08-04 11:30 — Michael: it is the whole default view, not one command.** `gh pr edit`
fails the same way (`repository.pullRequest.projectCards`), so writes are affected too, not just
reads. Assume any `gh` subcommand that fetches or updates a whole issue or PR is broken here, and
reach for `--json` on reads and the REST API on writes:

    gh api repos/md12ol/GraphEvolutionTool/pulls/<n> -X PATCH -F body=@body.md

`-F body=@file` reads the body from a file, which also avoids fighting the shell over backticks and
`§` in a long PR description.


## Conventions

- **Approving a plan is never authorization to commit, push, or open a PR.** Added 2026-08-04
  after PR #39 was opened unprompted and closed again. `/start` writes those outward actions into
  `plan.md` as tasks, and agreeing the task list is agreement about *what the work is* — not a
  standing go-ahead to perform it. Every commit, push and PR needs its own explicit instruction,
  each time, no matter what the plan says. This is the same rule as "don't commit or push unless
  asked" below; it is restated here because a plan step reading "open the PR" is exactly what makes
  that rule look satisfied when it is not.
- **Prefer explicit loops to iterator chains.** Both owners have to read every line here and one of
  us does not write Rust, so a plain `for` with an accumulator beats a chain that needs a turbofish,
  a closure returning through an `Option`, or more than about two adapters. Keep comments terse and
  written for someone new to the code; ~~link `official_spec_sheet.md` rather than restating it —
  a copy of the sheet drifts, and the sheet is the authority.~~ Agreed 2026-08-04; reasoning in
  `decisions.md` 2026-08-04 22:12.
  **Amended 2026-08-13 — Michael: do not reference the sheet from `get/src` at all.** Not by
  section number, not by name, not as a link. The original clause was aimed at *restating* the
  sheet, and "link it instead" was the cheap alternative — but the reader of a shipped crate has no
  access to `official_spec_sheet.md`, so every pointer to it is a dead end rather than a shortcut.
  Measured 2026-08-13: **135** such references had accumulated in `get/src`, against 10,251 lines.
  The rest of the clause stands unchanged — terse, for someone new, and never a copy of the sheet.
  Where a comment needs the *reason* a thing is correct, state the reason itself rather than citing
  where it was agreed. Backlog is GitHub #68; `documentation/`'s equivalent is #67.
- **No agent co-attribution on commits or PRs — ever, and never ask.** No `Co-Authored-By: Claude`
  trailer, no "Generated with Claude Code" footer, no `🤖` line. The author and committer are the
  owner whose machine it is, and nothing else appears. Added to *this* file 2026-08-09 — Michael,
  after six commits landed carrying the trailer. **The rule already existed and could not be seen
  from here:** James wrote it into `~/.claude/CLAUDE.md` on 2026-08-03, which is global and
  per-machine, so it bound his sessions and no one else's. A convention that lives only in one
  person's home directory protects one person — the same argument that puts traps on `main`. The
  precedent was also in plain sight and went unchecked: every one of the 40 commits before that day
  carries no trailer. **Check `git log` before inventing a commit convention.**
- **When a `planned` feature ships, de-badge its documentation in the same PR.** Added
  2026-08-13 — Michael. `documentation/` describes GET as `official_spec_sheet.md` designs it, so
  anything designed but not yet built is written in the present tense carrying a
  `badge-planned` span and a `.plan-note` callout, with `documentation/status.html` indexing every
  one. That convention is only honest while someone maintains it: shipping the feature and leaving
  the badge turns "not built yet" into a lie, and shipping it while leaving `status.html`'s row
  makes the one page people check for the answer wrong. So the PR that lands the code also greps
  `documentation/` for `badge-planned`, drops the badge and its callout, and removes the
  `status.html` row. Reasoning in `decisions.md` 2026-08-12 18:52. **Contingent on `collab.md` #50**
  — if the present-tense convention is dropped, this rule goes with it.
- **Never mark work `[x]` that you have not seen verified.** If it only compiled, or only ran
  somewhere that doesn't count, it is `[~]`. Work that looks done and isn't is the most expensive
  failure mode this system has.
- **Every task needs a `Verify by:`** — the command, the log line, the artifact to inspect. A task
  with no verification method is how `[~]` items become false `[x]`s later.
- **Commit each verified step of a feature branch separately — don't batch a task's changes into
  one commit at the end.** One commit per task-list item once its own `Verify by:` has passed: a
  lint-policy decision, a formatting sweep, one file of a readability pass. Keeps every commit
  independently reviewable and lets a reviewer, or a later `git bisect`, isolate exactly which step
  introduced a problem, rather than auditing one large diff at PR time. Added 2026-08-05 — Michael.
- Absolute dates only, never "today" or "last session".
- Reference code as `path:line`.
- Don't commit or push unless asked. **One exception, added 2026-08-13: `/save` and `/park` commit
  and push `.claude/work/<owner>/` — that path, at that step, and nothing else.** The exception
  exists because the live task directories are tracked precisely so a task can be resumed on another
  machine, and a save that never reaches `origin` fails silently: you find out on the other laptop,
  usually a day late. It does not widen. Code, the persistent docs and the spec sheet each still
  need their own explicit instruction, every time, and a `/save` that finds uncommitted source
  leaves it alone and says so.
- Flag temporary work as temporary and add it to `hotfixes.md`.
- Date rules when you change them, and supersede rather than overwrite: strike the old line through
  and add the new one with its date and reason. The reversal trail is worth more than a tidy file.

## Do not use the auto-memory store for this project

**Write project state into the file that owns that lifetime**, not into a memory file:
temporary code → `hotfixes.md` · someone else's work → `issues.md` · why → `decisions.md` ·
what happened → `work/<owner>/current/history.md` · what's next → `work/<owner>/current/plan.md` · workspace gotchas →
`traps.md` · how we work → this file.

The reason is not that memory is useless — it is that a second, auto-loading store of the same facts
drifts out of sync with the files, and a stale memory that presents itself as current is worse than
no memory at all. That is what happened in the project this template came from: two entries had gone
wrong while still auto-loading as though true, and the store was deleted.

<!-- Delete this section if you would rather use the memory store. If you do keep memory, at least
     pick ONE home per fact — the failure is duplication, not memory itself. -->
