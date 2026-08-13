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

- [ ] `seed`, `run_index` and the generating config TOML reach `RunResult` —
      `get/src/lib.rs`, `get/src/dispatch.rs`, `get/src/py_result.rs`. Run-level, so they live on the
      result rather than on every in-memory row; the CSV writer emits them per row, which is what
      §6.4 actually asks for. `run_index` is `0` until #20.
      **Verify by:** a Rust test reading `result.seed` and `result.run_index` after a run; the TOML
      round-trips through `Config::from_toml_str`.

- [ ] `save_logs` becomes a method on `RunResult` and the evolver-side `&self` stub is deleted —
      `get/src/py_result.rs`, `get/src/lib.rs:266`. Header + one row per logged iteration, columns
      in §6.4's order with `seed` and `run_index` last.
      **Verify by:** a test writing to a temp path and re-reading it; row count is
      `num_generations + 1` (generational) and `num_mating_events / population_size + 1`
      (steady-state).

- [ ] `save_results` becomes a method on `RunResult`; the evolver stub at `get/src/lib.rs:272` is
      deleted. Writes the best genome's `print()` string, its edge list and its fitness, and the
      config TOML alongside as the provenance record (§6.4, §8).
      **Verify by:** a test asserting both files exist and the config file parses back;
      `grep -rn "todo!" get/src` finds neither method.

- [ ] Document why the log's best row can beat the reported `best_fitness` — in the column
      documentation the CSV's consumers read, per #21's 2026-08-13 amendment. The final population
      is re-scored, so a stochastic objective's earlier lucky draw is not kept.
      **Verify by:** the explanation is present wherever the columns are described and does not cite
      the spec sheet or an issue number (`CLAUDE.md`, amended 2026-08-13).

- [ ] Exercise it from real Python — `maturin develop`, run a config, `pandas.read_csv` the log and
      plot it. Nothing on this stack has been run from Python yet.
      **Verify by:** the issue's own gate — the CSV loads in pandas and plots. Python is not on
      `PATH` here; see `traps.md`, `python3-is-absent-and-bare-python-is-the-store-stub`.

- [ ] File the documentation consequences in `documentation/mdube_edits.md` — do not edit the site.
      At least: `guide/output.html`, `reference/lib.html`, `reference/py-result.html` if it exists,
      and any `status.html` row this de-badges.
      **Verify by:** the queue file names each page and what is now false in it.

- [ ] Full gate before the PR: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`. `cargo test` needs Python on `PATH` on this machine — `traps.md`,
      `cargo-test-cannot-link-python-unless-extension-module-is-off`.
      **Verify by:** all three clean, test count reported.

- [ ] Commit each verified step separately, then push and open the PR — **on explicit instruction
      only**, per `CLAUDE.md`. Note the stacked base in the PR body.
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
