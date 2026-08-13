# History — GitHub #21: define and write the run output (convergence log + best individual)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

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
