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

**Task-scoped** — `.claude/work/current/`, archived by `/done` when the task ends:

| File | |
|---|---|
| `work/current/plan.md` | objective + tasks. `[ ]` pending · `[x]` done **and verified** · `[~]` done, NOT verified. **A task list, not a record** — see the size rules below |
| `work/current/plan_superseded.md` | original wording of tasks now done. Reference only, never actionable |
| `work/current/history.md` | append-only session log for this task |
| `work/current/handoff.md` | prompt for the next session — **read this first** |

**Persistent** — these describe the *code*, not the work, so they outlive the task:

| File | |
|---|---|
| `decisions.md` | append-only: what was chosen and why |
| `issues.md` | staged for the tracker, for other people |
| `hotfixes.md` | temporary code in the tree, each with a `Remove when:` and an **`Owner:`** — a hotfix lives in one person's working tree, so it needs a name on it |
| `traps.md` | permanent gotchas about this workspace — the things that bite every session |
| `collab.md` | **questions and overrides between the owners.** Post a question for the other to answer, or flag a decision on your side that conflicts with theirs. Answers are appended *inside* the item, stamped. Settled items move to **Settled** and compress to a one-line disposition once their reasoning lives in `decisions.md` or the spec — never edit someone else's words, and never drop the only copy of a reason |

Finished tasks land in `.claude/work/archive/<YYYY-MM>_<slug>/` — **tracked**, so a finished
task's record reaches the other owner. Only `work/current/` is per-person.

### Keep `plan.md` small — it is a task list, not a record

Left alone it grows without bound. In the project this template came from it reached **1432 lines**
and had to be halved by hand. Evidence, rationale and superseded wording had all piled up in it, and
each of those already has a file that owns it: what happened → `work/current/history.md` · why →
`decisions.md` · original wording of a finished task → `work/current/plan_superseded.md` · temporary code
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

**1. The persistent docs merge by union — so stamp every entry with an author.**
`decisions.md`, `traps.md`, `hotfixes.md`, `issues.md` and `collab.md` are append-only, which
means both of us write to the tail of the same file and every concurrent session would otherwise
end in a merge conflict. `/.gitattributes` sets `merge=union` on them: both sides' lines survive,
no conflict markers.

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

Audit any of these files with:

```bash
grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d
```

Anything it prints is a line two entries could collapse onto. All five files were clean on
2026-07-31.

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

## Workflow

**Start the task**

1. New task
2. `/start` — agree the objective, write `work/current/plan.md` **before any code**
3. Work

**Then loop, once per session** ⟳

4. `/save` — update every doc, write the next-session prompt · *last thing before you stop*
5. `/load` — read the handoff, check it against the repo, report · *first thing when you return*
6. Work
7. Not finished? → back to **4**

**Finish the task**

8. `/done <slug>` — settle every loose end, then archive `work/current/` → `archive/<YYYY-MM>_<slug>/`

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

**Verify after filing; don't trust the exit code.** Re-read the issue with `gh issue view <n>` and
confirm the body survived — collapsed blocks, code fences and tables are what get mangled. A filed
issue with a broken body is worse than an unfiled one.

**The sync obligation.** A staged issue can be rewritten freely; a filed one cannot. Once filed,
the tracker is the source of truth — changes go to the tracker in the same session, and
`issues.md` must not become a private fork of it.

Record any `gh` quirk here the moment you hit it. These cost an hour each, every time, and they
are exactly what a cold session cannot rediscover.


## Conventions

- **Never mark work `[x]` that you have not seen verified.** If it only compiled, or only ran
  somewhere that doesn't count, it is `[~]`. Work that looks done and isn't is the most expensive
  failure mode this system has.
- **Every task needs a `Verify by:`** — the command, the log line, the artifact to inspect. A task
  with no verification method is how `[~]` items become false `[x]`s later.
- Absolute dates only, never "today" or "last session".
- Reference code as `path:line`.
- Don't commit or push unless asked.
- Flag temporary work as temporary and add it to `hotfixes.md`.
- Date rules when you change them, and supersede rather than overwrite: strike the old line through
  and add the new one with its date and reason. The reversal trail is worth more than a tidy file.

## Do not use the auto-memory store for this project

**Write project state into the file that owns that lifetime**, not into a memory file:
temporary code → `hotfixes.md` · someone else's work → `issues.md` · why → `decisions.md` ·
what happened → `work/current/history.md` · what's next → `work/current/plan.md` · workspace gotchas →
`traps.md` · how we work → this file.

The reason is not that memory is useless — it is that a second, auto-loading store of the same facts
drifts out of sync with the files, and a stale memory that presents itself as current is worse than
no memory at all. That is what happened in the project this template came from: two entries had gone
wrong while still auto-loading as though true, and the store was deleted.

<!-- Delete this section if you would rather use the memory store. If you do keep memory, at least
     pick ONE home per fact — the failure is duplication, not memory itself. -->
