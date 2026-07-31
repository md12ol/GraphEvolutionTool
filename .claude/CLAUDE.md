# GraphEvolutionTool (GET) — working rules

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
| `hotfixes.md` | temporary code in the tree, each with a `Remove when:` |
| `traps.md` | permanent gotchas about this workspace — the things that bite every session |

Finished tasks land in `.claude/work/archive/<YYYY-MM>_<slug>/`.

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
