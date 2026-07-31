# Plan — Implement the steady-state evolver
_Started 2026-07-31 · last updated 2026-07-31_

> **This task is COMPLETE and awaiting `/done`.** All six tasks are `[x]`; 97 tests pass on `main`.
>
> A later session on 2026-07-31 did work **outside this plan** — a joint design call with James
> producing `/official_spec_sheet.md`. That work has no plan of its own and should have had one.
> **Next session: run `/done steady-state-evolver`, then `/start` for the spec-driven
> implementation.** Do not append the spec work to this plan; it is a different task.
>
> The spec records decisions the code does not yet implement — see `decisions.md`, 2026-07-31,
> twelve entries stamped "Michael & James".

**Branch:** `mdube_steady_state_implementation`, cut from `main` at `8970797`
on 2026-07-31. All work for this task lands here, not on `main`.

## Objective

Make `SteadyStateEvolver` actually run: fill the `todo!()`s in
`get/src/evolver/steady_state.rs`, plus the three shared helpers in
`get/src/evolver/common.rs` that it cannot run without. Done means a seeded
`run()` evolves a population against a `Fitness` and returns a real
`EvolutionOutcome`, proven by tests rather than by compiling.

**Out of scope** — see the section at the bottom. In particular: the
generational evolver, `SirFitness`, config dispatch, and the pyclass all stay
`todo!()`. This task uses a test-only fitness.

## Decisions taken at /start (2026-07-31)

Agreed with the user before planning; `/save` writes these to `decisions.md`.

1. ~~**Two children per mating event, replacing the two worst** *in the
   population*.~~ **Superseded 2026-07-31** by decision 7 below — replacement is
   tournament-local, not global. The "two children" half still holds.
2. **Incremental scoring.** Each event expresses and scores only the new children,
   writing into the fitness array in place — which is what `mating_event`'s
   `fitnesses: &mut [f64]` parameter is for. Known cost: one FFI hop per event
   for a future Python-backed fitness. See task 6.
3. **Log one row per `population_size` events** — a "generation equivalent", so a
   steady-state log is comparable to a generational one and stays a sane size.
4. **Implement `common.rs` helpers here**, and tell James so he doesn't
   duplicate them. Note this turned out to mean *all three* — logging every
   `population_size` events still needs `generation_stats`.
5. **Ties for "worst": lowest index wins.** Any consistent order would do;
   pinning it keeps runs reproducible.
6. **Replacement is unconditional** — a child enters even if it is worse than the
   individual it displaces. The population best is still never discarded, which
   is the guarantee `steady_state.rs:24` relies on.
7. **(2026-07-31, supersedes 1) Tournament-local replacement.** One tournament of
   distinct individuals per mating event: the best two breed, the two children
   overwrite the worst two **of that same tournament**. This is Ashlock-style
   "single tournament selection", which matches the SDA lineage of this codebase.
   Self-elitist, diversity-preserving, and O(k log k) per event instead of an
   O(population) scan for the global worst — which matters at the configured
   100,000 mating events. Confirm the lineage guess with James.
8. **(2026-07-31, supersedes the earlier with-replacement call for this path)**
   `tournament_indices` samples **without** replacement, because "the worst two
   members" is undefined over a multiset. `select` keeps sampling **with**
   replacement and is untouched — each follows the convention standard for its
   purpose. The divergence costs ~2% in expected distinct entrants at a
   population of 100, which is noise against a stochastic run. Consequence:
   parents are always two different individuals, so self-mating is impossible by
   construction.

## Tasks

- [x] 1. `Selection::select` — tournament selection, entrants drawn with
      replacement, `total_cmp` so `NaN` never wins, ties to the lower index.
      8 tests pass; inverting the comparison fails 3 of them, so they bite.
      Defensive `NaN` handling → `hotfixes.md`; self-pairing → `collab.md` #6.

