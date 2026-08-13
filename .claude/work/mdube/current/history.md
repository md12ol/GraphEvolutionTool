# History — GitHub #21: define and write the run output (convergence log + best individual)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-13 (cont. 4): tasks 4–10 implemented, verified from real Python, and pushed

**All of #21's remaining code tasks landed**, one commit per task-list item, each gated on its own
`cargo test`/`clippy`/`fmt` pass:

- `d187d10` — `seed`, `run_index` (hard `0`), `config_toml` on `RunResult`. `GraphEvolver::new`
  reads the config file's raw text itself (`Config::from_toml_str` + `.validate()` inline) rather
  than through `Config::from_path`, since the raw text is the provenance record and `from_path`
  doesn't hand it back; `from_config` already had `text` in scope from `PyConfig::to_toml()`.
- `79003ac` — `save_logs` on `PyRunResult`, CSV with a header row, `iteration,best_fitness,
  mean_fitness,std_dev,ci_95,seed,run_index`. No `csv` crate — the data is all-numeric, so hand-
  rolled `writeln!` avoided a new dependency. `pymethods` are private by default; needed `pub fn`
  to call from a Rust test directly (matches `PyConfig::to_toml`'s existing precedent).
- `d30d31d` — `save_results` on `PyRunResult`, writing the best genome/edges/fitness to `filename`
  and the provenance TOML to `{filename}.toml` — derived, not a second argument, so it can't be
  forgotten (the plan's open question on this was never actually settled by James; went with the
  planned default).
- `b4e3bb7` — doc comment on `PyRunResult::best_fitness` explaining the final-population-not-best-
  ever caveat; `guide/evolvers.html` already had the full reasoning, so no site edit was needed —
  this was a genuinely skipped plan task, caught by `/save`'s sweep, not done inline with the rest.
- `24c3cc0` — two `documentation/mdube_edits.md` entries (ci_95/seed/run_index/config_toml;
  save_logs/save_results), naming every page each de-badges and the `run`→`run_index` naming fix
  the site's example code needs.

**Verified against real Python, not just `cargo test`** — task 8's own gate. No `pyproject.toml`
in this repo (per `reference/pyo3-maturin.md`) and Debian's PEP 668 blocks a bare `pip install`, so
built `.venv` + `maturin develop`, `pip install pandas matplotlib`. A full run through the built
module: `save_logs` → `pandas.read_csv` (correct dtypes) → matplotlib plot (sent to Michael);
`save_results` → `{path}.toml` → `Config::from_toml_str` round-trip. `.venv/` gitignored — see
below, landed on `main` directly rather than in this branch, since it isn't specific to #21.

**Branch pushed and merged with `main`.** `mdube_run_output` is at `b4e3bb7`, includes a clean
`git merge main` (`49dc100`, only `.claude/work/` and `.gitignore` touched — same auto-resolve
pattern as the earlier merge). PR **not yet opened** — on explicit instruction only, not yet given.

**A branch collision mid-session**, worth knowing about rather than acting on: another session
checked out `mdube_docs_worktree` in this same checkout and committed to it (`f343402`, the
`/makeAgenda`/`/startMeeting`/`/endMeeting` skills, later part of PR #70) while this session had
uncommitted task-4 edits pending. Recovered cleanly — `git stash`, switch back, `git stash apply` —
nothing lost, verified by diff before continuing. See `traps.md` for the durable version of this.

**Off-task, landed on `main` directly:** `.gitignore` gained `.venv/` (`092b944`) — repo hygiene,
not #21-specific, so it didn't belong in this branch; done via a temporary sparse-checkout widen in
the docs worktree (`git sparse-checkout set '/.claude/work/*' '/.gitignore'`, commit, push, narrow
back) since `main` can't be checked out twice across the two working trees.

**Git manifest at end of session:**
- `GraphEvolutionTool` (code) — branch `mdube_run_output` @ `b4e3bb7`, pushed, matches origin. Clean
  except one untracked `GraphEvolutionTool.code-workspace` (belongs to PR #70, not this branch).
- `GraphEvolutionTool-docs` — branch `main` @ (this save's commit), pushed. Clean.

---

## Session 2026-08-13 (cont. 3): `ci_95` committed; a large side-quest built and fixed the docs worktree

**#21 itself.** `ci_95` committed on `mdube_run_output` at `007d3cf` (see previous session entry for
the implementation detail — nothing changed there, just landed the commit). Branch has 3 local
commits not yet pushed (`333806d`, `2a6059b`, `007d3cf` since `origin/mdube_run_output`); pushing
needs its own explicit instruction per `CLAUDE.md`, not yet given.

**The side-quest, triggered by the user noticing the exact bug the new worktree exists to prevent**
(`per-owner-work-dirs` and `result-object` closing on `main` while this branch still showed them
parked). Built out over several iterations, each one surfacing a real problem the last one didn't
catch:

1. Dedicated `main` worktree at `../GraphEvolutionTool-docs`, every `.claude/work/`-touching skill
   updated to use it (PR #70, `mdube_docs_worktree`, open — not merged, per "nobody merges their
   own PR").
2. **Sparse-checked-out to `.claude/work/` only** after the user objected to a full second checkout
   duplicating `get/`/`documentation/`/`.claude/skills` — `git sparse-checkout set '/.claude/work/*'`.
3. `setup_docs_worktree.sh` written, then extended twice more: generates and opens a two-folder VS
   Code workspace automatically, and — after the user asked specifically for this — offers, with an
   explicit `[y/N]` confirmation and a one-line explanation, to hide the now-stale `.claude/work/`
   in the code folder's Explorer. Never applies silently; skips cleanly with no TTY.
4. **Hit and fixed a real VS Code bug**: a folder-local `files.exclude` in the docs worktree
   intermittently rendered that whole Explorer root as empty, surviving a reload. Root cause not
   fully pinned down; fixed by deleting the settings file, which was redundant anyway once sparse
   checkout did the same job at the filesystem level. Logged in `traps.md`.
5. `mdube_run_output`'s own stale `.claude/work/mdube/` — frozen since the branch was cut, still
   showing two since-archived tasks as parked — removed from the branch entirely
   (`333806d`) so the later `git merge main` (`2a6059b`) resolved cleanly (4 rename/delete
   conflicts, all auto-resolving to `main`'s content).
6. Found while re-reading `collab.md` at the end of all this: `## Open`/`## Settled` stopped being
   real section boundaries around item `#48` (everything since has landed after `## Settled`,
   regardless), items `#14`–`#39` are physically out of chronological order, and `### 48` is used
   for two different items. Filed as `collab.md` **#59**, not fixed — reorganizing is exactly the
   kind of edit that needs the announce-first rule, so it's flagged for the next joint meeting
   rather than done unilaterally.

**Git manifest.** `mdube_run_output` (this task's branch): 3 local commits ahead of origin, clean
otherwise. `mdube_docs_worktree` (PR #70): clean, matches `origin/mdube_docs_worktree`. `main` (via
the docs worktree): clean, matches `origin/main` — carries `collab.md` #58 (3 addenda + summary)
and #59, `traps.md`'s new entry, and `decisions.md`'s worktree-migration entry, all already pushed.

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
