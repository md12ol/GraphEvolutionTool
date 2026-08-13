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

**It supersedes `CLAUDE.md`'s "de-badge its documentation in the same PR" rule** for the *timing*
only. The de-badging still has to happen — badge, `.plan-note` callout, and the `status.html` row —
just in the sweep rather than in the shipping PR. That rule binds both owners, so the wording in
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

## 2026-08-13 15:04 — Michael — ci_95, seed and run_index now ship on RunResult

- **Trigger:** GitHub #21, `mdube_run_output` branch, commits `5b1f066` and `d187d10`.
  `GenerationStats` gained `ci_95`; `RunResult` gained `seed`, `run_index` (hard `0` until
  replicates, #20) and `config_toml`.
- **Files:** `guide/output.html` (log column table ~L34-51; "Getting the data out" section and its
  plan-note ~L138-165; the CSV example's `run` column ~L179); `reference/lib.html`
  (`RunResult`/`GenerationStats` signature blocks ~L288-319; `save_logs`'s own column table already
  names `ci_95` correctly at ~L342 but the surrounding badge is stale — see the next entry);
  `status.html` (the "`ci_95`, and the per-row seed and run index" row, ~L98-105).
- **Now false:** `output.html` marks the `ci_95` and `seed`/`run` log columns `planned`.
  `lib.rs.html`'s `GenerationStats` signature block lists only `iteration, best_fitness,
  mean_fitness, std_dev`, with no mention of `RunResult`'s new `seed`, `run_index`, `config_toml`.
  `status.html`'s row says today's log has only four columns.
- **Should say:** All shipped. `GenerationStats` is `iteration, best_fitness, mean_fitness,
  std_dev, ci_95`; `RunResult` additionally carries `seed: int`, `run_index: int` (hard `0` until
  #20) and `config_toml: str` — the TOML document the run's config was parsed from.
- **Naming correction:** the site's example column and prose call it `run` (`output.html` L48,
  L179; `lib.rs.html` L350). The shipped column is `run_index`. Rename throughout.
- **Badges:** `output.html` — drop `badge-planned` from the `ci_95` row (L44) and the `seed, run`
  row (L48). `status.html` — delete the "`ci_95`, and the per-row seed and run index" table row
  (~L98-105) entirely.

*#run-output-ci95-seed · filed 2026-08-13 15:04 — Michael.*

## 2026-08-13 15:04 — Michael — save_logs and save_results are real, and live on RunResult, not the evolver

- **Trigger:** GitHub #21, same branch, commits `79003ac` and `d30d31d`. Both are `pub fn` on
  `py_result::PyRunResult`, not `&self` stubs on `GraphEvolver`. `grep -rn "todo!" get/src` is now
  empty.
- **Files:** `reference/lib.html` (the two `api-item` blocks ~L321-380, and the `plan-note` at
  ~L382-391); `guide/output.html` ("Getting the data out" section ~L138-165, and its plan-note);
  `status.html` (the "`save_logs` / `save_results`" row, ~L106-113).
- **Now false:** the site calls these as `evolver.save_logs(...)` / `evolver.save_results(...)`
  everywhere. `lib.rs.html`'s plan-note says both are `todo!()` placeholders that panic, at
  `lib.rs:266-269` and `:272-275`, and still take `&self` on the evolver. `status.html` says "Both
  raise."
- **Should say:** both are called on the **result**, not the evolver:
  `result.save_logs(filename: str) -> None` and `result.save_results(filename: str) -> None`,
  defined in `py_result.rs` (`PyRunResult::save_logs`, `PyRunResult::save_results`). `save_logs`
  writes a header then one row per logged iteration, columns
  `iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index`. `save_results` writes the
  best fitness, the winning genome's `print()` string and its edge list to `filename`, and the
  run's config TOML to `{filename}.toml` alongside it — derived from `filename` rather than a
  second argument, so it can't be forgotten. Verified against real Python: `maturin develop`, a
  full run, `save_logs` + `pandas.read_csv` + a matplotlib plot, `save_results` +
  `Config::from_toml_str` round-tripping the written TOML.
- **Badges:** `reference/lib.html` — drop `badge-planned` from both `api-item` blocks (L325, L369)
  and delete the now-false plan-note (L382-391); replace with what the methods actually do, or
  drop it — sweep's call. `guide/output.html` — the "Getting the data out" section (L138) mixes a
  shipped half (single-run `save_logs`/`save_results`) with an unshipped half (`runs=30,
  max_cores=8`, the list return, #20) under one `planned` badge; split it rather than dropping the
  badge outright — the single-run calls are real, the replicate signature is not.
  `status.html` — delete the "`save_logs` / `save_results`" row (~L106-113) entirely.

*#run-output-save-methods · filed 2026-08-13 15:04 — Michael.*
