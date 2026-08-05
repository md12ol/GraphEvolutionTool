# Plan — Issue #15: convert fitness direction only at the boundary, not inside the engine
_Started 2026-08-04 · last updated 2026-08-04_

## Objective

Stop the engine converting fitness direction internally. After this task, `express_and_score` is
the only place a direction is applied inside `get/src/`: `generation_stats` no longer takes a
`Direction`, `SteadyStateEvolver::outcome` no longer orients, and `EvolutionOutcome` carries the
`Direction` so a future boundary can convert once on the way out. The `std_dev` special case and
the test defending it disappear with the conversion that caused them.

GitHub **#15**, tier (1). Design already settled — spec §5.1 and §6.4, and `decisions.md`
2026-07-31 "The engine is oriented internally; convert only at the boundary". This task implements
that entry; it does not re-decide it.

**Out of scope** — the outward conversion itself. The Python boundary that would consume
`EvolutionOutcome.direction` does not exist yet: `get/src/lib.rs:40` is still a stub and removing
`best_fitness()` is **#27, Michael's**. This task ends with the direction *carried and documented*,
not consumed.

## Tasks

- [x] Branch `jsargant_direction_at_boundary` off `main` at `6c21d42`.

- [x] `generation_stats` drops `direction` and stops converting — `common.rs:255`.
      Verified: builds clean, `Direction` no longer imported at lib scope in that file.

- [x] `EvolutionOutcome` gains `direction` and `best_fitness_engine` — `mod.rs:96-118`.
      Verified: builds clean; `GenerationStats` unchanged apart from its doc comment.

- [x] `SteadyStateEvolver::outcome` stores the direction instead of applying it — `steady_state.rs:123-138`.
      Verified: `grep -n "orient" get/src/evolver/steady_state.rs` returns nothing outside tests.

- [x] Tests rewritten, and both guards confirmed to fail if the conversion returns.
      `generation_stats_stays_in_engine_orientation_under_maximize` (`common.rs:381`) replaces the
      round-trip test; new `the_outcome_stays_engine_oriented_and_carries_the_direction`
      (`steady_state.rs:608`) covers `outcome`, which every other steady-state test misses because
      its `NodeCount` harness is `Minimize` and orientation is then the identity. Sabotage check
      run 2026-08-04: reinstating both conversions failed exactly those two (plus 4 collaterals);
      reverted, 128 pass.

- [x] Full verify pass. `cargo test -p get` 128 pass; clippy `--all-targets` byte-identical to the
      `main` baseline (the same 2 dead-code warnings from the unbuilt #25, 4 lines total); no new
      rustdoc link warnings. Formatted with `rustfmt --edition 2024 --config skip_children=true` —
      see the trap below.

- [x] Commit the code locally — `320fe68` on `jsargant_direction_at_boundary`, 3 files.
      Authorized 2026-08-04; push and PR deliberately held back.

- [x] Push the rustfmt trap and both #15 decisions to `main` — `252347d`, pure appends (+58/−0).
      Sent ahead of the PR because #22 is Michael's current work and the trap is live for him.

- [x] Branch pushed and **PR #41** opened against `main`, authorized 2026-08-04 22:2x.
      Verified from the remote: `state: open, mergeable: true`, 1 commit, 3 files, +110/−41; body
      diffed byte-for-byte against the source file (only a trailing newline differs).

- ~~Michael reviews and merges #41.~~ **Struck 2026-08-05 — not a task, and not James's.** The PR
      body reads `Closes #15.`, so the merge closes the issue with no action owed here. Verified
      open with zero reviews on 2026-08-05; the task was closed anyway rather than blocking #24.
      See `decisions.md` 2026-08-05.

## Open questions

None. The one that was here — the replacement name for `EvolutionOutcome.best_fitness` — was
settled as `best_fitness_engine` by James on 2026-08-04. `GenerationStats.best_fitness` keeps its
name: it is a log column named in spec §6.4's table and does not itself cross the boundary.

## Recorded — nothing pending

All three landed on `main` in `252347d` on 2026-08-04: the rustfmt/`mod.rs` trap in `traps.md`, and
two `decisions.md` entries (the `best_fitness_engine` rename; the `Maximize` test harness). The
underlying *design* needed no new entry — it was already recorded 2026-07-31, "The engine is
oriented internally; convert only at the boundary". #15 reverses `collab.md` #11, which the issue
body already states.

## Out of scope

- **Converting outward.** Belongs to the Python boundary — #27 (Michael) removes `best_fitness()`
  and returns a result object; #21 (Michael) defines the log. This task only makes the direction
  available to them.
- **`ci_95`** — spec §6.4 lists it as a `GenerationStats` column and the struct lacks it. That is
  #21, Michael's. Don't add it here.
- **`collab.md` items 20 and 21** — both sit with Michael, neither blocks this. Item 20's stale
  `CLAUDE.md` line was decided on 2026-08-04 to be left to him; do not fix it unilaterally.
