# Plan — GitHub #21: define and write the run output (convergence log + best individual)
_Started 2026-08-13 · last updated 2026-08-13_

## Objective

A run's output is complete and writable: the log carries `ci_95`, `seed` and `run_index` alongside
the four columns it already has, and `save_logs` / `save_results` are real methods on `RunResult`
that write a CSV pandas can load and a best-individual record with the generating config TOML
beside it. The evolver-side `&self` stubs are gone. Spec §6.4; GitHub #21, as amended 2026-08-13.

**Out of scope:** replicates and the list-of-results return (#20) — `run_index` ships as `0` for the
single run this API still does, so #20 only has to fill it in. `set_base_graph` (#28). The across-run
confidence band, which §6.4 assigns to the user aggregating replicate logs. Any edit to
`documentation/` — those go in `documentation/mdube_edits.md` per `collab.md` #53.

**Stacked on unmerged work.** `RunResult` was added on `mdube_result_object` (PR #65) and the
per-owner workflow on `mdube_per_owner_work_dirs` (PR #69); task 1 branched off the first and merged
the second. **Both merged 2026-08-13**, along with PR #66. `git merge main` into `mdube_run_output`
is **done** — clean, 4 rename/delete conflicts in `.claude/work/` auto-resolved to `main`'s content
(expected, see `decisions.md` 2026-08-13 — the worktree migration).

## Tasks

- [x] Branch `mdube_run_output` created off `mdube_result_object` with `mdube_per_owner_work_dirs`
      merged in — clean, no shared files. Verified: both are ancestors of `HEAD`
      (`git merge-base --is-ancestor`). Rationale in `decisions.md` 2026-08-13 02:41.

- [x] `ci_95` added to `GenerationStats` (`evolver/mod.rs`) and computed in `generation_stats()`
      (`evolver/common.rs:297`) — sample deviation (`n-1`), `n=1` gives `0.0`. Verified:
      `generation_stats_computes_best_mean_and_population_deviation` and
      `a_single_individual_has_zero_deviation` both extended; 235 tests, clippy, fmt all clean.

- [x] `ci_95` carried through `erase` (`dispatch.rs`) and `PyGenerationStats::from_erased`
      (`py_result.rs`) unconverted, same as `std_dev`. Verified:
      `the_erased_history_comes_out_in_the_objectives_own_units` extended to assert `ci_95 >= 0.0`
      under a maximizing objective; 235 tests pass.

- [x] `seed`, `run_index` (hard `0`) and `config_toml` reach `RunResult` — `lib.rs`, `dispatch.rs`,
      `py_result.rs`. Verified: `run_returns_a_complete_result_object` reads both fields and
      round-trips `config_toml` through `Config::from_toml_str`. Commit `d187d10`.

- [x] `save_logs` is a method on `RunResult`; evolver stub deleted. Verified:
      `save_logs_writes_one_row_per_logged_iteration_plus_a_header` checks both row-count formulas
      and every column. Commit `79003ac`.

- [x] `save_results` is a method on `RunResult`; evolver stub deleted, `{filename}.toml` alongside.
      Verified: `save_results_writes_the_best_individual_and_a_reparseable_config`; `grep -rn
      "todo!" get/src` empty. Commit `d30d31d`.

- [x] Documented why the log's best row can beat reported `best_fitness` — doc comment on
      `PyRunResult::best_fitness`; `guide/evolvers.html` already carried the full reasoning.
      Commit `b4e3bb7`.

- [x] Exercised from real Python — `.venv`, `maturin develop`, a real run, `save_logs` +
      `pandas.read_csv` + a matplotlib plot, `save_results` + TOML round-trip. All passed
      2026-08-13; plot sent to Michael.

- [x] Documentation consequences filed in `documentation/mdube_edits.md`, not the site — two
      entries naming `guide/output.html`, `reference/lib.html`, `status.html` and what's false in
      each. Commit `24c3cc0`. No `reference/py-result.html` exists.

- [x] Full gate: `cargo test` (237 pass), `cargo clippy --all-targets -- -D warnings`, `cargo fmt
      --check` — all clean, re-verified after every subsequent commit through `b4e3bb7`.

- [ ] Open the PR — commits are made and pushed (`5b1f066` .. `b4e3bb7`, `mdube_run_output` merged
      with `main` at `49dc100`); opening the PR itself is still **on explicit instruction only**,
      per `CLAUDE.md`. Note the stacked base (#65, #69, both merged) in the PR body.
      **Verify by:** `gh pr view <n> --json body` shows the body survived.

## Open questions

- **`run_index` as a hard `0`, or omitted until #20?** Planned as `0`: §6.4 wants the column so
  concatenated logs stay separable, and a column that appears later changes the CSV schema on users
  who already wrote a reader. Blocks: the `seed`/`run_index` task. Flag to James if he disagrees.
- **Where the provenance TOML lands** — beside `save_results`'s file with a derived name, or an
  explicit second argument. Planned as derived; blocks nothing until that task starts.

## Out of scope

- Replicates, `max_cores`, list return — GitHub #20, blocked on #27.
- `set_base_graph` — GitHub #28.
- `documentation/` edits — queued in `documentation/mdube_edits.md`, swept as their own task
  (`collab.md` #53).
- Comment-volume cleanup in the files this touches — GitHub #68, deliberately tier (8).
