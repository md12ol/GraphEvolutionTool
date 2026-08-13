# Plan — GitHub #27: `run` returns a result object; `best_fitness()` is removed
_Started 2026-08-13 · last updated 2026-08-13_

## Objective

`GraphEvolver.run(seed)` returns one result object carrying the best fitness (objective units),
the best individual's edge list, its `Genome::print()` string and the convergence history. The
`best_fitness` field and the `best_fitness()` method are gone, so a run's state lives in the value
the caller holds and never on the evolver. Spec §8; GitHub #27.

**Out of scope:** replicates and the list-of-results return (#20), `ci_95` / `seed` / `run_index`
columns and CSV writing (#21), `set_base_graph` (#28). `save_logs` / `save_results` stay `todo!()`
— #21 owns re-homing them onto the result object now that the evolver caches nothing.

## Tasks

- [x] Branch `mdube_result_object` created off `main`.

- [x] `ErasedOutcome` widened with `best_genome_repr` and `history`; `erase()`
      (`get/src/dispatch.rs:411`) orients both fitness columns of every row and leaves `std_dev`.
      Verified by `the_erased_history_comes_out_in_the_objectives_own_units`.

- [x] `PyRunResult` / `PyGenerationStats` added in a new `get/src/py_result.rs`, registered as
      `RunResult` / `GenerationStats`. Own module rather than `lib.rs`, mirroring `py_config.rs`.

- [x] `run` returns `PyRunResult`; the `best_fitness` field and getter are gone.
      Verified: `grep -rn "best_fitness()" get/src` finds only a doc comment.

- [x] Two consecutive runs on one evolver do not leak state —
      `two_runs_on_one_evolver_do_not_leak_state`, which re-runs seed 4 after seed 5 and gets the
      first result back exactly.

- [x] De-badge the documentation this ships. Two `status.html` rows dropped, the result object and
      log documented across ten pages, every `best_fitness()` example updated.
      Verified: `documentation/README.md`'s checker prints `checked 39 pages against 38 nav
      entries` with no errors.

- [x] Second pass over the docs, after the first missed things: ~45 `src` line references that this
      task's own edits had shifted, `lib.html`'s struct signature still showing `best_fitness`, and
      `generational.html` + `steady-state.html` both still claiming the log never reaches Python.
      Also `examples/config_builder.py`, which claimed `run` was unimplemented pending #26.

- [x] Create `documentation/mdube_edits.md` — the per-owner pending-edits queue that replaces
      editing the site per task, on Michael's instruction 2026-08-13. Routing is by
      `git config user.email`, unrecognised → ask. Nothing pending; #27's docs were applied above.

- [x] `collab.md` #53 — put the queue, `jsargant_edits.md`, and the `CLAUDE.md` contradiction to
      James. Audited: `uniq -d` clean, 41 headings at column 0.

- [x] Full gate: 235 tests pass, clippy clean under `-D warnings`, `cargo fmt --check` clean.
      Note `cargo test` needs Python on `PATH` on this machine — `traps.md`,
      `cargo-test-cannot-link-python-unless-extension-module-is-off`.

- [x] Committed, pushed, PR opened. Three commits on `mdube_result_object`; PR #65 open, body
      verified via `--json`. `main` carries the queue files, `collab.md` #53/#54 and `decisions.md`.
      PR #66 opened for the separate `*.sh text eol=lf` fix.

- [x] Strip the spec-sheet and issue references from this branch's own comments, after the
      convention changed mid-task (`decisions.md` 2026-08-13 02:16). `py_result.rs` carries none;
      `dispatch.rs` and `lib.rs` are each one lower than `main`. 235 tests, clippy, fmt all clean.

- [x] File the two cleanup issues at tier (8): #67 for `documentation/`, #68 for `get/src`
      comments. Both verified with `gh issue view --json`.

- [ ] **Waiting on James** — nothing to do until he responds.
      PR #65 and PR #66 both need his merge; `collab.md` #53 (per-owner doc queue,
      `jsargant_edits.md`) and #54 (the sheet-linking amendment, pushed direct) both need a reply.
      **Verify by:** `gh pr list --state open` is empty and #53/#54 carry an appended answer.

- [x] Sharpened GitHub #68's evidence for `dispatch.rs` — the diluted 29% row is now flagged and a
      new subsection carries the non-test figures. Verified via `gh issue view 68 --json body`.

- [x] Updated GitHub #21 with the `save_logs`/`save_results` re-homing requirement and the
      final-vs-best-ever note. Verified via `gh issue view 21 --json body`.

## Open questions

- None blocking. The `save_logs`/`save_results` consequence is recorded above and belongs to #21.

## Out of scope

- Replicates, `max_cores`, list return — GitHub #20, blocked on this.
- `ci_95`, per-row `seed`/`run_index`, CSV writing — GitHub #21.
- The uncommitted `*.sh text eol=lf` change in `.gitattributes` — its own branch, not this one.