- [x] 1c. Fitness direction — `Direction` enum + `Fitness::direction()` defaulting
      to `Minimize`. `Direction::orient` converts both ways and asserts on
      `NaN` — the single gate into the engine. 7 tests, 75 in the suite.
      Trait change → `collab.md` #9. "Cost" vocabulary dropped
      2026-07-31: one word, `orient`, one concept.
      No hotfixes outstanding — enforcement replaced the contract-only approach
      the same day, both entries moved to `hotfixes.md` § Resolved.

- [x] 1b. `Selection::tournament_indices` — one tournament of distinct
      individuals, returned best-first, for tournament-local replacement
      (decision 7). 4 more tests, 12 in the module, 68 in the suite. The
      whole-population case is RNG-independent, so it pins the ordering exactly.

- [x] 2. `common::evaluate` — parallel expression via rayon (what James's
      `Context: Send + Sync` bound is for), batch scoring, then `orient` applied
      once so returned fitnesses are always lower-is-better. 4 tests, 79 in the
      suite. Contract change to James's doc → `collab.md` #10.

- [x] 3. `common::generation_stats` — best/mean converted back to the
      objective's units, `std_dev` deliberately not (invariant under negation),
      population deviation not sample. Gained a `direction` parameter →
      `collab.md` #11. 4 tests, 83 in the suite; the `Maximize` test
      asserts `std_dev` equals the `Minimize` case exactly, so negating it
      later fails. **`common.rs` now has no `todo!()`s.**

- [x] 4. `SteadyStateEvolver::mating_event` — one tournament, two best breed,
      two children overwrite the two worst of that same tournament. Both
      children scored in one `evaluate` batch, halving per-event FFI hops.
      `MIN_TOURNAMENT_SIZE = 4` and `population >= tournament_size` asserted at
      construction. 7 tests, 90 in the suite; pointing replacement at the
      tournament's best instead fails 4 of them. Config-layer validation is the
      proper home → `collab.md` #12.

- [x] 5. `SteadyStateEvolver::run` — split into `evolve` (the event loop and log
      cadence) and `outcome` (best selection, single final expression), so `run`
      is four named steps. `ChaCha8Rng`, because `StdRng`'s algorithm may change
      between `rand` releases and break the reproducibility `seed` exists for →
      `collab.md` #4. Logs the starting population as iteration 0, so
      `history.len() == num_mating_events / population_size + 1` → #5.
      14 tests, 97 in the suite. A no-op `evolve` fails 2 of them, including
      `a_run_actually_improves_the_population`, which was added because the
      other 12 all passed against an engine that never bred.

- [x] 6. Coordination items recorded for the James meeting —
      `.claude/work/collab.md`. Six open items, two already settled.
      Written 2026-07-31; keep appending as this task turns up more.

## Follow-up, after this task closes

- **Config-layer validation of `tournament_size`.** `config.rs` should reject
  `tournament_size < 4` for steady-state, and a population smaller than the
  tournament, so a bad file is reported rather than panicking at construction.
  Strategy-specific — generational has no such floor. James's file, so raised
  with him rather than changed here. Agreed 2026-07-31.

- **Worked doctest examples on the `Fitness` and `Genome` traits.** GET is meant
  to be used by people writing their own fitness functions and genomes, and
  neither trait has an example. Doctests are compiled and run by `cargo test`, so
  they cannot rot. `Genome` is the one that needs it most — an associated
  `Context: Send + Sync` plus an in-place two-parent `crossover` is an unusual
  shape people will get wrong. Agreed 2026-07-31; own task, not this one.

## Open questions

_None. Both /start questions were answered on 2026-07-31 — see "Decisions taken"
above, items 5 and 6._

## Out of scope

- **`GenerationalEvolver`** — James's. Untouched.
- **`SirFitness`** — stays `todo!()`; tasks here use a deterministic test-only
  fitness. Later task.
- **Config dispatch and the `GraphEvolver` pyclass** — the wiring that turns a
  `config.toml` into a sized population and a genome context. Later task; the
  evolver is testable without it.
- **`Selection` variants beyond tournament** — the enum exists so adding one is
  cheap; no second variant is needed now.
- **`log_interval` as a config field** — rejected at /start as too big a change
  to James's schema. Interval is `population_size`, in code.
