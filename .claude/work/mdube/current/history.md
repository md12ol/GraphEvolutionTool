# History — GitHub #21: define and write the run output (convergence log + best individual)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-13 (cont. 2): `ci_95` shipped in the engine and through erasure; task 3 of 11

**What changed.** `GenerationStats::ci_95` (`get/src/evolver/mod.rs`) and its computation in
`generation_stats()` (`get/src/evolver/common.rs:297`) — half-width `1.96 · s / √n` on the sample
deviation (`n-1`), `0.0` when `n=1`. Carried through `erase()` (`dispatch.rs:416`) and
`PyGenerationStats`/`PyRunResult::from_erased` (`py_result.rs`) unconverted, matching `std_dev`.

**Tests.** Extended `generation_stats_computes_best_mean_and_population_deviation` (asserts `ci_95`
against a hand-computed value, and that it actually differs from `std_dev` on the same data — so
the test can't pass by `ci_95` accidentally equaling `std_dev`), `a_single_individual_has_zero_deviation`
(`ci_95 == 0.0`, not NaN), `generation_stats_stays_in_engine_orientation_under_maximize` (`ci_95`
unchanged under negation), and `the_erased_history_comes_out_in_the_objectives_own_units`
(`ci_95 >= 0.0` post-erasure). **Validated:** 235 tests pass (up from 232 pre-merge — 3 new
assertions added to existing tests, no new `#[test]` functions), clippy and fmt both clean.

**Git manifest.** `mdube_run_output`, clean, nothing uncommitted. Not yet pushed this session's
code commits.

*Logged 2026-08-13 — Michael.*

## Session 2026-08-13 (cont.): unparked again, migrated into the new dedicated `main` worktree

Unparked via `/load run-output` while still on the old model (`.claude/work/mdube/` tracked inside
whatever branch was checked out) — the unpark move landed staged-but-uncommitted directly on
`mdube_run_output`. Before continuing #21's actual work, that same session found and fixed the root
cause: switching back to `mdube_run_output` after `result-object` and `per-owner-work-dirs` closed
on `main` showed both as still parked, because this branch's copy of `.claude/work/mdube/` was
frozen at branch-creation and never saw `main` move. Fixed by moving `.claude/work/<owner>/` into a
dedicated `main` worktree (`collab.md` #58, PR #70, not yet merged) — every skill now reads/writes
that worktree, never the branch checked out for code.

This task's own live files are migrated here as part of that fix: copied straight from
`mdube_run_output`'s working tree into the `main` worktree (no git history carried across, since
the source was uncommitted). `mdube_run_output` itself will drop its now-redundant
`.claude/work/mdube/` entirely in its next commit, so a later `git merge main` doesn't conflict on
that subtree.

*Logged 2026-08-13 — Michael.*

## Session 2026-08-13: parked to pick up `result-object` — PRs #65, #66, #69 confirmed merged

**What this session did.** No code changes. Confirmed via `gh pr list --state all` that PR #65
(`RunResult`), #66 (LF `.gitattributes`) and #69 (per-owner work dirs) are all now merged — none of
task 1's stacked-branch premise changed, but `plan.md`'s "stacked on unmerged work" note was stale
and is corrected. `result-object` (blocked on #65/#66 and `collab.md` #53/#54) is now unblocked on
the PR side and has real queued work (tracker edits for #21 and #68, pending user confirmation), so
this task is parked to pick it up. `per-owner-work-dirs` remains parked — unblocked on #69 but its
`collab.md` #55 reply is still outstanding.

**Git manifest.** `mdube_run_output`, clean, nothing uncommitted, not pushed, no PR open. `origin/main`
is 8 commits ahead of this branch (collab items #55–#57 landed there); merging `main` in is the
first thing the next session on this task should do.

## Session 2026-08-13: task opened and the stacked branch created

**What this session did.** `/start` for GitHub #21, chosen over #27 because #27's task
(`parked/result-object/`) is still blocked — PR #65 and #66 are both open and unmerged, and
`collab.md` #53 and #54 are unanswered. #21 is the only unblocked issue assigned to Michael.

**The branch.** `mdube_run_output`, created off `mdube_result_object` rather than `main`, with
`mdube_per_owner_work_dirs` merged in. Clean merge — 21 files, no conflicts, because the two PRs
touch disjoint sets (`.claude/` + `.gitignore` versus `get/src` + `documentation/` + `examples/`).
Verified with `git merge-base --is-ancestor` against both. Reasoning in `decisions.md`
2026-08-13 02:41.

**Why the base matters.** #21's scope was amended on 2026-08-13 after #27 landed: `save_logs` and
`save_results` are now structurally broken rather than merely unimplemented — both take `&self` and
the evolver caches nothing since #27 — so they have to be re-homed onto `RunResult`. `RunResult`
only exists on #65's branch, so `main` is not a viable base.

**What was read to write the plan, and what it established.**

- `official_spec_sheet.md` §6.4 — the five log columns, the two deliberately different denominators
  (`std_dev` population `n`, `ci_95` sample `n-1`), the within-run versus across-run distinction, and
  the provenance TOML written beside the results.
- `get/src/evolver/common.rs:292` `generation_stats()` — computes the population deviation today and
  is where `ci_95` goes.
- `get/src/dispatch.rs:411` `erase()` — orients `best_fitness` and `mean_fitness`, passes `std_dev`
  through; `ci_95` joins the pass-through group.
- `get/src/py_result.rs` on the #65 branch — `PyRunResult` / `PyGenerationStats` as they stand.
- `get/src/lib.rs:266,272` — the two `todo!()` stubs to delete.
- `get/src/py_config.rs:376` — `PyConfig::to_toml()` already exists, so the provenance record needs
  no new serializer.

**Design call recorded in the plan, not yet built:** run-level fields (`seed`, `run_index`, the
config TOML) go on `RunResult` and are emitted per row by the CSV writer, rather than being stored
on every in-memory `GenerationStats`. §6.4 asks for the columns in the file, and this keeps the
engine struct free of run metadata while making #20's replicate work a fill-in.

**Git manifest.** One repo. Branch `mdube_run_output` at `db5d863` (the merge commit), unpushed.
Working tree otherwise clean — no source was edited this session. `main` is at `5f9be3b`.
Other local branches: `mdube_result_object` (PR #65), `mdube_per_owner_work_dirs` (PR #69),
`mdube_sh_eol` (PR #66) — all open, all awaiting James.

*Session logged 2026-08-13 02:41 — Michael.*
